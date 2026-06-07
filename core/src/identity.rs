use crate::error::{CoreError, CoreResult};

/// Parsed identity rule per policy.identity_rule grammar (§4.5).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentityRule {
    Field(String),
    Composite(Vec<String>),
}

pub fn identity_fields(rule: &str) -> CoreResult<IdentityRule> {
    if let Some(inner) = rule.strip_prefix("composite:[") {
        if !inner.ends_with(']') {
            return Err(CoreError::InvalidIdentityRule(rule.to_string()));
        }
        let fields: Vec<String> = inner[..inner.len() - 1]
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        if fields.is_empty() || fields.iter().any(|f| f.is_empty()) {
            return Err(CoreError::InvalidIdentityRule(rule.to_string()));
        }
        Ok(IdentityRule::Composite(fields))
    } else if let Some(name) = rule.strip_prefix("field:") {
        if name.is_empty() {
            return Err(CoreError::InvalidIdentityRule(rule.to_string()));
        }
        Ok(IdentityRule::Field(name.to_string()))
    } else {
        Err(CoreError::InvalidIdentityRule(rule.to_string()))
    }
}

pub fn identity_field_names(rule: &IdentityRule) -> &[String] {
    match rule {
        IdentityRule::Field(n) => std::slice::from_ref(n),
        IdentityRule::Composite(v) => v.as_slice(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac_a1_1_single_key() {
        let rule = identity_fields("field:order_id").unwrap();
        assert_eq!(rule, IdentityRule::Field("order_id".into()));
    }

    #[test]
    fn ac_a1_2_composite_order_matters() {
        let a = identity_fields("composite:[order_id,line_id]").unwrap();
        let b = identity_fields("composite:[line_id,order_id]").unwrap();
        assert_ne!(a, b);
    }

    #[test]
    fn ac_a1_3_missing_field_fails_loudly() {
        use crate::canon::{canon_identity, CanonValue, Record};
        use crate::model::CanonSpec;
        use std::collections::BTreeMap;

        let mut rec: Record = BTreeMap::new();
        rec.insert("order_id".into(), CanonValue::String("1".into()));
        let rule = identity_fields("composite:[order_id,line_id]").unwrap();
        let err = canon_identity(&rec, &rule, &CanonSpec::default()).unwrap_err();
        assert!(matches!(err, CoreError::MissingIdentityField(_)));
    }

    #[test]
    fn ac_a1_4_null_per_policy() {
        use crate::canon::{canon_identity, CanonValue, Record};
        use crate::model::CanonSpec;
        use std::collections::BTreeMap;

        let mut rec: Record = BTreeMap::new();
        rec.insert("order_id".into(), CanonValue::Null);
        let rule = identity_fields("field:order_id").unwrap();
        let bytes = canon_identity(&rec, &rule, &CanonSpec::default()).unwrap();
        assert!(!bytes.is_empty());
    }
}
