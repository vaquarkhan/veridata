use serde_json::Value;
use veridata_core::canon::Record;
use veridata_spi::{json_to_record, ConnectorError, ConnectorResult};

pub fn parse_message(payload: &[u8]) -> ConnectorResult<Record> {
    let v: Value = serde_json::from_slice(payload)
        .map_err(|e| ConnectorError::Kafka(format!("invalid json: {e}")))?;
    Ok(json_to_record(&v))
}
