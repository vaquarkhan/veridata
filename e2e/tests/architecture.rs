//! Connectors must depend on spi + core only (not proof).

#[test]
fn kafka_depends_on_spi_not_proof() {
    let manifest = include_str!("../../connectors/kafka/Cargo.toml");
    assert!(manifest.contains("veridata-spi"));
    assert!(!manifest.contains("veridata-proof"));
}

#[test]
fn iceberg_depends_on_spi_not_proof() {
    let manifest = include_str!("../../connectors/iceberg/Cargo.toml");
    assert!(manifest.contains("veridata-spi"));
    assert!(!manifest.contains("veridata-proof"));
}
