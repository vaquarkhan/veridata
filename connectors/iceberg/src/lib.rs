//! Iceberg sink connector — reads filesystem warehouse Parquet snapshots.

mod connector;
mod parquet_value;
mod warehouse;

pub use connector::IcebergSinkConnector;
pub use warehouse::{write_snapshot, WarehouseConfig};
