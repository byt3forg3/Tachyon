//! Integration tests for the seeding functionality.

#[test]
fn test_seeding_influence() {
    let data = b"Seed Test Data for Avalanche Check";
    let h1 = tachyon::hash_seeded(data, 0x1234_5678_9ABC_DEF0);
    let h2 = tachyon::hash_seeded(data, 0x1234_5678_9ABC_DEF1);

    assert_ne!(
        h1, h2,
        "Different seeds must produce different hash outputs"
    );
}

#[test]
fn test_seed_zero_vs_default() {
    let data = b"Compatibility Check";
    let h_default = tachyon::hash(data);
    let h_seeded_zero = tachyon::hash_seeded(data, 0);

    assert_eq!(
        h_default, h_seeded_zero,
        "Default hash should be identical to seed 0"
    );
}

#[test]
fn test_streaming_seeding() -> Result<(), tachyon::CpuFeatureError> {
    use tachyon::Hasher;

    let data = b"Streaming Seed Test";
    let mut hasher1 = Hasher::new_full(0, 1)?;
    hasher1.update(data);
    let h1 = hasher1.finalize();

    let mut hasher2 = Hasher::new_full(0, 2)?;
    hasher2.update(data);
    let h2 = hasher2.finalize();

    assert_ne!(
        h1, h2,
        "Different seeds in streaming mode must produce different outputs"
    );
    Ok(())
}
