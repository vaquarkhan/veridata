//! Azure Event Hubs source (Kafka protocol surface).

#[cfg(feature = "kafka")]
use std::time::Duration;

#[cfg(feature = "kafka")]
use rdkafka::config::ClientConfig;
#[cfg(feature = "kafka")]
use rdkafka::consumer::{BaseConsumer, Consumer};
#[cfg(feature = "kafka")]
use rdkafka::message::Message;
#[cfg(feature = "kafka")]
use rdkafka::util::Timeout;
#[cfg(feature = "kafka")]
use rdkafka::TopicPartitionList;
#[cfg(feature = "kafka")]
use veridata_core::model::{Boundary, BoundaryMode, Fingerprint, Policy, Position, PositionKind};
#[cfg(feature = "kafka")]
use veridata_core::recon::build_fingerprint;
#[cfg(feature = "kafka")]
use veridata_spi::{
    parse_kafka_boundary, read_boundary_json, SourceConnector, ConnectorError, ConnectorResult,
};

#[cfg(feature = "kafka")]
pub struct EventHubsSourceConnector {
    connection_string: String,
    topic: String,
}

#[cfg(feature = "kafka")]
impl EventHubsSourceConnector {
    pub fn new(connection_string: impl Into<String>, topic: impl Into<String>) -> Self {
        Self {
            connection_string: connection_string.into(),
            topic: topic.into(),
        }
    }

    fn consumer(&self) -> ConnectorResult<BaseConsumer> {
        ClientConfig::new()
            .set("bootstrap.servers", &self.connection_string)
            .set("security.protocol", "SASL_SSL")
            .set("sasl.mechanism", "PLAIN")
            .set("group.id", "veridata-eventhubs")
            .set("enable.auto.commit", "false")
            .create()
            .map_err(|e| ConnectorError::Azure(format!("eventhubs consumer: {e}")))
    }
}

#[cfg(feature = "kafka")]
impl SourceConnector for EventHubsSourceConnector {
    fn source_ref(&self) -> &str {
        "eventhubs"
    }

    fn fingerprint_boundary(
        &self,
        boundary: &Boundary,
        policy: &Policy,
        salt: &[u8],
        content_fields: &[String],
    ) -> ConnectorResult<Vec<Fingerprint>> {
        if boundary.mode != BoundaryMode::OffsetRange {
            return Err(ConnectorError::InvalidBoundary(
                "eventhubs requires OFFSET_RANGE".into(),
            ));
        }
        let spec = parse_kafka_boundary(&read_boundary_json(boundary)?)?;
        let consumer = self.consumer()?;
        let mut tpl = TopicPartitionList::new();
        for p in &spec.partitions {
            tpl.add_partition_offset(&self.topic, p.id, rdkafka::Offset::Offset(p.start))
                .map_err(|e| ConnectorError::Azure(e.to_string()))?;
        }
        consumer
            .assign(&tpl)
            .map_err(|e| ConnectorError::Azure(e.to_string()))?;

        let mut out = Vec::new();
        let mut remaining: std::collections::HashMap<i32, i64> = spec
            .partitions
            .iter()
            .map(|p| (p.id, p.end - p.start + 1))
            .collect();

        while remaining.values().any(|&c| c > 0) {
            match consumer.poll(Timeout::After(Duration::from_secs(30))) {
                None => break,
                Some(Err(e)) => return Err(ConnectorError::Azure(e.to_string())),
                Some(Ok(msg)) => {
                    let partition = msg.partition();
                    let offset = msg.offset();
                    let Some(left) = remaining.get_mut(&partition) else {
                        continue;
                    };
                    if *left <= 0 {
                        continue;
                    }
                    let payload = msg.payload().unwrap_or_default();
                    let record = veridata_connector_kafka::parse_message(payload)?;
                    let pos = Position {
                        kind: PositionKind::KafkaOffset,
                        value: veridata_connector_kafka::encode_kafka_pos(partition, offset),
                    };
                    out.push(build_fingerprint(
                        &record,
                        content_fields,
                        salt,
                        pos,
                        policy,
                    )?);
                    *left -= 1;
                }
            }
        }
        out.sort_by(|a, b| a.pos.value.cmp(&b.pos.value));
        Ok(out)
    }
}
