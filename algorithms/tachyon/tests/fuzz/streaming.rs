use bolero::check;
use tachyon::{hash, Hasher};

#[test]
fn fuzz_streaming_consistency() {
    check!().with_type::<Vec<u8>>().for_each(|data| {
        // ── 1. One-shot baseline ─────────────────────────────────────────────
        let expected = hash(data);

        // ── 2. Single update ─────────────────────────────────────────────────
        {
            let mut hasher = Hasher::new();
            hasher.update(data);
            let res = hasher.finalize();
            assert_eq!(res, expected, "Streaming single update mismatch");
        }

        // ── 3. Byte-by-byte updates (small inputs only) ──────────────────────
        if data.len() < 256 {
            let mut hasher = Hasher::new();
            for b in data {
                hasher.update(&[*b]);
            }
            let res = hasher.finalize();
            assert_eq!(res, expected, "Byte-by-byte streaming mismatch");
        }

        // ── 4. Arbitrary split points ────────────────────────────────────────
        if data.len() > 1 {
            for split_idx in [1, data.len() / 2, data.len() - 1] {
                let mut hasher = Hasher::new();
                let (first, second) = data.split_at(split_idx);
                hasher.update(first);
                hasher.update(second);
                let res = hasher.finalize();
                assert_eq!(res, expected, "Split at {split_idx} mismatch");
            }
        }
    });
}
