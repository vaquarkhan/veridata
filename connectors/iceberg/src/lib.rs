//! Iceberg sink connector — reads filesystem warehouse Parquet snapshots.

pub mod parquet_value;
pub mod warehouse;
mod connector;

pub use connector::IcebergSinkConnector;
pub use warehouse::{write_snapshot, WarehouseConfig};
