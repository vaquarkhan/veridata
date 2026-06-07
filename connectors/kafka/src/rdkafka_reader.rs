use std::time::Duration;

use rdkafka::config::ClientConfig;
use rdkafka::consumer::{BaseConsumer, Consumer};
use rdkafka::message::Message;
use rdkafka::util::Timeout;
use rdkafka::TopicPartitionList;
use veridata_core::model::{Boundary, BoundaryMode, Fingerprint, Policy, Position, PositionKind};
use veridata_core::recon::build_fingerprint;
use veridata_spi::{
    parse_kafka_boundary, read_boundary_json, SourceConnector, ConnectorError, ConnectorResult,
};

use crate::parse::parse_message;

pub struct KafkaSourceConnector {
    pub bootstrap_servers: String,
    pub topic: String,
    pub group_id: String,
}

impl KafkaSourceConnector {
    pub fn new(bootstrap_servers: impl Into<String>, topic: impl Into<String>) -> Self {
        Self {
            bootstrap_servers: bootstrap_servers.into(),
            topic: topic.into(),
            group_id: format!("veridata-{}", std::time::SystemTime::now().elapsed().unwrap_or_default().as_nanos()),
        }
    }

    fn consumer(&self) -> ConnectorResult<BaseConsumer> {
        ClientConfig::new()
            .set("bootstrap.servers", &self.bootstrap_servers)
            .set("group.id", &self.group_id)
            .set("enable.auto.commit", "false")
            .set("auto.offset.reset", "earliest")
            .create()
            .map_err(|e| ConnectorError::Kafka(e.to_string()))
    }
}

impl SourceConnector for KafkaSourceConnector {
    fn source_ref(&self) -> &str {
        "kafka"
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
                "kafka source requires OFFSET_RANGE".into(),
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
            match consumer.poll(Timeout::After(Duration::from_secs(10))) {
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
                        value: super::connector::encode_kafka_pos(partition, offset),
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
