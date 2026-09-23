use bolero::check;
use tachyon::{hash, Hasher};

#[test]
fn fuzz_parallel_consistency() {
    check!().with_type::<Vec<u8>>().for_each(|data| {
        // ── 1. Parallel execution (Rayon) ────────────────────────────────────

        // Logic: `tachyon::hash` automatically uses Rayon for large inputs.
        // Even if input is small, it exercises the dispatch logic.
        let parallel_hash = hash(data);

        // ── 2. Sequential reference ──────────────────────────────────────────

        // Logic: `Hasher` updates are strictly sequential.
        // This serves as the "Ground Truth" for the parallel split implementation.
        let mut hasher = Hasher::new();
        hasher.update(data);
        let sequential_hash = hasher.finalize();

        // ── 3. Verification ──────────────────────────────────────────────────

        assert_eq!(
            parallel_hash, sequential_hash,
            "Parallel hash mismatch (Rayon vs Sequential)"
        );
    });
}
