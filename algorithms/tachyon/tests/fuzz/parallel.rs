use bolero::check;
use tachyon::{hash, Hasher};

#[test]
fn fuzz_parallel_consistency() {
    check!().with_type::<Vec<u8>>().for_each(|data| {
        // =============================================================================
        // PARALLEL EXECUTION (RAYON)
        // =============================================================================

        // Logic: `tachyon::hash` automatically uses Rayon for large inputs.
        // Even if input is small, it exercises the dispatch logic.
        let parallel_hash = hash(data);

        // =============================================================================
        // SEQUENTIAL REFERENCE
        // =============================================================================

        // Logic: `Hasher` updates are strictly sequential.
        // This serves as the "Ground Truth" for the parallel split implementation.
        if let Ok(mut hasher) = Hasher::new() {
            hasher.update(data);
            let sequential_hash = hasher.finalize();

            // =============================================================================
            // VERIFICATION
            // =============================================================================

            assert_eq!(
                parallel_hash, sequential_hash,
                "Parallel hash mismatch (Rayon vs Sequential)"
            );
        }
    });
}
