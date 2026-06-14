//! Databricks Delta / Unity Catalog sink on S3 or ADLS.

use std::sync::Arc;

use object_store::ObjectStore;
use veridata_spi::{SinkConnector, ConnectorError, ConnectorResult};

use crate::object_iceberg::ObjectStoreIcebergSink;

/// Delta tables stored as Parquet + snapshot manifest (Unity Catalog volume or external location).
pub struct DatabricksDeltaSink {
    inner: ObjectStoreIcebergSink,
    pub catalog: String,
    pub schema: String,
    pub table: String,
}

impl DatabricksDeltaSink {
    pub fn new(
        store: Arc<dyn ObjectStore>,
        prefix: impl Into<String>,
        catalog: impl Into<String>,
        schema: impl Into<String>,
        table: impl Into<String>,
    ) -> Self {
        let table_name = table.into();
        Self {
            inner: ObjectStoreIcebergSink::new(store, prefix, &table_name),
            catalog: catalog.into(),
            schema: schema.into(),
            table: table_name,
        }
    }

    pub fn from_s3_location(location: &str, table: impl Into<String>) -> ConnectorResult<Self> {
        let sink = ObjectStoreIcebergSink::from_s3_uri(location, table)?;
        Ok(Self {
            inner: sink,
            catalog: "main".into(),
            schema: "default".into(),
            table: "delta".into(),
        })
    }
}

impl SinkConnector for DatabricksDeltaSink {
    fn sink_ref(&self) -> &str {
        "databricks_delta"
    }

    fn fingerprint_boundary(
        &self,
        boundary: &veridata_core::model::Boundary,
        policy: &veridata_core::model::Policy,
        salt: &[u8],
        content_fields: &[std::string::String],
        mode: veridata_spi::PushdownMode,
    ) -> ConnectorResult<veridata_spi::SinkFingerprintBatch> {
        self.inner
            .fingerprint_boundary(boundary, policy, salt, content_fields, mode)
    }

    fn schema_snapshot(&self) -> ConnectorResult<veridata_spi::SchemaSnapshot> {
        self.inner.schema_snapshot()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac_d1_2_uses_spi_only() {
        fn assert_sink<T: SinkConnector>() {}
        assert_sink::<DatabricksDeltaSink>();
    }
}
