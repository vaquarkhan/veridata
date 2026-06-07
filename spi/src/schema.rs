use veridata_core::model::Boundary;

use crate::{ConnectorError, ConnectorResult};

/// Snapshot of field names present in a connector's effective schema.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaSnapshot {
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaDrift {
    pub missing: Vec<String>,
    pub unexpected: Vec<String>,
}

pub fn check_schema_drift(expected: &SchemaSnapshot, actual: &SchemaSnapshot) -> Option<SchemaDrift> {
    let mut missing: Vec<String> = expected
        .fields
        .iter()
        .filter(|f| !actual.fields.contains(f))
        .cloned()
        .collect();
    let mut unexpected: Vec<String> = actual
        .fields
        .iter()
        .filter(|f| !expected.fields.contains(f))
        .cloned()
        .collect();
    missing.sort();
    unexpected.sort();
    if missing.is_empty() && unexpected.is_empty() {
        None
    } else {
        Some(SchemaDrift { missing, unexpected })
    }
}

pub fn require_no_drift(expected: &SchemaSnapshot, actual: &SchemaSnapshot) -> ConnectorResult<()> {
    if let Some(drift) = check_schema_drift(expected, actual) {
        return Err(ConnectorError::SchemaDrift(format!(
            "missing={:?} unexpected={:?}",
            drift.missing, drift.unexpected
        )));
    }
    Ok(())
}

pub fn boundary_value(boundary: &Boundary) -> ConnectorResult<&[u8]> {
    if boundary.value.is_empty() {
        return Err(ConnectorError::InvalidBoundary("empty boundary value".into()));
    }
    Ok(&boundary.value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac_a7_1_schema_drift_flagged() {
        let expected = SchemaSnapshot {
            fields: vec!["order_id".into(), "amount".into()],
        };
        let actual = SchemaSnapshot {
            fields: vec!["order_id".into()],
        };
        let drift = check_schema_drift(&expected, &actual).unwrap();
        assert_eq!(drift.missing, vec!["amount".to_string()]);
    }
}
