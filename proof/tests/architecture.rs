//! Architecture gate: core must not depend on connectors, network, or proof crates.

#[test]
fn core_has_no_io_or_connector_deps() {
    let manifest = include_str!("../../core/Cargo.toml");
    let forbidden = [
        "tokio",
        "reqwest",
        "hyper",
        "sqlx",
        "rdkafka",
        "veridata-proof",
        "connectors",
    ];
    for dep in forbidden {
        assert!(
            !manifest.contains(&format!("{dep} =")),
            "core must not depend on {dep}"
        );
    }
}

#[test]
fn proof_depends_on_core_not_connectors() {
    let manifest = include_str!("../Cargo.toml");
    assert!(manifest.contains("veridata-core"));
    assert!(!manifest.contains("connectors"));
}
