//! RFC 8785 (JCS) canonical JSON for signing payloads.

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

fn serialize_number(n: &serde_json::Number) -> String {
    if let Some(u) = n.as_u64() {
        return u.to_string();
    }
    if let Some(i) = n.as_i64() {
        return i.to_string();
    }
    if let Some(f) = n.as_f64() {
        if f.is_finite() {
            return serde_json::Number::from_f64(f)
                .map(|x| x.to_string())
                .unwrap_or_else(|| "null".into());
        }
    }
    "null".into()
}

fn serialize_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn serialize(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(b) => if *b { "true" } else { "false" }.into(),
        Value::Number(n) => serialize_number(n),
        Value::String(s) => serialize_string(s),
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
