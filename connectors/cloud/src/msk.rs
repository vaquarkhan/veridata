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
use veridata_connector_kafka::parse::parse_message;
#[cfg(feature = "kafka")]
use veridata_core::model::{Boundary, BoundaryMode, Fingerprint, Policy, Position, PositionKind};
#[cfg(feature = "kafka")]
use veridata_core::recon::build_fingerprint;
#[cfg(feature = "kafka")]
use veridata_spi::{
    parse_kafka_boundary, read_boundary_json, SourceConnector, ConnectorError, ConnectorResult,
};

/// Amazon MSK source with IAM/OAuthBearer SASL (P4).
#[cfg(feature = "kafka")]
pub struct MskKafkaSource {
    bootstrap_servers: String,
    topic: String,
    region: String,
}

#[cfg(feature = "kafka")]
impl MskKafkaSource {
    pub fn new(
        bootstrap_servers: impl Into<String>,
        topic: impl Into<String>,
        region: impl Into<String>,
    ) -> Self {
        Self {
            bootstrap_servers: bootstrap_servers.into(),
            topic: topic.into(),
            region: region.into(),
        }
    }

    fn consumer(&self) -> ConnectorResult<BaseConsumer> {
        ClientConfig::new()
            .set("bootstrap.servers", &self.bootstrap_servers)
            .set("group.id", "veridata-msk")
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .set("security.protocol", "SASL_SSL")
            .set("sasl.mechanism", "OAUTHBEARER")
            .set("sasl.oauthbearer.method", "AWS")
            .set("sasl.oauthbearer.aws.region", &self.region)
            .create()
            .map_err(|e| ConnectorError::Kafka(format!("msk consumer: {e}")))
    }
}

#[cfg(feature = "kafka")]
impl SourceConnector for MskKafkaSource {
    fn source_ref(&self) -> &str {
        "msk"
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
                "msk source requires OFFSET_RANGE".into(),
            ));
        }
        let spec = parse_kafka_boundary(&read_boundary_json(boundary)?)?;
        let consumer = self.consumer()?;
        let mut tpl = TopicPartitionList::new();
        for p in &spec.partitions {
            tpl.add_partition_offset(&self.topic, p.id, rdkafka::Offset::Offset(p.start))
                .map_err(|e| ConnectorError::Kafka(e.to_string()))?;
        }
        consumer
            .assign(&tpl)
            .map_err(|e| ConnectorError::Kafka(e.to_string()))?;

        let mut out = Vec::new();
        let mut remaining: std::collections::HashMap<i32, i64> = spec
            .partitions
            .iter()
            .map(|p| (p.id, p.end - p.start + 1))
            .collect();

        while remaining.values().any(|&c| c > 0) {
            match consumer.poll(Timeout::After(Duration::from_secs(30))) {
                None => break,
                Some(Err(e)) => return Err(ConnectorError::Kafka(e.to_string())),
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
                    let record = parse_message(payload)?;
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

#[cfg(test)]
mod tests {
    #[test]
    fn msk_module_compiles() {
        assert!(true);
    }
}
