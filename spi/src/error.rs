use thiserror::Error;
use veridata_core::CoreError;

#[derive(Debug, Error)]
pub enum ConnectorError {
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error("invalid boundary: {0}")]
    InvalidBoundary(String),
    #[error("io error: {0}")]
    Io(String),
    #[error("kafka error: {0}")]
    Kafka(String),
    #[error("iceberg error: {0}")]
    Iceberg(String),
    #[error("gcp error: {0}")]
    Gcp(String),
    #[error("azure error: {0}")]
    Azure(String),
    #[error("bigquery error: {0}")]
    BigQuery(String),
    #[error("databricks error: {0}")]
    Databricks(String),
    #[error("object store error: {0}")]
    ObjectStore(String),
    #[error("schema drift: {0}")]
    SchemaDrift(String),
    #[error("{0}")]
    Other(String),
}

pub type ConnectorResult<T> = Result<T, ConnectorError>;
