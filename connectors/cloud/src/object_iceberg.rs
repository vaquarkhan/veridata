use std::sync::Arc;

use bytes::Bytes;
use futures::StreamExt;
use object_store::path::Path as ObjectPath;
use object_store::ObjectStore;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ProjectionMask;
use veridata_connector_iceberg::parquet_value::{record_from_batch, schema_fields_from_batch};
use veridata_connector_iceberg::warehouse::{load_snapshots, snapshots_in_range, SnapshotManifest};
use veridata_core::model::{Boundary, BoundaryMode, Fingerprint, Policy, Position, PositionKind};
use veridata_core::recon::build_fingerprint;
use veridata_spi::{
    parse_iceberg_boundary, read_boundary_json, PushdownMode, SchemaSnapshot, SinkConnector,
    SinkFingerprintBatch, ConnectorError, ConnectorResult,
};

/// Iceberg warehouse backed by S3/GCS/ADLS via `object_store`.
pub struct ObjectStoreIcebergSink {
    store: Arc<dyn ObjectStore>,
    prefix: String,
    table: String,
}

impl ObjectStoreIcebergSink {
    pub fn new(store: Arc<dyn ObjectStore>, prefix: impl Into<String>, table: impl Into<String>) -> Self {
        Self {
            store,
            prefix: prefix.into().trim_matches('/').to_string(),
            table: table.into(),
        }
    }

    pub fn from_s3_uri(uri: &str, table: impl Into<String>) -> ConnectorResult<Self> {
        let (bucket, prefix) = parse_s3_uri(uri)?;
        #[cfg(feature = "aws")]
        {
            let store = object_store::aws::AmazonS3Builder::from_env()
                .with_bucket_name(&bucket)
                .build()
                .map_err(|e| ConnectorError::ObjectStore(e.to_string()))?;
            Ok(Self::new(Arc::new(store), prefix, table))
        }
        #[cfg(not(feature = "aws"))]
        {
            let _ = (bucket, prefix, table);
            Err(ConnectorError::Other(
                "s3 iceberg requires veridata-connector-cloud/aws feature".into(),
            ))
        }
    }

    fn metadata_path(&self) -> ObjectPath {
        let p = if self.prefix.is_empty() {
            format!("{}/metadata/snapshots.json", self.table)
        } else {
            format!("{}/{}/metadata/snapshots.json", self.prefix, self.table)
        };
        ObjectPath::from(p)
    }

    fn data_path(&self, file: &str) -> ObjectPath {
        let p = if self.prefix.is_empty() {
            format!("{}/{file}", self.table)
        } else {
            format!("{}/{}/{file}", self.prefix, self.table)
        };
        ObjectPath::from(p)
    }

    fn load_snapshots_async(&self) -> ConnectorResult<Vec<SnapshotManifest>> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| ConnectorError::Io(e.to_string()))?;
        let path = self.metadata_path();
        rt.block_on(async {
            let data = self
                .store
                .get(&path)
                .await
                .map_err(|e| ConnectorError::ObjectStore(e.to_string()))?
                .bytes()
                .await
                .map_err(|e| ConnectorError::ObjectStore(e.to_string()))?;
            serde_json::from_slice(&data).map_err(|e| ConnectorError::Iceberg(e.to_string()))
        })
    }

    fn read_parquet_bytes(&self, path: &ObjectPath) -> ConnectorResult<Bytes> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| ConnectorError::Io(e.to_string()))?;
        rt.block_on(async {
            self.store
                .get(path)
                .await
                .map_err(|e| ConnectorError::ObjectStore(e.to_string()))?
                .bytes()
                .await
                .map_err(|e| ConnectorError::ObjectStore(e.to_string()))
        })
    }
}

impl SinkConnector for ObjectStoreIcebergSink {
    fn sink_ref(&self) -> &str {
        "iceberg_s3"
    }

    fn fingerprint_boundary(
        &self,
        boundary: &Boundary,
        policy: &Policy,
        salt: &[u8],
        content_fields: &[String],
        mode: PushdownMode,
    ) -> ConnectorResult<SinkFingerprintBatch> {
        if boundary.mode != BoundaryMode::BatchId {
            return Err(ConnectorError::InvalidBoundary(
                "iceberg sink uses BATCH_ID snapshot boundary".into(),
            ));
        }
        let spec = parse_iceberg_boundary(&read_boundary_json(boundary)?)?;
        let manifests = self.load_snapshots_async()?;
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
                let path = self.data_path(file);
                let bytes = self.read_parquet_bytes(&path)?;
                let builder = ParquetRecordBatchReaderBuilder::try_new(bytes)
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
                        let pos = Position {
                            kind: PositionKind::IcebergRow,
                            value: serde_json::to_vec(&serde_json::json!({
                                "snapshot_id": snap.snapshot_id,
                                "file_index": file_idx,
                                "row_index": row_idx,
                            }))
                            .unwrap_or_default(),
                        };
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
        Ok(SinkFingerprintBatch {
            fingerprints,
            pushdown_used: pushdown && projection_used,
        })
    }

    fn schema_snapshot(&self) -> ConnectorResult<SchemaSnapshot> {
        let manifests = self.load_snapshots_async()?;
        let Some(first) = manifests.first() else {
            return Ok(SchemaSnapshot { fields: vec![] });
        };
        let Some(file) = first.files.first() else {
            return Ok(SchemaSnapshot { fields: vec![] });
        };
        let bytes = self.read_parquet_bytes(&self.data_path(file))?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(bytes)
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

fn parse_s3_uri(uri: &str) -> ConnectorResult<(String, String)> {
    let rest = uri
        .strip_prefix("s3://")
        .ok_or_else(|| ConnectorError::InvalidBoundary("expected s3:// URI".into()))?;
    let mut parts = rest.splitn(2, '/');
    let bucket = parts
        .next()
        .ok_or_else(|| ConnectorError::InvalidBoundary("s3 uri missing bucket".into()))?
        .to_string();
    let prefix = parts.next().unwrap_or("").to_string();
    Ok((bucket, prefix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac_d1_2_uses_spi_only() {
        fn assert_sink<T: SinkConnector>() {}
        assert_sink::<ObjectStoreIcebergSink>();
    }
}
