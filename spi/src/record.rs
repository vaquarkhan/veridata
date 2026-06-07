use serde_json::Value;
use veridata_core::canon::{CanonValue, Record};

pub fn json_to_record(v: &Value) -> Record {
    let mut rec = Record::new();
    let Some(obj) = v.as_object() else {
        return rec;
    };
    for (k, val) in obj {
        if let Some(cv) = json_value_to_canon(val) {
            rec.insert(k.clone(), cv);
        }
    }
    rec
}

fn json_value_to_canon(v: &Value) -> Option<CanonValue> {
    match v {
        Value::Null => Some(CanonValue::Null),
        Value::Bool(b) => Some(CanonValue::Bool(*b)),
        Value::Number(n) => Some(CanonValue::String(format!("dec:{n}"))),
        Value::String(s) => Some(CanonValue::String(s.clone())),
        Value::Array(items) => Some(CanonValue::Array(
            items.iter().filter_map(json_value_to_canon).collect(),
        )),
        Value::Object(_) => None, // nested maps not in v1 reference path
    }
}

pub fn record_field_names(rec: &Record) -> Vec<String> {
    rec.keys().cloned().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_flat_json() {
        let rec = json_to_record(&json!({"order_id":"1","amount":"dec:1.5"}));
        assert_eq!(rec.len(), 2);
    }
}
