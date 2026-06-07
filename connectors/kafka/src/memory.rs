use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use veridata_core::model::{Boundary, Fingerprint, Policy, Position, PositionKind};
use veridata_core::recon::build_fingerprint;
use veridata_spi::{
    parse_kafka_boundary, read_boundary_json, SourceConnector, ConnectorError, ConnectorResult,
};

use crate::parse::parse_message;

/// In-memory Kafka source for tests and environments without librdkafka.
#[derive(Clone, Default)]
pub struct MemoryKafkaSource {
    pub topic: String,
    /// partition -> offset -> json payload
    pub messages: Arc<Mutex<BTreeMap<i32, BTreeMap<i64, Vec<u8>>>>>,
}

impl MemoryKafkaSource {
    pub fn new(topic: impl Into<String>) -> Self {
        Self {
            topic: topic.into(),
            messages: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }

    pub fn produce(&self, partition: i32, offset: i64, json: &[u8]) {
        self.messages
            .lock()
            .unwrap()
            .entry(partition)
            .or_default()
            .insert(offset, json.to_vec());
    }
}

impl SourceConnector for MemoryKafkaSource {
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
        let spec = parse_kafka_boundary(&read_boundary_json(boundary)?)?;
        let store = self.messages.lock().unwrap();
        let mut out = Vec::new();
        for part in &spec.partitions {
            let Some(offsets) = store.get(&part.id) else {
                continue;
            };
            for offset in part.start..=part.end {
                let Some(payload) = offsets.get(&offset) else {
                    continue;
                };
                let record = parse_message(payload)?;
                let pos = Position {
                    kind: PositionKind::KafkaOffset,
                    value: crate::connector::encode_kafka_pos(part.id, offset),
                };
                out.push(build_fingerprint(
                    &record,
                    content_fields,
                    salt,
                    pos,
                    policy,
                )?);
            }
        }
        Ok(out)
    }
}

impl MemoryKafkaSource {
    pub fn read_payloads_for_boundary(&self, boundary: &Boundary) -> ConnectorResult<Vec<Vec<u8>>> {
        let spec = parse_kafka_boundary(&read_boundary_json(boundary)?)?;
        let store = self.messages.lock().unwrap();
        let mut out = Vec::new();
        for part in &spec.partitions {
            if let Some(offsets) = store.get(&part.id) {
                for offset in part.start..=part.end {
                    if let Some(bytes) = offsets.get(&offset) {
                        out.push(bytes.clone());
                    }
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use veridata_core::model::BoundaryMode;
    use veridata_core::testutil::{content_fields, default_policy, TEST_SALT};

    #[test]
    fn ac_a4_1_offset_range_reproducible() {
        let src = MemoryKafkaSource::new("orders");
        for i in 0..2i64 {
            src.produce(
                0,
                i,
                format!(
                    r#"{{"order_id":"{}","line_id":"1","amount":"dec:{}","status":"shipped"}}"#,
                    1000 + i,
                    10.5 + i as f64
                )
                .as_bytes(),
            );
        }
        let boundary = Boundary {
            mode: BoundaryMode::OffsetRange,
            value: br#"{"partitions":[{"id":0,"start":0,"end":1}]}"#.to_vec(),
        };
        let policy = default_policy();
        let fields = content_fields();
        let a = src
            .fingerprint_boundary(&boundary, &policy, &TEST_SALT, &fields)
            .unwrap();
        let b = src
            .fingerprint_boundary(&boundary, &policy, &TEST_SALT, &fields)
            .unwrap();
        assert_eq!(a.len(), 2);
        assert_eq!(a, b);
    }
}
