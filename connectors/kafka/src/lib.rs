//! Kafka source connector — reads offset-range boundary and emits fingerprints.

mod connector;
mod memory;
mod parse;

#[cfg(feature = "rdkafka-backend")]
mod rdkafka_reader;

pub use connector::{encode_kafka_pos, KafkaSourceConnector};
pub use memory::MemoryKafkaSource;
pub use parse::parse_message;
