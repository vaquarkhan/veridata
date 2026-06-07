use std::path::PathBuf;

use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ProjectionMask;
use veridata_core::model::{Boundary, BoundaryMode, Fingerprint, Policy, Position, PositionKind};
use veridata_core::recon::build_fingerprint;
use veridata_spi::{
    parse_iceberg_boundary, read_boundary_json, PushdownMode, SchemaSnapshot,
    SinkConnector, SinkFingerprintBatch, ConnectorError, ConnectorResult,
};

use crate::parquet_value::{record_from_batch, schema_fields_from_batch};
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

    fn fingerprint_rows(
        &self,
        boundary: &Boundary,
        policy: &Policy,
        salt: &[u8],
        content_fields: &[String],
        mode: PushdownMode,
    ) -> ConnectorResult<(Vec<Fingerprint>, bool)> {
        if boundary.mode != BoundaryMode::BatchId {
            return Err(ConnectorError::InvalidBoundary(
                "iceberg sink uses BATCH_ID snapshot boundary".into(),
            ));
        }
        let spec = parse_iceberg_boundary(&read_boundary_json(boundary)?)?;
        let manifests = load_snapshots(&self.config.metadata_path())?;
        let selected = snapshots_in_range(&manifests, spec.snapshot_from, spec.snapshot_to);

        let identity_rule = veridata_core::identity::identity_fields(&policy.identity_rule)?;
        let id_names: Vec<String> = match identity_rule {
            veridata_core::identity::IdentityRule::Field(n) => vec![n],
            veridata_core::identity::IdentityRule::Composite(v) => v,
        };

        let mut selected_cols: Vec<String> = content_fields.to_vec();
        for f in &id_names {
            if !selected_cols.contains(f) {
                selected_cols.push(f.clone());
            }
        }
        selected_cols.sort();
        selected_cols.dedup();

        let pushdown = mode == PushdownMode::Pushdown;
        let mut fingerprints = Vec::new();
        let mut projection_used = false;

        for snap in selected {
            for (file_idx, file) in snap.files.iter().enumerate() {
                let path = self.config.data_path(file);
                let reader = std::fs::File::open(&path).map_err(|e| ConnectorError::Io(e.to_string()))?;
                let builder = ParquetRecordBatchReaderBuilder::try_new(reader)
                    .map_err(|e| ConnectorError::Iceberg(e.to_string()))?;

                let builder = if pushdown {
                    let arrow_schema = builder.schema();
                    let parquet_schema = builder.parquet_schema();
                    let mut roots: Vec<usize> = selected_cols
                        .iter()
                        .filter_map(|name| {
                            arrow_schema
                                .column_with_name(name)
                                .map(|(i, _)| parquet_schema.get_column_root_idx(i))
                        })
                        .collect();
                    roots.sort_unstable();
                    roots.dedup();
                    if roots.len() == selected_cols.len() {
                        projection_used = true;
                        let mask = ProjectionMask::roots(parquet_schema, roots);
                        builder.with_projection(mask)
                    } else {
                        builder
                    }
                } else {
                    builder
                };

                let mut batch_reader = builder
                    .build()
                    .map_err(|e| ConnectorError::Iceberg(e.to_string()))?;

                let mut row_idx: u64 = 0;
                while let Some(batch) = batch_reader
                    .next()
                    .transpose()
                    .map_err(|e| ConnectorError::Iceberg(e.to_string()))?
                {
                    let cols: Vec<String> = if pushdown && projection_used {
                        selected_cols.clone()
                    } else {
                        schema_fields_from_batch(&batch)
                            .into_iter()
                            .filter(|c| selected_cols.contains(c))
                            .collect()
                    };
                    let n = batch.num_rows();
                    for i in 0..n {
                        let rec = record_from_batch(&batch, i, &cols)?;
                        let pos = iceberg_pos(snap.snapshot_id, file_idx as u32, row_idx);
                        fingerprints.push(build_fingerprint(
                            &rec,
                            content_fields,
                            salt,
                            pos,
                            policy,
                        )?);
                        row_idx += 1;
                    }
                }
            }
        }
        Ok((fingerprints, pushdown && projection_used))
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
        let (fingerprints, pushdown_used) =
            self.fingerprint_rows(boundary, policy, salt, content_fields, mode)?;
        Ok(SinkFingerprintBatch {
            fingerprints,
            pushdown_used,
        })
    }

    fn schema_snapshot(&self) -> ConnectorResult<SchemaSnapshot> {
        let manifests = load_snapshots(&self.config.metadata_path())?;
        let Some(first) = manifests.first() else {
            return Ok(SchemaSnapshot { fields: vec![] });
        };
        let Some(file) = first.files.first() else {
            return Ok(SchemaSnapshot { fields: vec![] });
        };
        let path = self.config.data_path(file);
        let reader = std::fs::File::open(&path).map_err(|e| ConnectorError::Io(e.to_string()))?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(reader)
            .map_err(|e| ConnectorError::Iceberg(e.to_string()))?;
        let fields: Vec<String> = builder
            .schema()
            .fields()
            .iter()
            .map(|f| f.name().clone())
            .collect();
        Ok(SchemaSnapshot { fields })
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::{Int64Array, RecordBatch, StringArray};
    use arrow::datatypes::{DataType, Field, Schema};
    use parquet::arrow::ArrowWriter;
    use parquet::file::properties::WriterProperties;
    use std::fs;
    use std::sync::Arc;
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

    fn write_typed_snapshot(dir: &tempfile::TempDir) -> WarehouseConfig {
        let config = WarehouseConfig {
            root: dir.path().to_path_buf(),
            table: "typed".into(),
        };
        let table_dir = config.root.join(&config.table);
        let data_dir = table_dir.join("data");
        let meta_dir = table_dir.join("metadata");
        fs::create_dir_all(&data_dir).unwrap();
        fs::create_dir_all(&meta_dir).unwrap();
        let file_name = "data/snap-1.parquet";
        let file_path = table_dir.join(file_name);
        let schema = Arc::new(Schema::new(vec![
            Field::new("order_id", DataType::Int64, false),
            Field::new("line_id", DataType::Int64, false),
            Field::new("amount", DataType::Float64, false),
            Field::new("status", DataType::Utf8, false),
        ]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Int64Array::from(vec![1000, 1001])),
                Arc::new(Int64Array::from(vec![1, 1])),
                Arc::new(arrow::array::Float64Array::from(vec![10.5, 11.5])),
                Arc::new(StringArray::from(vec!["shipped", "shipped"])),
            ],
        )
        .unwrap();
        let file = fs::File::create(&file_path).unwrap();
        let mut writer =
            ArrowWriter::try_new(file, schema, Some(WriterProperties::builder().build())).unwrap();
        writer.write(&batch).unwrap();
        writer.close().unwrap();
        let manifests = serde_json::json!([{"snapshot_id":1,"files":[file_name]}]);
        fs::write(
            config.metadata_path(),
            serde_json::to_string_pretty(&manifests).unwrap(),
        )
        .unwrap();
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
        let run = || {
            connector.fingerprint_boundary(&boundary, &policy, &TEST_SALT, &fields, PushdownMode::Pushdown)
        };
        let a = run().unwrap();
        let b = run().unwrap();
        assert_eq!(a.fingerprints.len(), 2);
        assert_eq!(a.fingerprints, b.fingerprints);
    }

    #[test]
    fn ac_d4_1_pushdown_uses_column_projection() {
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
    fn ac_a7_2_typed_parquet_columns_supported() {
        let dir = tempdir().unwrap();
        let config = write_typed_snapshot(&dir);
        let connector = IcebergSinkConnector { config };
        let snap = connector.schema_snapshot().unwrap();
        assert!(snap.fields.contains(&"order_id".to_string()));
        let boundary = Boundary {
            mode: BoundaryMode::BatchId,
            value: br#"{"snapshot_from":1,"snapshot_to":1}"#.to_vec(),
        };
        let policy = default_policy();
        let fields = content_fields();
        let batch = connector
            .fingerprint_boundary(&boundary, &policy, &TEST_SALT, &fields, PushdownMode::Pushdown)
            .unwrap();
        assert_eq!(batch.fingerprints.len(), 2);
    }

    #[test]
    fn ac_d1_2_uses_spi_only() {
        fn assert_sink<T: SinkConnector>() {}
        assert_sink::<IcebergSinkConnector>();
    }
}
