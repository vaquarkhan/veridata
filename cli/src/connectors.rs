use veridata_connector_iceberg::{IcebergSinkConnector, WarehouseConfig};
use veridata_connector_kafka::MemoryKafkaSource;
use veridata_core::model::{Boundary, BoundaryMode};
use veridata_spi::{PushdownMode, SinkConnector, SourceConnector};

use crate::config::ReconConfig;

pub enum DynSource {
    Memory(MemoryKafkaSource),
    #[cfg(feature = "cloud")]
    Msk(veridata_connector_cloud::msk::MskKafkaSource),
    #[cfg(feature = "cloud")]
    PubSub(veridata_connector_cloud::pubsub::PubSubSourceConnector),
    #[cfg(feature = "cloud")]
    EventHubs(veridata_connector_cloud::eventhubs::EventHubsSourceConnector),
}

pub enum DynSink {
    Iceberg(IcebergSinkConnector),
    #[cfg(feature = "cloud")]
    IcebergS3(veridata_connector_cloud::object_iceberg::ObjectStoreIcebergSink),
    #[cfg(feature = "cloud")]
    BigQuery(veridata_connector_cloud::bigquery::BigQuerySinkConnector),
    #[cfg(feature = "cloud")]
    Databricks(veridata_connector_cloud::databricks::DatabricksDeltaSink),
}

impl SourceConnector for DynSource {
    fn source_ref(&self) -> &str {
        match self {
            Self::Memory(s) => s.source_ref(),
            #[cfg(feature = "cloud")]
            Self::Msk(s) => s.source_ref(),
            #[cfg(feature = "cloud")]
            Self::PubSub(s) => s.source_ref(),
            #[cfg(feature = "cloud")]
            Self::EventHubs(s) => s.source_ref(),
        }
    }

    fn fingerprint_boundary(
        &self,
        boundary: &Boundary,
        policy: &veridata_core::model::Policy,
        salt: &[u8],
        content_fields: &[String],
    ) -> veridata_spi::ConnectorResult<Vec<veridata_core::model::Fingerprint>> {
        match self {
            Self::Memory(s) => s.fingerprint_boundary(boundary, policy, salt, content_fields),
            #[cfg(feature = "cloud")]
            Self::Msk(s) => s.fingerprint_boundary(boundary, policy, salt, content_fields),
            #[cfg(feature = "cloud")]
            Self::PubSub(s) => s.fingerprint_boundary(boundary, policy, salt, content_fields),
            #[cfg(feature = "cloud")]
            Self::EventHubs(s) => s.fingerprint_boundary(boundary, policy, salt, content_fields),
        }
    }

    fn schema_snapshot(&self) -> veridata_spi::ConnectorResult<veridata_spi::SchemaSnapshot> {
        match self {
            Self::Memory(s) => s.schema_snapshot(),
            #[cfg(feature = "cloud")]
            Self::Msk(s) => s.schema_snapshot(),
            #[cfg(feature = "cloud")]
            Self::PubSub(s) => s.schema_snapshot(),
            #[cfg(feature = "cloud")]
            Self::EventHubs(s) => s.schema_snapshot(),
        }
    }
}

impl SinkConnector for DynSink {
    fn sink_ref(&self) -> &str {
        match self {
            Self::Iceberg(s) => s.sink_ref(),
            #[cfg(feature = "cloud")]
            Self::IcebergS3(s) => s.sink_ref(),
            #[cfg(feature = "cloud")]
            Self::BigQuery(s) => s.sink_ref(),
            #[cfg(feature = "cloud")]
            Self::Databricks(s) => s.sink_ref(),
        }
    }

    fn fingerprint_boundary(
        &self,
        boundary: &Boundary,
        policy: &veridata_core::model::Policy,
        salt: &[u8],
        content_fields: &[String],
        mode: PushdownMode,
    ) -> veridata_spi::ConnectorResult<veridata_spi::SinkFingerprintBatch> {
        match self {
            Self::Iceberg(s) => s.fingerprint_boundary(boundary, policy, salt, content_fields, mode),
            #[cfg(feature = "cloud")]
            Self::IcebergS3(s) => s.fingerprint_boundary(boundary, policy, salt, content_fields, mode),
            #[cfg(feature = "cloud")]
            Self::BigQuery(s) => s.fingerprint_boundary(boundary, policy, salt, content_fields, mode),
            #[cfg(feature = "cloud")]
            Self::Databricks(s) => s.fingerprint_boundary(boundary, policy, salt, content_fields, mode),
        }
    }

    fn schema_snapshot(&self) -> veridata_spi::ConnectorResult<veridata_spi::SchemaSnapshot> {
        match self {
            Self::Iceberg(s) => s.schema_snapshot(),
            #[cfg(feature = "cloud")]
            Self::IcebergS3(s) => s.schema_snapshot(),
            #[cfg(feature = "cloud")]
            Self::BigQuery(s) => s.schema_snapshot(),
            #[cfg(feature = "cloud")]
            Self::Databricks(s) => s.schema_snapshot(),
        }
    }
}

pub fn build_source(config: &ReconConfig) -> anyhow::Result<DynSource> {
    match config.source.kind.as_str() {
        "memory_kafka" => Ok(DynSource::Memory(MemoryKafkaSource::new(&config.source.topic))),
        #[cfg(feature = "cloud")]
        "msk" | "kafka" => {
            let servers = config
                .source
                .bootstrap_servers
                .clone()
                .ok_or_else(|| anyhow::anyhow!("source.bootstrap_servers required for msk"))?;
            let region = config
                .source
                .region
                .clone()
                .unwrap_or_else(|| "us-east-1".into());
            Ok(DynSource::Msk(veridata_connector_cloud::msk::MskKafkaSource::new(
                servers,
                &config.source.topic,
                region,
            )))
        }
        #[cfg(feature = "cloud")]
        "pubsub" => {
            let project = config
                .source
                .project
                .clone()
                .ok_or_else(|| anyhow::anyhow!("source.project required for pubsub"))?;
            let sub = config
                .source
                .subscription
                .clone()
                .ok_or_else(|| anyhow::anyhow!("source.subscription required for pubsub"))?;
            Ok(DynSource::PubSub(
                veridata_connector_cloud::pubsub::PubSubSourceConnector::new(project, sub),
            ))
        }
        #[cfg(feature = "cloud")]
        "eventhubs" => {
            let conn = config
                .source
                .connection_string
                .clone()
                .ok_or_else(|| anyhow::anyhow!("source.connection_string required for eventhubs"))?;
            Ok(DynSource::EventHubs(
                veridata_connector_cloud::eventhubs::EventHubsSourceConnector::new(
                    conn,
                    &config.source.topic,
                ),
            ))
        }
        other => anyhow::bail!(
            "unknown source type '{other}' (enable --features cloud for msk/pubsub/eventhubs)"
        ),
    }
}

pub fn build_sink(config: &ReconConfig) -> anyhow::Result<DynSink> {
    match config.sink.kind.as_str() {
        "iceberg" => {
            let root = config
                .sink
                .warehouse
                .clone()
                .ok_or_else(|| anyhow::anyhow!("sink.warehouse required for iceberg"))?;
            Ok(DynSink::Iceberg(IcebergSinkConnector::new(
                root,
                &config.sink.table,
            )))
        }
        #[cfg(feature = "cloud")]
        "iceberg_s3" | "iceberg-s3" => {
            let uri = config
                .sink
                .warehouse_uri
                .clone()
                .ok_or_else(|| anyhow::anyhow!("sink.warehouse_uri required for iceberg_s3"))?;
            Ok(DynSink::IcebergS3(
                veridata_connector_cloud::object_iceberg::ObjectStoreIcebergSink::from_s3_uri(
                    &uri,
                    &config.sink.table,
                )?,
            ))
        }
        #[cfg(feature = "cloud")]
        "bigquery" => {
            let project = config
                .source
                .project
                .clone()
                .or_else(|| std::env::var("GCP_PROJECT").ok())
                .ok_or_else(|| anyhow::anyhow!("source.project or GCP_PROJECT required for bigquery"))?;
            let dataset = config
                .sink
                .dataset
                .clone()
                .ok_or_else(|| anyhow::anyhow!("sink.dataset required for bigquery"))?;
            Ok(DynSink::BigQuery(
                veridata_connector_cloud::bigquery::BigQuerySinkConnector::new(
                    project,
                    dataset,
                    &config.sink.table,
                ),
            ))
        }
        #[cfg(feature = "cloud")]
        "databricks" | "databricks_delta" => {
            let uri = config
                .sink
                .warehouse_uri
                .clone()
                .ok_or_else(|| anyhow::anyhow!("sink.warehouse_uri required for databricks"))?;
            Ok(DynSink::Databricks(
                veridata_connector_cloud::databricks::DatabricksDeltaSink::from_s3_location(
                    &uri,
                    &config.sink.table,
                )?,
            ))
        }
        other => anyhow::bail!(
            "unknown sink type '{other}' (enable --features cloud for iceberg_s3/bigquery/databricks)"
        ),
    }
}

pub fn kafka_boundary(config: &ReconConfig) -> anyhow::Result<Boundary> {
    Ok(Boundary {
        mode: BoundaryMode::OffsetRange,
        value: serde_json::to_vec(&serde_json::json!({
            "partitions": config.source.boundary.partitions.iter().map(|p| {
                serde_json::json!({"id": p.id, "start": p.start, "end": p.end})
            }).collect::<Vec<_>>()
        }))?,
    })
}

pub fn sink_boundary(config: &ReconConfig) -> anyhow::Result<Boundary> {
    if config.sink.kind == "bigquery" {
        return Ok(Boundary {
            mode: BoundaryMode::BatchId,
            value: serde_json::to_vec(&serde_json::json!({
                "sql_filter": config.sink.boundary.sql_filter,
            }))?,
        });
    }
    Ok(Boundary {
        mode: BoundaryMode::BatchId,
        value: serde_json::to_vec(&serde_json::json!({
            "snapshot_from": config.sink.boundary.snapshot_from,
            "snapshot_to": config.sink.boundary.snapshot_to,
        }))?,
    })
}
