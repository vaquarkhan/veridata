use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use veridata_spi::ConnectorError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotManifest {
    pub snapshot_id: i64,
    pub files: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WarehouseConfig {
    pub root: PathBuf,
    pub table: String,
}

impl WarehouseConfig {
    pub fn metadata_path(&self) -> PathBuf {
        self.root.join(&self.table).join("metadata").join("snapshots.json")
    }

    pub fn data_path(&self, file: &str) -> PathBuf {
        self.root.join(&self.table).join(file)
    }
}

pub fn load_snapshots(path: &Path) -> Result<Vec<SnapshotManifest>, ConnectorError> {
    let text = std::fs::read_to_string(path).map_err(|e| ConnectorError::Io(e.to_string()))?;
    serde_json::from_str(&text).map_err(|e| ConnectorError::Iceberg(e.to_string()))
}

pub fn snapshots_in_range(
    manifests: &[SnapshotManifest],
    from: i64,
    to: i64,
) -> Vec<&SnapshotManifest> {
    manifests
        .iter()
        .filter(|m| m.snapshot_id >= from && m.snapshot_id <= to)
        .collect()
}

/// Write a reference Iceberg-style snapshot (Parquet data + metadata manifest).
pub fn write_snapshot(
    config: &WarehouseConfig,
    snapshot_id: i64,
    rows: &[serde_json::Value],
) -> Result<(), ConnectorError> {
    use arrow::array::{RecordBatch, StringArray};
    use arrow::datatypes::{DataType, Field, Schema};
    use parquet::arrow::ArrowWriter;
    use parquet::file::properties::WriterProperties;
    use std::fs;
    use std::sync::Arc;

    let table_dir = config.root.join(&config.table);
    let data_dir = table_dir.join("data");
    let meta_dir = table_dir.join("metadata");
    fs::create_dir_all(&data_dir).map_err(|e| ConnectorError::Io(e.to_string()))?;
    fs::create_dir_all(&meta_dir).map_err(|e| ConnectorError::Io(e.to_string()))?;

    let file_name = format!("data/snap-{snapshot_id}.parquet");
    let file_path = table_dir.join(&file_name);

    let order_id: StringArray = rows
        .iter()
        .map(|r| r.get("order_id").and_then(|v| v.as_str()).map(String::from))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| ConnectorError::Iceberg("missing order_id".into()))?
        .into();
    let line_id: StringArray = rows
        .iter()
        .map(|r| r.get("line_id").and_then(|v| v.as_str()).map(String::from))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| ConnectorError::Iceberg("missing line_id".into()))?
        .into();
    let amount: StringArray = rows
        .iter()
        .map(|r| r.get("amount").and_then(|v| v.as_str()).map(String::from))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| ConnectorError::Iceberg("missing amount".into()))?
        .into();
    let status: StringArray = rows
        .iter()
        .map(|r| r.get("status").and_then(|v| v.as_str()).map(String::from))
        .collect::<Option<Vec<_>>>()
        .ok_or_else(|| ConnectorError::Iceberg("missing status".into()))?
        .into();

    let schema = Arc::new(Schema::new(vec![
        Field::new("order_id", DataType::Utf8, false),
        Field::new("line_id", DataType::Utf8, false),
        Field::new("amount", DataType::Utf8, false),
        Field::new("status", DataType::Utf8, false),
    ]));
    let batch = RecordBatch::try_new(
        schema.clone(),
        vec![
            Arc::new(order_id),
            Arc::new(line_id),
            Arc::new(amount),
            Arc::new(status),
        ],
    )
    .map_err(|e| ConnectorError::Iceberg(e.to_string()))?;

    let file = fs::File::create(&file_path).map_err(|e| ConnectorError::Io(e.to_string()))?;
    let mut writer = ArrowWriter::try_new(file, schema, Some(WriterProperties::builder().build()))
        .map_err(|e| ConnectorError::Iceberg(e.to_string()))?;
    writer
        .write(&batch)
        .map_err(|e| ConnectorError::Iceberg(e.to_string()))?;
    writer
        .close()
        .map_err(|e| ConnectorError::Iceberg(e.to_string()))?;

    let mut manifests = load_snapshots(&config.metadata_path()).unwrap_or_default();
    manifests.push(SnapshotManifest {
        snapshot_id,
        files: vec![file_name],
    });
    let json = serde_json::to_string_pretty(&manifests)
        .map_err(|e| ConnectorError::Iceberg(e.to_string()))?;
    fs::write(config.metadata_path(), json).map_err(|e| ConnectorError::Io(e.to_string()))?;
    Ok(())
}
