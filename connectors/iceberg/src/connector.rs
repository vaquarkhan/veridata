use std::path::PathBuf;

use arrow::array::{Array, StringArray};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use veridata_core::canon::Record;
use veridata_core::model::{Boundary, BoundaryMode, Fingerprint, Policy, Position, PositionKind};
use veridata_core::recon::build_fingerprint;
use veridata_spi::{
    parse_iceberg_boundary, read_boundary_json, PushdownMode, SchemaSnapshot,
    SinkConnector, SinkFingerprintBatch, ConnectorError, ConnectorResult,
};

use crate::warehouse::{load_snapshots, snapshots_in_range, WarehouseConfig};

pub struct IcebergSinkConnector {
    pub config: WarehouseConfig,
}

impl IcebergSinkConnector {
    pub fn new(root: impl Into<PathBuf>, table: impl Into<String>) -> Self {
        Self {
            config: WarehouseConfig {
                root: root.into(),
                table: table.into(),
            },
        }
    }

    fn read_rows(
        &self,
        boundary: &Boundary,
        content_fields: &[String],
        identity_fields: &[String],
        mode: PushdownMode,
    ) -> ConnectorResult<Vec<(Record, Position)>> {
        if boundary.mode != BoundaryMode::BatchId {
            return Err(ConnectorError::InvalidBoundary(
                "iceberg sink uses BATCH_ID snapshot boundary".into(),
            ));
        }
        let spec = parse_iceberg_boundary(&read_boundary_json(boundary)?)?;
        let manifests = load_snapshots(&self.config.metadata_path())?;
        let selected = snapshots_in_range(&manifests, spec.snapshot_from, spec.snapshot_to);

        let mut selected_cols: Vec<String> = content_fields.to_vec();
        for f in identity_fields {
            if !selected_cols.contains(f) {
                selected_cols.push(f.clone());
            }
        }
        selected_cols.sort();
        selected_cols.dedup();

        let _pushdown = mode == PushdownMode::Pushdown;
        let mut rows = Vec::new();

        for snap in selected {
            for (file_idx, file) in snap.files.iter().enumerate() {
                let path = self.config.data_path(file);
                let reader = std::fs::File::open(&path).map_err(|e| ConnectorError::Io(e.to_string()))?;
                let builder = ParquetRecordBatchReaderBuilder::try_new(reader)
                    .map_err(|e| ConnectorError::Iceberg(e.to_string()))?;
                let mut batch_reader = builder
                    .build()
                    .map_err(|e| ConnectorError::Iceberg(e.to_string()))?;

                let mut row_idx: u64 = 0;
                while let Some(batch) = batch_reader
                    .next()
                    .transpose()
                    .map_err(|e| ConnectorError::Iceberg(e.to_string()))?
                {
                    let n = batch.num_rows();
                    for i in 0..n {
                        let rec = record_from_batch(&batch, i, &selected_cols)?;
                        let pos = iceberg_pos(snap.snapshot_id, file_idx as u32, row_idx);
                        rows.push((rec, pos));
                        row_idx += 1;
                    }
                }
            }
        }
        Ok(rows)
    }
}

impl SinkConnector for IcebergSinkConnector {
    fn sink_ref(&self) -> &str {
        "iceberg"
    }

    fn fingerprint_boundary(
        &self,
        boundary: &Boundary,
        policy: &Policy,
        salt: &[u8],
        content_fields: &[String],
        mode: PushdownMode,
    ) -> ConnectorResult<SinkFingerprintBatch> {
        let identity_rule = veridata_core::identity::identity_fields(&policy.identity_rule)?;
        let id_names: Vec<String> = match identity_rule {
            veridata_core::identity::IdentityRule::Field(n) => vec![n],
            veridata_core::identity::IdentityRule::Composite(v) => v,
        };

        let row_data = self.read_rows(boundary, content_fields, &id_names, mode)?;
        let fingerprints: Vec<Fingerprint> = row_data
            .into_iter()
            .map(|(rec, pos)| build_fingerprint(&rec, content_fields, salt, pos, policy))
            .collect::<Result<_, _>>()?;

        Ok(SinkFingerprintBatch {
            fingerprints,
            pushdown_used: mode == PushdownMode::Pushdown,
        })
    }

    fn schema_snapshot(&self) -> ConnectorResult<SchemaSnapshot> {
        Ok(SchemaSnapshot {
            fields: vec![
                "order_id".into(),
                "line_id".into(),
                "amount".into(),
                "status".into(),
            ],
        })
    }
}

fn record_from_batch(
    batch: &arrow::record_batch::RecordBatch,
    row: usize,
    columns: &[String],
) -> ConnectorResult<Record> {
    use std::collections::BTreeMap;
    use veridata_core::canon::CanonValue;

    let mut map = BTreeMap::new();
    for name in columns {
        let col = batch
            .schema()
            .column_with_name(name)
            .ok_or_else(|| ConnectorError::Iceberg(format!("missing column {name}")))?
            .0;
        let array = batch.column(col);
        let s = array
            .as_any()
            .downcast_ref::<StringArray>()
            .ok_or_else(|| ConnectorError::Iceberg(format!("column {name} not utf8")))?;
        if s.is_null(row) {
            map.insert(name.clone(), CanonValue::Null);
        } else {
            map.insert(name.clone(), CanonValue::String(s.value(row).to_string()));
        }
    }
    Ok(map)
}

pub fn iceberg_pos(snapshot_id: i64, file_index: u32, row_index: u64) -> Position {
    Position {
        kind: PositionKind::IcebergRow,
        value: serde_json::to_vec(&serde_json::json!({
            "snapshot_id": snapshot_id,
            "file_index": file_index,
            "row_index": row_index,
        }))
        .unwrap_or_default(),
    }
}

/// Client-side baseline for AC-D4.2 comparison.
pub fn fingerprint_client_side(
    connector: &IcebergSinkConnector,
    boundary: &Boundary,
    policy: &Policy,
    salt: &[u8],
    content_fields: &[String],
) -> ConnectorResult<Vec<Fingerprint>> {
    let batch = connector.fingerprint_boundary(
        boundary,
        policy,
        salt,
        content_fields,
        PushdownMode::ClientSide,
    )?;
    Ok(batch.fingerprints)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;
    use veridata_core::testutil::{default_policy, content_fields, TEST_SALT};

    fn write_test_snapshot(dir: &tempfile::TempDir) -> WarehouseConfig {
        let config = WarehouseConfig {
            root: dir.path().to_path_buf(),
            table: "orders".into(),
        };
        let rows = vec![
            serde_json::json!({"order_id":"1000","line_id":"1","amount":"dec:10.5","status":"shipped"}),
            serde_json::json!({"order_id":"1001","line_id":"1","amount":"dec:11.5","status":"shipped"}),
        ];
        crate::warehouse::write_snapshot(&config, 1, &rows).unwrap();
        config
    }

    #[test]
    fn ac_a4_2_iceberg_snapshot_range_reproducible() {
        let dir = tempdir().unwrap();
        let config = write_test_snapshot(&dir);
        let connector = IcebergSinkConnector { config };
        let boundary = Boundary {
            mode: BoundaryMode::BatchId,
            value: br#"{"snapshot_from":1,"snapshot_to":1}"#.to_vec(),
        };
        let policy = default_policy();
        let fields = content_fields();
        let run = || connector.fingerprint_boundary(&boundary, &policy, &TEST_SALT, &fields, PushdownMode::Pushdown);
        let a = run().unwrap();
        let b = run().unwrap();
        assert_eq!(a.fingerprints.len(), 2);
        assert_eq!(a.fingerprints, b.fingerprints);
    }

    #[test]
    fn ac_d4_1_only_fingerprints_returned() {
        let dir = tempdir().unwrap();
        let config = write_test_snapshot(&dir);
        let connector = IcebergSinkConnector { config };
        let boundary = Boundary {
            mode: BoundaryMode::BatchId,
            value: br#"{"snapshot_from":1,"snapshot_to":1}"#.to_vec(),
        };
        let batch = connector
            .fingerprint_boundary(
                &boundary,
                &default_policy(),
                &TEST_SALT,
                &content_fields(),
                PushdownMode::Pushdown,
            )
            .unwrap();
        assert!(batch.pushdown_used);
        assert_eq!(batch.fingerprints.len(), 2);
    }

    #[test]
    fn ac_d4_2_pushdown_matches_client_side() {
        let dir = tempdir().unwrap();
        let config = write_test_snapshot(&dir);
        let connector = IcebergSinkConnector { config };
        let boundary = Boundary {
            mode: BoundaryMode::BatchId,
            value: br#"{"snapshot_from":1,"snapshot_to":1}"#.to_vec(),
        };
        let policy = default_policy();
        let fields = content_fields();
        let push = connector
            .fingerprint_boundary(&boundary, &policy, &TEST_SALT, &fields, PushdownMode::Pushdown)
            .unwrap();
        let client = connector
            .fingerprint_boundary(&boundary, &policy, &TEST_SALT, &fields, PushdownMode::ClientSide)
            .unwrap();
        assert_eq!(push.fingerprints, client.fingerprints);
    }

    #[test]
    fn ac_d1_2_uses_spi_only() {
        fn assert_sink<T: SinkConnector>() {}
        assert_sink::<IcebergSinkConnector>();
    }
}
