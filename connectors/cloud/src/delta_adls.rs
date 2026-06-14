//! Delta Lake on Azure ADLS — delegates to object-store Parquet reader.

pub use crate::object_iceberg::ObjectStoreIcebergSink as DeltaAdlsSinkConnector;
