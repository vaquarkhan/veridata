use arrow::array::{
    Array, BooleanArray, Float64Array, Int64Array, StringArray, TimestampMicrosecondArray,
};
use arrow::datatypes::DataType;
use veridata_core::canon::{CanonValue, Record};
use veridata_spi::ConnectorError;

pub fn record_from_batch(
    batch: &arrow::record_batch::RecordBatch,
    row: usize,
    columns: &[String],
) -> Result<Record, ConnectorError> {
    use std::collections::BTreeMap;

    let mut map = BTreeMap::new();
    for name in columns {
        let col = batch
            .schema()
            .column_with_name(name)
            .ok_or_else(|| ConnectorError::Iceberg(format!("missing column {name}")))?
            .0;
        let array = batch.column(col);
        let value = arrow_value_at(array.as_ref(), row, name)?;
        map.insert(name.clone(), value);
    }
    Ok(map)
}

fn arrow_value_at(array: &dyn Array, row: usize, name: &str) -> Result<CanonValue, ConnectorError> {
    if array.is_null(row) {
        return Ok(CanonValue::Null);
    }
    match array.data_type() {
        DataType::Utf8 | DataType::LargeUtf8 => {
            let s = array
                .as_any()
                .downcast_ref::<StringArray>()
                .ok_or_else(|| ConnectorError::Iceberg(format!("column {name} not utf8")))?;
            Ok(CanonValue::String(s.value(row).to_string()))
        }
        DataType::Int64 => {
            let a = array
                .as_any()
                .downcast_ref::<Int64Array>()
                .ok_or_else(|| ConnectorError::Iceberg(format!("column {name} not int64")))?;
            Ok(CanonValue::String(a.value(row).to_string()))
        }
        DataType::Float64 => {
            let a = array
                .as_any()
                .downcast_ref::<Float64Array>()
                .ok_or_else(|| ConnectorError::Iceberg(format!("column {name} not float64")))?;
            Ok(CanonValue::String(format!("dec:{}", a.value(row))))
        }
        DataType::Boolean => {
            let a = array
                .as_any()
                .downcast_ref::<BooleanArray>()
                .ok_or_else(|| ConnectorError::Iceberg(format!("column {name} not bool")))?;
            Ok(CanonValue::Bool(a.value(row)))
        }
        DataType::Timestamp(_, _) => {
            let a = array
                .as_any()
                .downcast_ref::<TimestampMicrosecondArray>()
                .ok_or_else(|| ConnectorError::Iceberg(format!("column {name} not timestamp")))?;
            let micros = a.value(row);
            let secs = micros / 1_000_000;
            let nanos = ((micros % 1_000_000) * 1_000) as u32;
            let dt = chrono::DateTime::from_timestamp(secs, nanos)
                .ok_or_else(|| ConnectorError::Iceberg(format!("bad timestamp in {name}")))?;
            Ok(CanonValue::String(format!("ts:{}", dt.format("%Y-%m-%dT%H:%M:%S%.6fZ"))))
        }
        other => Err(ConnectorError::Iceberg(format!(
            "unsupported parquet type for {name}: {other:?}"
        ))),
    }
}

pub fn schema_fields_from_batch(batch: &arrow::record_batch::RecordBatch) -> Vec<String> {
    batch
        .schema()
        .fields()
        .iter()
        .map(|f| f.name().clone())
        .collect()
}
