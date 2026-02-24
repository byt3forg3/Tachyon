use bolero::check;
use tachyon::{hash, Hasher};

#[test]
fn fuzz_streaming_consistency() {
    check!().with_type::<Vec<u8>>().for_each(|data| {
        // =============================================================================
        // BASELINE (ONE-SHOT)
        // =============================================================================
        let expected = hash(data);

        // =============================================================================
        // STREAMING VARIATIONS
        // =============================================================================

        // 1. Single Update
        if let Ok(mut hasher) = Hasher::new() {
            hasher.update(data);
            let res = hasher.finalize();
            assert_eq!(res, expected, "Streaming single update mismatch");
        }

        // 2. Byte-by-Byte (Small Inputs Only)
        if data.len() < 256 {
            if let Ok(mut hasher) = Hasher::new() {
                for b in data {
                    hasher.update(&[*b]);
                }
                let res = hasher.finalize();
                assert_eq!(res, expected, "Byte-by-byte streaming mismatch");
            }
        }

        // 3. Arbitrary Split Points
        if data.len() > 1 {
            for split_idx in [1, data.len() / 2, data.len() - 1] {
                if let Ok(mut hasher) = Hasher::new() {
                    let (first, second) = data.split_at(split_idx);
                    hasher.update(first);
                    hasher.update(second);
                    let res = hasher.finalize();
                    assert_eq!(res, expected, "Split at {split_idx} mismatch");
                }
            }
        }
    });
}
