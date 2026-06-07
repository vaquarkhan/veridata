//! Ingest from memory Kafka source into Iceberg warehouse with optional fault injection.

use serde_json::Value;
use veridata_connector_iceberg::{write_snapshot, WarehouseConfig};
use veridata_connector_kafka::MemoryKafkaSource;
use veridata_core::model::Boundary;
use veridata_spi::{ConnectorError, ConnectorResult};

#[derive(Debug, Clone, Copy, Default)]
pub enum Fault {
    #[default]
    None,
    Drop(usize),
    Duplicate(usize),
    Mutate {
        index: usize,
        field: &'static str,
        value: &'static str,
    },
}

pub struct IngestResult {
    pub snapshot_id: i64,
    pub rows_written: usize,
}

pub fn ingest_memory_to_iceberg(
    source: &MemoryKafkaSource,
    kafka_boundary: &Boundary,
    warehouse: &WarehouseConfig,
    snapshot_id: i64,
    fault: Fault,
) -> ConnectorResult<IngestResult> {
    let payloads = source.read_payloads_for_boundary(kafka_boundary)?;
    let mut rows: Vec<Value> = payloads
        .iter()
        .map(|bytes| {
            serde_json::from_slice(bytes).map_err(|e| ConnectorError::Other(e.to_string()))
        })
        .collect::<Result<_, _>>()?;
    apply_fault(&mut rows, fault);
    write_snapshot(warehouse, snapshot_id, &rows)?;
    Ok(IngestResult {
        snapshot_id,
        rows_written: rows.len(),
    })
}

fn apply_fault(messages: &mut Vec<Value>, fault: Fault) {
    match fault {
        Fault::None => {}
        Fault::Drop(i) if i < messages.len() => {
            messages.remove(i);
        }
        Fault::Duplicate(i) if i < messages.len() => {
            messages.push(messages[i].clone());
        }
        Fault::Mutate { index, field, value } if index < messages.len() => {
            if let Some(obj) = messages[index].as_object_mut() {
                obj.insert(field.to_string(), Value::String(value.to_string()));
            }
        }
        _ => {}
    }
}

pub fn assert_no_raw_values(vrp_json: &str, samples: &[&str]) {
    for s in samples {
        assert!(
            !vrp_json.contains(s),
            "raw value {s} leaked into proof"
        );
    }
}
