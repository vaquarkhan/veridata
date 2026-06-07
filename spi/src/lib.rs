//! Connector SPI — reference integrations implement these traits without modifying core.

mod boundary;
mod error;
mod record;
mod schema;
mod sink;
mod source;

pub use boundary::{IcebergSnapshotBoundary, KafkaOffsetBoundary, KafkaPartitionRange, parse_iceberg_boundary, parse_kafka_boundary};
pub use error::{ConnectorError, ConnectorResult};
pub use record::{json_to_record, record_field_names};
pub use schema::{SchemaDrift, SchemaSnapshot, check_schema_drift};
pub use sink::{PushdownMode, SinkConnector, SinkFingerprintBatch};
pub use source::{read_boundary_json, SourceConnector};
