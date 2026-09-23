//! Hardware Dispatcher
//!
//! Selects the fastest available kernel (AVX-512, AES-NI, or portable) for the current CPU.

use crate::kernels;
use crate::types::{CpuFeatureError, KernelFn};

/// Work unit per parallel task: 256 KB (L2-cache friendly).
pub const CHUNK_SIZE: usize = 256 * 1024;

// =============================================================================
// DISPATCHER
// =============================================================================

/// Returns the fastest kernel for this CPU. Panics if unsupported.
#[must_use]
pub fn get_best_kernel() -> KernelFn {
    // ── 1. Runtime dispatch (std only) ───────────────────────────────────────
    #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
    {
        // Check for AVX-512 (Truck) vs AES-NI (Scooter)
        let has_avx512 = is_x86_feature_detected!("avx512f")
            && is_x86_feature_detected!("avx512bw")
            && is_x86_feature_detected!("vaes");

        let has_aesni = is_x86_feature_detected!("aes")
            && is_x86_feature_detected!("sse2")
            && is_x86_feature_detected!("pclmulqdq");

        if has_avx512 {
            if has_aesni {
                return safe_hybrid_wrapper;
            }
            return safe_avx512_wrapper;
        }

        if has_aesni {
            return safe_aesni_wrapper;
        }
    }

    // ── 2. Compile-time dispatch (no_std) ────────────────────────────────────
    #[cfg(not(feature = "std"))]
    {
        #[cfg(all(
            target_feature = "avx512f",
            target_feature = "avx512bw",
            target_feature = "vaes"
        ))]
        return safe_hybrid_wrapper;

        #[cfg(all(
            not(all(
                target_feature = "avx512f",
                target_feature = "avx512bw",
                target_feature = "vaes"
            )),
            target_feature = "aes",
            target_feature = "sse2",
            target_feature = "pclmulqdq"
        ))]
        return safe_aesni_wrapper;
    }

    // ── 3. Portable fallback ─────────────────────────────────────────────────
    kernels::portable::oneshot
}

/// Returns the AVX-512 kernel if the CPU supports it.
pub fn try_avx512_kernel() -> Result<KernelFn, CpuFeatureError> {
    #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
    {
        let has_avx512 = is_x86_feature_detected!("avx512f")
            && is_x86_feature_detected!("avx512bw")
            && is_x86_feature_detected!("vaes");
        if has_avx512 {
            let has_aesni = is_x86_feature_detected!("aes")
                && is_x86_feature_detected!("sse2")
                && is_x86_feature_detected!("pclmulqdq");
            return Ok(if has_aesni {
                safe_hybrid_wrapper
            } else {
                safe_avx512_wrapper
            });
        }
        Err(CpuFeatureError::new("avx512f + avx512bw + vaes"))
    }
    #[cfg(not(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64"))))]
    {
        #[cfg(all(
            target_feature = "avx512f",
            target_feature = "avx512bw",
            target_feature = "vaes"
        ))]
        return Ok(safe_hybrid_wrapper);
        #[allow(unreachable_code)]
        Err(CpuFeatureError::new("avx512f + avx512bw + vaes"))
    }
}

/// Returns the AES-NI kernel if the CPU supports it.
pub fn try_aesni_kernel() -> Result<KernelFn, CpuFeatureError> {
    #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
    {
        if is_x86_feature_detected!("aes")
            && is_x86_feature_detected!("sse2")
            && is_x86_feature_detected!("pclmulqdq")
        {
            return Ok(safe_aesni_wrapper);
        }
        Err(CpuFeatureError::new("aes + sse2 + pclmulqdq"))
    }
    #[cfg(not(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64"))))]
    {
        #[cfg(all(
            target_feature = "aes",
            target_feature = "sse2",
            target_feature = "pclmulqdq"
        ))]
        return Ok(safe_aesni_wrapper);
        #[allow(unreachable_code)]
        Err(CpuFeatureError::new("aes + sse2 + pclmulqdq"))
    }
}

/// Returns the portable software kernel (always available).
#[must_use]
pub fn portable_kernel() -> KernelFn {
    kernels::portable::oneshot
}

/// Returns the name of the active hardware backend.
#[must_use]
pub fn get_active_backend_name() -> &'static str {
    #[cfg(all(feature = "std", any(target_arch = "x86", target_arch = "x86_64")))]
    {
        let has_avx512 = is_x86_feature_detected!("avx512f")
            && is_x86_feature_detected!("avx512bw")
            && is_x86_feature_detected!("vaes");

        if has_avx512 {
            return "AVX-512 (Truck)\0";
        }

        let has_aesni = is_x86_feature_detected!("aes")
            && is_x86_feature_detected!("sse2")
            && is_x86_feature_detected!("pclmulqdq");

        if has_aesni {
            return "AES-NI (Scooter)\0";
        }
    }
    "Portable\0"
}

// =============================================================================
// WRAPPERS
// =============================================================================

/// Hybrid wrapper: AES-NI short path (< 64 B) + AVX-512 (>= 64 B)
#[inline]
#[allow(unsafe_code)]
#[allow(unused_variables)]
#[allow(dead_code)]
fn safe_hybrid_wrapper(
    input: &[u8],
    domain: u64,
    seed: u64,
    key: Option<&[u8; crate::kernels::constants::HASH_SIZE]>,
) -> [u8; crate::kernels::constants::HASH_SIZE] {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    // SAFETY: Only reachable after CPUID validation (AVX-512F/BW/VAES + AES/SSE2/PCLMULQDQ).
    // Short path (< 64B) uses AES-NI, bulk path (≥ 64B) uses AVX-512. All parameters validated.
    unsafe {
        if input.len() < 64 {
            kernels::aesni::short::oneshot_short(input, domain, seed, key)
        } else {
            kernels::avx512::oneshot(input, domain, seed, key)
        }
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    unreachable!("CPUID escape");
}

/// AVX-512-only wrapper: AES-NI short path (< 64 B) + AVX-512 (>= 64 B)
#[inline]
#[allow(unsafe_code)]
#[allow(unused_variables)]
#[allow(dead_code)]
fn safe_avx512_wrapper(
    input: &[u8],
    domain: u64,
    seed: u64,
    key: Option<&[u8; crate::kernels::constants::HASH_SIZE]>,
) -> [u8; crate::kernels::constants::HASH_SIZE] {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    // SAFETY: Only reachable after CPUID validation (AVX-512F/BW/VAES, NOT vpclmulqdq - dispatcher omits this check).
    // Short path (< 64B) uses AES-NI (requires AES/SSE2/PCLMULQDQ), bulk path (≥ 64B) uses AVX-512. All parameters validated.
    unsafe {
        if input.len() < 64 {
            kernels::aesni::short::oneshot_short(input, domain, seed, key)
        } else {
            kernels::avx512::oneshot(input, domain, seed, key)
        }
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    unreachable!("CPUID escape");
}

/// AES-NI-only wrapper: AES-NI short path (< 64 B) + AES-NI full path (>= 64 B)
#[inline]
#[allow(unsafe_code)]
#[allow(unused_variables)]
#[allow(dead_code)]
fn safe_aesni_wrapper(
    input: &[u8],
    domain: u64,
    seed: u64,
    key: Option<&[u8; crate::kernels::constants::HASH_SIZE]>,
) -> [u8; crate::kernels::constants::HASH_SIZE] {
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    // SAFETY: Only reachable after CPUID validation (AES/SSE2/PCLMULQDQ).
    // Both short (< 64B) and bulk (≥ 64B) paths use AES-NI. All parameters validated.
    unsafe {
        if input.len() < 64 {
            kernels::aesni::short::oneshot_short(input, domain, seed, key)
        } else {
            kernels::aesni::oneshot(input, domain, seed, key)
        }
    }
    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
    unreachable!("CPUID escape");
}
