//! Synthetic fingerprint generation for tests (F-A2, fault matrix, determinism).

use std::collections::BTreeMap;

use crate::canon::{CanonValue, Record};
use crate::model::{Fingerprint, Policy};
use crate::recon::{build_fingerprint, kafka_pos};

pub const TEST_SALT: [u8; 32] = [0xAB; 32];

pub fn default_policy() -> Policy {
    Policy {
        identity_rule: "composite:[order_id,line_id]".into(),
        canon: crate::model::CanonSpec::default(),
        hash_algorithm: "sha256".into(),
        content_fields: content_fields(),
        exclude_fields: vec!["_meta".into()],
        tolerances: crate::model::Tolerances::default(),
        late_arrival_window: "900s".into(),
    }
}

pub fn content_fields() -> Vec<String> {
    vec![
        "order_id".into(),
        "line_id".into(),
        "amount".into(),
        "status".into(),
    ]
}

pub fn sample_records(n: usize) -> Vec<Record> {
    (0..n)
        .map(|i| {
            let mut rec: Record = BTreeMap::new();
            rec.insert("order_id".into(), CanonValue::String(format!("{}", 1000 + i)));
            rec.insert("line_id".into(), CanonValue::String("1".into()));
            rec.insert(
                "amount".into(),
                CanonValue::String(format!("dec:{}", 10.5 + i as f64)),
            );
            rec.insert("status".into(), CanonValue::String("shipped".into()));
            rec.insert("_meta".into(), CanonValue::String(format!("ignore-{i}")));
            rec
        })
        .collect()
}

pub fn fingerprints_from_records(
    records: &[Record],
    policy: &Policy,
    offset_start: u64,
) -> Vec<Fingerprint> {
    let fields = crate::policy_util::effective_content_fields(
        &policy.content_fields,
        &policy.exclude_fields,
    );
    records
        .iter()
        .enumerate()
        .map(|(i, rec)| {
            build_fingerprint(
                rec,
                &fields,
                &TEST_SALT,
                kafka_pos(offset_start + i as u64),
                policy,
            )
            .expect("fingerprint")
        })
        .collect()
}

pub fn inject_drop(source: &[Fingerprint], drop_index: usize) -> Vec<Fingerprint> {
    source
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != drop_index)
        .map(|(_, f)| f.clone())
        .collect()
}

pub fn inject_dup(sink: &[Fingerprint], dup_index: usize) -> Vec<Fingerprint> {
    let mut out = sink.to_vec();
    out.push(out[dup_index].clone());
    out
}

pub fn inject_mutation(
    _source: &[Fingerprint],
    records: &[Record],
    mut_index: usize,
    field: &str,
    new_value: CanonValue,
    policy: &Policy,
) -> Vec<Fingerprint> {
    let mut recs: Vec<Record> = records.to_vec();
    recs[mut_index].insert(field.to_string(), new_value);
    fingerprints_from_records(&recs, policy, 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hash::Sha256Hasher;
    use crate::recon::reconcile;
    use crate::model::Verdict;

    #[test]
    fn ac_a2_3_only_selected_fields_matter() {
        let policy = default_policy();
        let fields = vec!["order_id".into(), "status".into()];
        let mut r1: Record = BTreeMap::new();
        r1.insert("order_id".into(), CanonValue::String("1".into()));
        r1.insert("line_id".into(), CanonValue::String("99".into()));
        r1.insert("status".into(), CanonValue::String("ok".into()));
        let mut r2 = r1.clone();
        r2.insert("line_id".into(), CanonValue::String("different".into()));
        let f1 = build_fingerprint(&r1, &fields, &TEST_SALT, kafka_pos(0), &policy).unwrap();
        let f2 = build_fingerprint(&r2, &fields, &TEST_SALT, kafka_pos(0), &policy).unwrap();
        assert_eq!(f1.content_hash, f2.content_hash);
    }

    #[test]
    fn ac_a2_1_metadata_excluded_same_hash() {
        let policy = default_policy();
        let fields = content_fields();
        let mut r1: Record = BTreeMap::new();
        r1.insert("order_id".into(), CanonValue::String("1".into()));
        r1.insert("line_id".into(), CanonValue::String("1".into()));
        r1.insert("amount".into(), CanonValue::String("dec:1.0".into()));
        r1.insert("status".into(), CanonValue::String("ok".into()));
        let mut r2 = r1.clone();
        r2.insert("_meta".into(), CanonValue::String("changed".into()));
        let f1 = build_fingerprint(&r1, &fields, &TEST_SALT, kafka_pos(0), &policy).unwrap();
        let f2 = build_fingerprint(&r2, &fields, &TEST_SALT, kafka_pos(0), &policy).unwrap();
        assert_eq!(f1.content_hash, f2.content_hash);
    }

    #[test]
    fn ac_a2_2_business_field_change_different_hash() {
        let policy = default_policy();
        let fields = content_fields();
        let mut r1: Record = BTreeMap::new();
        r1.insert("order_id".into(), CanonValue::String("1".into()));
        r1.insert("line_id".into(), CanonValue::String("1".into()));
        r1.insert("amount".into(), CanonValue::String("dec:1.0".into()));
        r1.insert("status".into(), CanonValue::String("ok".into()));
        let mut r2 = r1.clone();
        r2.insert("amount".into(), CanonValue::String("dec:2.0".into()));
        let f1 = build_fingerprint(&r1, &fields, &TEST_SALT, kafka_pos(0), &policy).unwrap();
        let f2 = build_fingerprint(&r2, &fields, &TEST_SALT, kafka_pos(0), &policy).unwrap();
        assert_ne!(f1.content_hash, f2.content_hash);
    }

    #[test]
    fn ac_a5_1_no_raw_in_hashes() {
        let recs = sample_records(1);
        let fps = fingerprints_from_records(&recs, &default_policy(), 0);
        let hex_id = hex::encode(fps[0].id_hash);
        assert!(!hex_id.contains("1000"));
    }

    #[test]
    fn ac_a5_2_different_salts_uncorrelated() {
        let policy = default_policy();
        let fields = content_fields();
        let recs = sample_records(1);
        let f1 = build_fingerprint(&recs[0], &fields, &[1u8; 32], kafka_pos(0), &policy).unwrap();
        let f2 = build_fingerprint(&recs[0], &fields, &[2u8; 32], kafka_pos(0), &policy).unwrap();
        assert_ne!(f1.id_hash, f2.id_hash);
    }

    #[test]
    fn fault_matrix_drop_dup_mutation() {
        let policy = default_policy();
        let recs = sample_records(5);
        let src = fingerprints_from_records(&recs, &policy, 0);

        let drop_snk = inject_drop(&src, 4);
        let drop_out = reconcile(&src, &drop_snk, &policy, &Sha256Hasher).unwrap();
        assert_eq!(drop_out.result.verdict, Verdict::Fail);

        let dup_snk = inject_dup(&src, 2);
        let dup_out = reconcile(&src, &dup_snk, &policy, &Sha256Hasher).unwrap();
        assert_eq!(dup_out.result.verdict, Verdict::Fail);

        let mut_snk = inject_mutation(
            &src,
            &recs,
            2,
            "amount",
            CanonValue::String("dec:999.99".into()),
            &policy,
        );
        let mut_out = reconcile(&src, &mut_snk, &policy, &Sha256Hasher).unwrap();
        assert_eq!(mut_out.result.verdict, Verdict::Fail);

        let clean = reconcile(&src, &src, &policy, &Sha256Hasher).unwrap();
        assert_eq!(clean.result.verdict, Verdict::Pass);
    }
}
