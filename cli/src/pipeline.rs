use veridata_connector_iceberg::{IcebergSinkConnector, WarehouseConfig};
use veridata_connector_kafka::MemoryKafkaSource;
use veridata_core::model::{Boundary, BoundaryMode};
use veridata_core::{effective_content_fields, generate_proof_salt};
use veridata_e2e::{ingest_memory_to_iceberg, Fault};
use veridata_proof::{build_vrp, VrpDocument};
use veridata_spi::{check_schema_drift, PushdownMode, SchemaSnapshot, SinkConnector, SourceConnector};

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
    let mut policy = config.to_core_policy()?;
    let fields = effective_content_fields(
        &config.policy.content_fields,
        &config.policy.exclude_fields,
    );
    policy.content_fields = config.policy.content_fields.clone();
    policy.exclude_fields = config.policy.exclude_fields.clone();

    let salt = generate_proof_salt();

    let source = MemoryKafkaSource::new(&config.source.topic);
    if demo {
        let end = config
            .source
            .boundary
            .partitions
            .first()
            .map(|p| p.end)
            .unwrap_or(4);
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

    let sink = IcebergSinkConnector::new(warehouse.root.clone(), &config.sink.table);

    if let Some(expected) = &config.sink.expected_schema {
        let actual = sink.schema_snapshot()?;
        let expected_snap = SchemaSnapshot {
            fields: expected.clone(),
        };
        if let Some(drift) = check_schema_drift(&expected_snap, &actual) {
            anyhow::bail!(
                "schema drift: missing={:?} unexpected={:?}",
                drift.missing,
                drift.unexpected
            );
        }
    }

    let src_fps = source.fingerprint_boundary(&kafka_boundary, &policy, &salt, &fields)?;
    let snk = sink.fingerprint_boundary(
        &iceberg_boundary,
        &policy,
        &salt,
        &fields,
        PushdownMode::Pushdown,
    )?;

    let signer = load_signer(&config.crypto)?;
    let created_at = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();

    let doc = build_vrp(
        &src_fps,
        &snk.fingerprints,
        &policy,
        kafka_boundary,
        &format!("kafka:{}", config.source.topic),
        &format!("iceberg:{}.{}", warehouse.root.display(), config.sink.table),
        &salt,
        &config.producer,
        &created_at,
        None,
        signer.as_ref(),
    )?;

    Ok(ReconcileResult {
        verdict: doc.reconciliation.verdict.clone(),
        doc,
    })
}
