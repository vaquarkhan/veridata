//! Minimal RFC 8785-style canonical JSON matching P0 Python reference.

use serde_json::Value;

use super::{VrpDocument, VrpError, VrpResult};

pub fn signing_payload(doc: &VrpDocument) -> VrpResult<Vec<u8>> {
    let json = serde_json::to_value(doc)?;
    let Value::Object(map) = json else {
        return Err(VrpError::Invalid("VRP not an object".into()));
    };
    let mut subset = map.clone();
    subset.remove("signature");
    subset.remove("created_at");
    subset.remove("proof_id");
    let canonical = serialize(&Value::Object(subset));
    Ok(canonical.into_bytes())
}

fn serialize(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(b) => if *b { "true" } else { "false" }.into(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => serde_json::to_string(s).unwrap(),
        Value::Array(arr) => {
            let parts: Vec<String> = arr.iter().map(serialize).collect();
            format!("[{}]", parts.join(","))
        }
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            let parts: Vec<String> = keys
                .into_iter()
                .map(|k| format!("{}:{}", serde_json::to_string(k).unwrap(), serialize(&map[k])))
                .collect();
            format!("{{{}}}", parts.join(","))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn keys_sorted() {
        let v = json!({"b":1,"a":2});
        assert_eq!(serialize(&v), r#"{"a":2,"b":1}"#);
    }
}
