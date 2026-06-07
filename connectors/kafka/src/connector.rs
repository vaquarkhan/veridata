//! Shared helpers and SPI compliance tests.

#[cfg(feature = "rdkafka-backend")]
pub use crate::rdkafka_reader::KafkaSourceConnector;

#[cfg(not(feature = "rdkafka-backend"))]
pub use crate::memory::MemoryKafkaSource as KafkaSourceConnector;

pub fn encode_kafka_pos(partition: i32, offset: i64) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({"partition": partition, "offset": offset}))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use veridata_spi::SourceConnector;

    #[test]
    fn ac_d1_2_uses_spi_only() {
        fn assert_source<T: SourceConnector>() {}
        assert_source::<KafkaSourceConnector>();
    }
}
