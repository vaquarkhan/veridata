use veridata_connector_iceberg::{IcebergSinkConnector, WarehouseConfig};
use veridata_connector_kafka::MemoryKafkaSource;
use veridata_core::model::{Boundary, BoundaryMode};
use veridata_core::testutil::TEST_SALT;
use veridata_e2e::{ingest_memory_to_iceberg, Fault};
use veridata_proof::{build_vrp, VrpDocument};
use veridata_spi::{PushdownMode, SinkConnector, SourceConnector};

use crate::config::ReconConfig;
use crate::keys::load_signer;

pub struct ReconcileResult {
    pub doc: VrpDocument,
    pub verdict: String,
}

pub fn seed_demo_messages(source: &MemoryKafkaSource, n: usize) {
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

pub fn run_reconcile(config: &ReconConfig, demo: bool) -> anyhow::Result<ReconcileResult> {
    let policy = config.to_core_policy()?;
    let fields = config.policy.content_fields.clone();

    let source = MemoryKafkaSource::new(&config.source.topic);
    if demo {
        let end = config.source.boundary.partitions.first().map(|p| p.end).unwrap_or(4);
        seed_demo_messages(&source, (end + 1) as usize);
    }

    let kafka_boundary = Boundary {
        mode: BoundaryMode::OffsetRange,
        value: serde_json::to_vec(&serde_json::json!({
            "partitions": config.source.boundary.partitions.iter().map(|p| {
                serde_json::json!({"id": p.id, "start": p.start, "end": p.end})
            }).collect::<Vec<_>>()
        }))?,
    };

    let warehouse = WarehouseConfig {
        root: config.sink.warehouse.clone(),
        table: config.sink.table.clone(),
    };
    std::fs::create_dir_all(&warehouse.root)?;

    let snapshot_id = config.sink.boundary.snapshot_to;
    ingest_memory_to_iceberg(&source, &kafka_boundary, &warehouse, snapshot_id, Fault::None)?;

    let iceberg_boundary = Boundary {
        mode: BoundaryMode::BatchId,
        value: serde_json::to_vec(&serde_json::json!({
            "snapshot_from": config.sink.boundary.snapshot_from,
            "snapshot_to": config.sink.boundary.snapshot_to,
        }))?,
    };

    let src_fps = source.fingerprint_boundary(&kafka_boundary, &policy, &TEST_SALT, &fields)?;
    let sink = IcebergSinkConnector::new(warehouse.root.clone(), &config.sink.table);
    let snk = sink.fingerprint_boundary(
        &iceberg_boundary,
        &policy,
        &TEST_SALT,
        &fields,
        PushdownMode::Pushdown,
    )?;

    let signer = load_signer(&config.crypto.private_key_file)?;
    let created_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let doc = build_vrp(
        &src_fps,
        &snk.fingerprints,
        &policy,
        kafka_boundary,
        &format!("kafka:{}", config.source.topic),
        &format!("iceberg:{}.{}", warehouse.root.display(), config.sink.table),
        &TEST_SALT,
        &config.producer,
        &created_at,
        None,
        &signer,
    )?;

    Ok(ReconcileResult {
        verdict: doc.reconciliation.verdict.clone(),
        doc,
    })
}
