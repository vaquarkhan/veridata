//! P2 E2E: Kafka (memory reference) -> ingest -> Iceberg sink -> VRP -> offline verify.

use veridata_connector_iceberg::{IcebergSinkConnector, WarehouseConfig};
use veridata_connector_kafka::MemoryKafkaSource;
use veridata_core::hash::Sha256Hasher;
use veridata_core::model::{Boundary, BoundaryMode, Verdict};
use veridata_core::recon::reconcile;
use veridata_core::testutil::{content_fields, default_policy, TEST_SALT};
use veridata_e2e::{assert_no_raw_values, ingest_memory_to_iceberg, Fault};
use veridata_proof::sign::KeyPair;
use veridata_proof::{build_vrp, Signer, Verifier, VerifyOutcome};
use veridata_spi::{PushdownMode, SinkConnector, SourceConnector};

fn sample_messages(source: &MemoryKafkaSource, n: usize) {
    for i in 0..n {
        source.produce(
            0,
            i as i64,
            format!(
                r#"{{"order_id":"{}","line_id":"1","amount":"dec:{}","status":"shipped"}}"#,
                1000 + i,
                10.5 + i as f64
            )
            .as_bytes(),
        );
    }
}

struct E2eOutcome {
    verdict: Verdict,
    vrp_json: String,
    verify: VerifyOutcome,
}

fn run_spi_e2e(fault: Fault, n: usize) -> E2eOutcome {
    let source = MemoryKafkaSource::new("orders");
    sample_messages(&source, n);

    let warehouse = WarehouseConfig {
        root: tempfile::tempdir().unwrap().keep(),
        table: "orders".into(),
    };

    let kafka_boundary = Boundary {
        mode: BoundaryMode::OffsetRange,
        value: format!(
            r#"{{"partitions":[{{"id":0,"start":0,"end":{}}}]}}"#,
            n as i64 - 1
        )
        .into_bytes(),
    };
    let iceberg_boundary = Boundary {
        mode: BoundaryMode::BatchId,
        value: br#"{"snapshot_from":1,"snapshot_to":1}"#.to_vec(),
    };

    ingest_memory_to_iceberg(&source, &kafka_boundary, &warehouse, 1, fault).expect("ingest");

    let policy = default_policy();
    let fields = content_fields();
    let src_fps = source
        .fingerprint_boundary(&kafka_boundary, &policy, &TEST_SALT, &fields)
        .expect("source");
    let sink = IcebergSinkConnector::new(warehouse.root.clone(), "orders");
    let snk = sink
        .fingerprint_boundary(
            &iceberg_boundary,
            &policy,
            &TEST_SALT,
            &fields,
            PushdownMode::Pushdown,
        )
        .expect("sink");

    let recon = reconcile(&src_fps, &snk.fingerprints, &policy, &Sha256Hasher).unwrap();
    let signer = KeyPair::test_key();
    let doc = build_vrp(
        &src_fps,
        &snk.fingerprints,
        &policy,
        kafka_boundary,
        "kafka:orders",
        "iceberg:warehouse.orders",
        &TEST_SALT,
        "veridata/0.1.0-e2e",
        "2026-06-07T00:00:00Z",
        None,
        &signer,
    )
    .unwrap();

    let vrp_json = serde_json::to_string(&doc).unwrap();
    let verify = Verifier::from_public_key_b64(&signer.public_key_b64())
        .unwrap()
        .verify(&doc)
        .unwrap();

    E2eOutcome {
        verdict: recon.result.verdict,
        vrp_json,
        verify,
    }
}

#[test]
fn e2e_clean_pass_offline_verified() {
    let out = run_spi_e2e(Fault::None, 5);
    assert_eq!(out.verdict, Verdict::Pass);
    assert_eq!(out.verify, VerifyOutcome::Pass);
    assert_no_raw_values(&out.vrp_json, &["1000", "shipped"]);
}

#[test]
fn e2e_drop_detected_and_proven() {
    let out = run_spi_e2e(Fault::Drop(4), 5);
    assert_eq!(out.verdict, Verdict::Fail);
    assert_eq!(out.verify, VerifyOutcome::Fail);
}

#[test]
fn e2e_duplicate_detected_and_proven() {
    let out = run_spi_e2e(Fault::Duplicate(2), 5);
    assert_eq!(out.verdict, Verdict::Fail);
    assert_eq!(out.verify, VerifyOutcome::Fail);
}

#[test]
fn e2e_mutation_detected_and_proven() {
    let out = run_spi_e2e(
        Fault::Mutate {
            index: 2,
            field: "amount",
            value: "dec:999.99",
        },
        5,
    );
    assert_eq!(out.verdict, Verdict::Fail);
    assert_eq!(out.verify, VerifyOutcome::Fail);
}

#[test]
fn e2e_boundary_reproducible() {
    let source = MemoryKafkaSource::new("orders");
    sample_messages(&source, 3);
    let boundary = Boundary {
        mode: BoundaryMode::OffsetRange,
        value: br#"{"partitions":[{"id":0,"start":0,"end":2}]}"#.to_vec(),
    };
    let policy = default_policy();
    let fields = content_fields();
    let a = source
        .fingerprint_boundary(&boundary, &policy, &TEST_SALT, &fields)
        .unwrap();
    let b = source
        .fingerprint_boundary(&boundary, &policy, &TEST_SALT, &fields)
        .unwrap();
    assert_eq!(a, b);
}

#[test]
fn ac_d1_1_connectors_use_spi() {
    fn assert_source<T: SourceConnector>() {}
    fn assert_sink<T: SinkConnector>() {}
    assert_source::<MemoryKafkaSource>();
    assert_sink::<IcebergSinkConnector>();
}
