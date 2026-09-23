use bolero::check;
use tachyon::{hash, verify};

#[test]
fn fuzz_verification_logic() {
    check!().with_type::<Vec<u8>>().for_each(|data| {
        // ── 1. Positive verification ─────────────────────────────────────────

        let h = hash(data);
        assert!(verify(data, &h), "verify() failed on correct data");

        // ── 2. Data corruption ───────────────────────────────────────────────
        if !data.is_empty() {
            let mut corrupted_data = data.clone();
            corrupted_data[0] ^= 0x01;
            assert!(
                !verify(&corrupted_data, &h),
                "verify() succeeded on corrupted data"
            );
        }

        // ── 3. Hash corruption ───────────────────────────────────────────────
        let mut bad_h = h;
        bad_h[0] ^= 0xFF; // Flip influential bits

        assert!(
            !verify(data, &bad_h),
            "verify() succeeded on corrupted hash"
        );
    });
}
