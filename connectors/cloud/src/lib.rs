//! Cloud platform connectors: MSK, object-store Iceberg, Pub/Sub, BigQuery, Event Hubs, Databricks.

#[cfg(feature = "kafka")]
pub mod msk;
pub mod object_iceberg;
#[cfg(feature = "gcp")]
pub mod bigquery;
#[cfg(feature = "gcp")]
pub mod pubsub;
#[cfg(feature = "azure")]
pub mod eventhubs;
#[cfg(feature = "azure")]
pub mod delta_adls;
pub mod databricks;
