use std::collections::BTreeMap;

use unicode_normalization::UnicodeNormalization;

use crate::error::{CoreError, CoreResult};
use crate::identity::{identity_field_names, IdentityRule};
use crate::model::CanonSpec;

pub type Record = BTreeMap<String, CanonValue>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonValue {
    Null,
    Bool(bool),
    String(String),
    /// Decimal encoded as `dec:<literal>` in tests or plain numeric string.
    Decimal(String),
    /// Timestamp encoded as `ts:<RFC3339>` in tests or plain RFC3339 string.
    Timestamp(String),
    Array(Vec<CanonValue>),
}

/// Identity canon preserves composite field order (AC-A1.2).
pub fn canon_identity(record: &Record, rule: &IdentityRule, spec: &CanonSpec) -> CoreResult<Vec<u8>> {
    if spec.version != 1 {
        return Err(CoreError::UnsupportedCanonVersion(spec.version));
    }
    let fields = identity_field_names(rule);
    canon_fields(record, fields, spec, false)
}

/// Content canon uses lexicographic field order (§4.2).
pub fn canon_content(record: &Record, fields: &[String], spec: &CanonSpec) -> CoreResult<Vec<u8>> {
    if spec.version != 1 {
        return Err(CoreError::UnsupportedCanonVersion(spec.version));
    }
    let mut sorted: Vec<String> = fields.to_vec();
    sorted.sort();
    canon_fields(record, &sorted, spec, true)
}

fn canon_fields(
    record: &Record,
    fields: &[String],
    spec: &CanonSpec,
    _lexicographic: bool,
) -> CoreResult<Vec<u8>> {
    let mut out = Vec::new();
    for name in fields {
        let value = record
            .get(name)
            .ok_or_else(|| CoreError::MissingIdentityField(name.clone()))?;
        out.extend_from_slice(b"\xf0");
        out.extend_from_slice(&u32_be(name.len() as u32));
        out.extend_from_slice(name.as_bytes());
        encode_value(&mut out, value, spec)?;
    }
    Ok(out)
}

fn u32_be(n: u32) -> [u8; 4] {
    n.to_be_bytes()
}

fn encode_value(out: &mut Vec<u8>, value: &CanonValue, spec: &CanonSpec) -> CoreResult<()> {
    match value {
        CanonValue::Null => out.push(0xA0),
        CanonValue::Bool(b) => {
            out.push(0xA1);
            out.push(if *b { 0x01 } else { 0x00 });
        }
        CanonValue::String(s) => {
            if let Some(ts) = s.strip_prefix("ts:") {
                let raw = ts.as_bytes();
                out.push(0xA4);
                out.extend_from_slice(&u32_be(raw.len() as u32));
                out.extend_from_slice(raw);
            } else if let Some(dec) = s.strip_prefix("dec:") {
                encode_decimal(out, dec, spec.decimal_scale)?;
            } else {
                encode_string(out, s)?;
            }
        }
        CanonValue::Decimal(d) => encode_decimal(out, d, spec.decimal_scale)?,
        CanonValue::Timestamp(t) => {
            let utc = normalize_timestamp(t, spec)?;
            let raw = utc.as_bytes();
            out.push(0xA4);
            out.extend_from_slice(&u32_be(raw.len() as u32));
            out.extend_from_slice(raw);
        }
        CanonValue::Array(items) => {
            out.push(0xA5);
            out.extend_from_slice(&u32_be(items.len() as u32));
            if spec.array_as_set {
                let mut encoded: Vec<Vec<u8>> = Vec::new();
                for item in items {
                    let mut buf = Vec::new();
                    encode_value(&mut buf, item, spec)?;
                    encoded.push(buf);
                }
                encoded.sort();
                for e in encoded {
                    out.extend_from_slice(&e);
                }
            } else {
                for item in items {
                    encode_value(out, item, spec)?;
                }
            }
        }
    }
    Ok(())
}

fn encode_string(out: &mut Vec<u8>, s: &str) -> CoreResult<()> {
    let nfc: String = s.nfc().collect();
    let raw = nfc.as_bytes();
    out.push(0xA2);
    out.extend_from_slice(&u32_be(raw.len() as u32));
    out.extend_from_slice(raw);
    Ok(())
}

fn encode_decimal(out: &mut Vec<u8>, s: &str, scale: u32) -> CoreResult<()> {
    let canonical = canonical_decimal(s, scale)?;
    let raw = canonical.as_bytes();
    out.push(0xA3);
    out.extend_from_slice(&u32_be(raw.len() as u32));
    out.extend_from_slice(raw);
    Ok(())
}

fn canonical_decimal(s: &str, scale: u32) -> CoreResult<String> {
    let s = s.trim();
    if s.is_empty() {
        return Err(CoreError::InvalidDecimal(s.to_string()));
    }
    let (whole, frac) = if let Some((w, f)) = s.split_once('.') {
        (w, f)
    } else {
        (s, "")
    };
    if whole.is_empty() || !whole.chars().all(|c| c.is_ascii_digit() || c == '-') {
        return Err(CoreError::InvalidDecimal(s.to_string()));
    }
    let scale = scale as usize;
    let mut frac_padded = frac.to_string();
    while frac_padded.len() < scale {
        frac_padded.push('0');
    }
    frac_padded.truncate(scale);
    if scale > 0 {
        Ok(format!("{whole}.{frac_padded}"))
    } else {
        Ok(whole.to_string())
    }
}

fn normalize_timestamp(ts: &str, spec: &CanonSpec) -> CoreResult<String> {
    // Accept RFC3339 input; emit UTC with microsecond precision.
    let ts = ts.strip_prefix("ts:").unwrap_or(ts);
    let parsed = chrono::DateTime::parse_from_rfc3339(ts)
        .map_err(|e| CoreError::Other(format!("invalid timestamp: {e}")))?;
    let utc = parsed.with_timezone(&chrono::Utc);
    match spec.timestamp_precision.as_str() {
        "micros" => Ok(utc.format("%Y-%m-%dT%H:%M:%S%.6fZ").to_string()),
        _ => Err(CoreError::Other(format!(
            "unsupported timestamp_precision: {}",
            spec.timestamp_precision
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::identity_fields;
    use std::collections::BTreeMap;

    fn spec() -> CanonSpec {
        CanonSpec::default()
    }

    fn rec(fields: &[(&str, CanonValue)]) -> Record {
        fields.iter().map(|(k, v)| (k.to_string(), v.clone())).collect()
    }

    #[test]
    fn ac_a3_1_field_order_independence_for_content() {
        let fields = vec![
            "amount".into(),
            "line_id".into(),
            "order_id".into(),
            "status".into(),
        ];
        let r1 = rec(&[
            ("order_id", CanonValue::String("1".into())),
            ("line_id", CanonValue::String("2".into())),
            ("amount", CanonValue::String("dec:1.5".into())),
            ("status", CanonValue::String("ok".into())),
        ]);
        let mut fields2 = fields.clone();
        fields2.reverse();
        let b1 = canon_content(&r1, &fields, &spec()).unwrap();
        let b2 = canon_content(&r1, &fields2, &spec()).unwrap();
        assert_eq!(b1, b2);
    }

    #[test]
    fn ac_a3_2_timezone_utc_equality() {
        let r1 = rec(&[("t", CanonValue::Timestamp("2026-01-01T12:00:00+05:00".into()))]);
        let r2 = rec(&[("t", CanonValue::Timestamp("2026-01-01T07:00:00Z".into()))]);
        let fields = vec!["t".into()];
        assert_eq!(
            canon_content(&r1, &fields, &spec()).unwrap(),
            canon_content(&r2, &fields, &spec()).unwrap()
        );
    }

    #[test]
    fn ac_a3_3_decimal_scale_equality() {
        let r1 = rec(&[("d", CanonValue::String("dec:1.50".into()))]);
        let r2 = rec(&[("d", CanonValue::String("dec:1.5".into()))]);
        let fields = vec!["d".into()];
        assert_eq!(
            canon_content(&r1, &fields, &spec()).unwrap(),
            canon_content(&r2, &fields, &spec()).unwrap()
        );
    }

    #[test]
    fn ac_a3_4_null_not_empty_string() {
        let r_null = rec(&[("x", CanonValue::Null)]);
        let r_empty = rec(&[("x", CanonValue::String("".into()))]);
        let fields = vec!["x".into()];
        assert_ne!(
            canon_content(&r_null, &fields, &spec()).unwrap(),
            canon_content(&r_empty, &fields, &spec()).unwrap()
        );
    }

    #[test]
    fn ac_a3_5_canon_version_recorded() {
        let mut s = spec();
        s.version = 99;
        let r = rec(&[("x", CanonValue::String("a".into()))]);
        let err = canon_content(&r, &["x".into()], &s).unwrap_err();
        assert!(matches!(err, CoreError::UnsupportedCanonVersion(99)));
    }

    #[test]
    fn ac_a1_2_composite_order_differs_in_canon() {
        let r = rec(&[
            ("order_id", CanonValue::String("1".into())),
            ("line_id", CanonValue::String("2".into())),
        ]);
        let rule_a = identity_fields("composite:[order_id,line_id]").unwrap();
        let rule_b = identity_fields("composite:[line_id,order_id]").unwrap();
        assert_ne!(
            canon_identity(&r, &rule_a, &spec()).unwrap(),
            canon_identity(&r, &rule_b, &spec()).unwrap()
        );
    }
}
