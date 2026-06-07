use serde::{Deserialize, Serialize};

use crate::ConnectorError;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KafkaPartitionRange {
    pub id: i32,
    pub start: i64,
    pub end: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KafkaOffsetBoundary {
    pub partitions: Vec<KafkaPartitionRange>,
}

pub fn parse_kafka_boundary(value: &[u8]) -> Result<KafkaOffsetBoundary, ConnectorError> {
    serde_json::from_slice(value).map_err(|e| ConnectorError::InvalidBoundary(e.to_string()))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IcebergSnapshotBoundary {
    pub snapshot_from: i64,
    pub snapshot_to: i64,
}

pub fn parse_iceberg_boundary(value: &[u8]) -> Result<IcebergSnapshotBoundary, ConnectorError> {
    serde_json::from_slice(value).map_err(|e| ConnectorError::InvalidBoundary(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_kafka_offset_boundary() {
        let b = parse_kafka_boundary(br#"{"partitions":[{"id":0,"start":0,"end":10}]}"#).unwrap();
        assert_eq!(b.partitions.len(), 1);
        assert_eq!(b.partitions[0].end, 10);
    }
}
