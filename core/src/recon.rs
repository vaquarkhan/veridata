use std::collections::BTreeMap;

use crate::canon::{canon_content, canon_identity, Record};
use crate::error::CoreResult;
use crate::hash::{fingerprint_hashes, hasher_for, Hasher};
use crate::identity::identity_fields;
use crate::model::{Fingerprint, Policy, Position, PositionKind};

pub fn build_fingerprint(
    record: &Record,
    content_fields: &[String],
    salt: &[u8],
    pos: Position,
    policy: &Policy,
) -> CoreResult<Fingerprint> {
    let hasher = hasher_for(&policy.hash_algorithm)?;
    build_fingerprint_with_hasher(record, content_fields, salt, pos, policy, hasher.as_ref())
}

pub fn build_fingerprint_with_hasher(
    record: &Record,
    content_fields: &[String],
    salt: &[u8],
    pos: Position,
    policy: &Policy,
    hasher: &dyn Hasher,
) -> CoreResult<Fingerprint> {
    let id_rule = identity_fields(&policy.identity_rule)?;
    let id_canon = canon_identity(record, &id_rule, &policy.canon)?;
    let content_canon = canon_content(record, content_fields, &policy.canon)?;
    let (id_hash, content_hash, fp) = fingerprint_hashes(hasher, salt, &id_canon, &content_canon);
    Ok(Fingerprint {
        id_hash,
        content_hash,
        fp,
        pos,
    })
}

pub fn kafka_pos(offset: u64) -> Position {
    Position {
        kind: PositionKind::KafkaOffset,
        value: offset.to_be_bytes().to_vec(),
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReconcileOutput {
    pub result: crate::model::ReconResult,
    pub source_leaves: Vec<crate::model::Hash32>,
}

pub fn reconcile(
    source: &[Fingerprint],
    sink: &[Fingerprint],
    policy: &Policy,
    hasher: &dyn Hasher,
) -> CoreResult<ReconcileOutput> {
    let mut sg: BTreeMap<[u8; 32], Vec<&Fingerprint>> = BTreeMap::new();
    let mut kg: BTreeMap<[u8; 32], Vec<&Fingerprint>> = BTreeMap::new();
    for fp in source {
        sg.entry(fp.id_hash).or_default().push(fp);
    }
    for fp in sink {
        kg.entry(fp.id_hash).or_default().push(fp);
    }

    let source_leaves: Vec<_> = {
        let mut leaves: Vec<_> = source
            .iter()
            .map(|f| crate::hash::merkle_leaf(hasher, &f.fp))
            .collect();
        leaves.sort();
        leaves
    };

    let all_ids: BTreeMap<_, _> = sg
        .keys()
        .chain(kg.keys())
        .map(|k| (*k, ()))
        .collect();

    let mut matched_fps: Vec<[u8; 32]> = Vec::new();
    let mut missing = Vec::new();
    let mut duplicated = Vec::new();
    let mut mutated = Vec::new();

    for id_h in all_ids.keys() {
        let mut s_list: Vec<&Fingerprint> = sg.get(id_h).cloned().unwrap_or_default();
        let mut k_list: Vec<&Fingerprint> = kg.get(id_h).cloned().unwrap_or_default();
        let s_orig_len = s_list.len();
        let k_orig_len = k_list.len();

        let mut i = 0;
        while i < s_list.len() {
            let sj = s_list[i];
            if let Some(match_idx) = k_list
                .iter()
                .position(|kj| kj.content_hash == sj.content_hash)
            {
                matched_fps.push(sj.fp);
                s_list.remove(i);
                k_list.remove(match_idx);
            } else {
                i += 1;
            }
        }

        if !s_list.is_empty() && kg.contains_key(id_h) {
            let id_has_mutation = if let (Some(sj), Some(kj)) = (s_list.first(), k_list.first()) {
                mutated.push(crate::model::MutatedRecord {
                    id_hash: *id_h,
                    source_content_hash: sj.content_hash,
                    sink_content_hash: kj.content_hash,
                });
                true
            } else {
                false
            };
            let missing_src = if id_has_mutation {
                &s_list[1..]
            } else {
                &s_list[..]
            };
            for sj in missing_src {
                let leaf = crate::hash::merkle_leaf(hasher, &sj.fp);
                missing.push(crate::model::MissingRecord {
                    id_hash: sj.id_hash,
                    source_pos: sj.pos.clone(),
                    merkle_leaf: leaf,
                    inclusion_proof: crate::hash::merkle_proof(hasher, &source_leaves, &leaf)?,
                });
            }
        } else {
            for sj in &s_list {
                let leaf = crate::hash::merkle_leaf(hasher, &sj.fp);
                missing.push(crate::model::MissingRecord {
                    id_hash: sj.id_hash,
                    source_pos: sj.pos.clone(),
                    merkle_leaf: leaf,
                    inclusion_proof: crate::hash::merkle_proof(hasher, &source_leaves, &leaf)?,
                });
            }
        }

        if k_orig_len > s_orig_len {
            duplicated.push(crate::model::DuplicatedRecord {
                id_hash: *id_h,
                source_multiplicity: s_orig_len as u64,
                sink_multiplicity: k_orig_len as u64,
            });
        }
    }

    // dedupe mutated
    let mut seen = std::collections::BTreeSet::new();
    mutated.retain(|m| {
        seen.insert((
            m.id_hash,
            m.source_content_hash,
            m.sink_content_hash,
        ))
    });

    let mut matched_leaves: Vec<_> = matched_fps
        .iter()
        .map(|fp| crate::hash::merkle_leaf(hasher, fp))
        .collect();
    matched_leaves.sort();
    let matched_root = crate::hash::merkle_root(hasher, matched_leaves);

    let verdict = derive_verdict(
        missing.len() as u64,
        duplicated.len() as u64,
        mutated.len() as u64,
        None,
        &policy.tolerances,
    );

    Ok(ReconcileOutput {
        result: crate::model::ReconResult {
            matched: crate::model::Commitment {
                count: matched_fps.len() as u64,
                merkle_root: matched_root,
            },
            missing,
            duplicated,
            mutated,
            verdict,
            unverified_reason: None,
        },
        source_leaves,
    })
}

pub fn derive_verdict(
    missing_count: u64,
    duplicated_count: u64,
    mutated_count: u64,
    unverified_reason: Option<String>,
    tolerances: &crate::model::Tolerances,
) -> crate::model::Verdict {
    use crate::model::{DuplicatePolicy, Verdict};
    if unverified_reason.is_some() {
        return Verdict::Unverified;
    }
    if missing_count > tolerances.max_drops {
        return Verdict::Fail;
    }
    if mutated_count > tolerances.max_mutations {
        return Verdict::Fail;
    }
    if duplicated_count > 0 && tolerances.duplicates == DuplicatePolicy::Forbid {
        return Verdict::Fail;
    }
    Verdict::Pass
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::canon::{CanonValue, Record};
    use crate::hash::Sha256Hasher;
    use crate::model::{DuplicatePolicy, Tolerances};
    use std::collections::BTreeMap;

    const SALT: [u8; 32] = [0xAB; 32];

    fn default_policy() -> Policy {
        Policy {
            identity_rule: "composite:[order_id,line_id]".into(),
            canon: crate::model::CanonSpec::default(),
            hash_algorithm: "sha256".into(),
            tolerances: Tolerances::default(),
            late_arrival_window: "900s".into(),
        }
    }

    fn sample_fps(n: usize) -> Vec<Fingerprint> {
        let policy = default_policy();
        let fields = vec![
            "order_id".into(),
            "line_id".into(),
            "amount".into(),
            "status".into(),
        ];
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
                build_fingerprint(&rec, &fields, &SALT, kafka_pos(i as u64), &policy).unwrap()
            })
            .collect()
    }

    #[test]
    fn ac_b1_1_identical_sets_pass() {
        let src = sample_fps(3);
        let snk = src.clone();
        let out = reconcile(&src, &snk, &default_policy(), &Sha256Hasher).unwrap();
        assert_eq!(out.result.verdict, crate::model::Verdict::Pass);
        assert_eq!(out.result.matched.count, 3);
    }

    #[test]
    fn ac_b1_2_equal_multiplicity_not_dup() {
        let src = sample_fps(2);
        let snk = src.clone();
        let out = reconcile(&src, &snk, &default_policy(), &Sha256Hasher).unwrap();
        assert!(out.result.duplicated.is_empty());
    }

    #[test]
    fn ac_b2_1_missing_listed_with_id_and_pos() {
        let src = sample_fps(3);
        let snk = sample_fps(2);
        let out = reconcile(&src, &snk, &default_policy(), &Sha256Hasher).unwrap();
        assert_eq!(out.result.missing.len(), 1);
        assert_eq!(out.result.missing[0].id_hash, src[2].id_hash);
        assert_eq!(out.result.missing[0].source_pos, src[2].pos);
    }

    #[test]
    fn ac_b2_2_n_drops_count() {
        let src = sample_fps(5);
        let snk = sample_fps(2);
        let out = reconcile(&src, &snk, &default_policy(), &Sha256Hasher).unwrap();
        assert_eq!(out.result.missing.len(), 3);
    }

    #[test]
    fn ac_b3_1_dup_multiplicities() {
        let src = sample_fps(3);
        let mut snk = sample_fps(3);
        snk.push(snk[2].clone());
        let out = reconcile(&src, &snk, &default_policy(), &Sha256Hasher).unwrap();
        assert_eq!(out.result.duplicated.len(), 1);
        assert_eq!(out.result.duplicated[0].source_multiplicity, 1);
        assert_eq!(out.result.duplicated[0].sink_multiplicity, 2);
    }

    #[test]
    fn ac_b4_1_mutation_evidence() {
        let src = sample_fps(3);
        let mut snk = sample_fps(3);
        let policy = default_policy();
        let fields = vec![
            "order_id".into(),
            "line_id".into(),
            "amount".into(),
            "status".into(),
        ];
        let mut rec: Record = BTreeMap::new();
        rec.insert("order_id".into(), CanonValue::String("1002".into()));
        rec.insert("line_id".into(), CanonValue::String("1".into()));
        rec.insert("amount".into(), CanonValue::String("dec:999.99".into()));
        rec.insert("status".into(), CanonValue::String("shipped".into()));
        snk[2] = build_fingerprint(&rec, &fields, &SALT, kafka_pos(2), &policy).unwrap();
        let out = reconcile(&src, &snk, &policy, &Sha256Hasher).unwrap();
        assert_eq!(out.result.mutated.len(), 1);
        assert_eq!(out.result.mutated[0].id_hash, src[2].id_hash);
    }

    #[test]
    fn ac_b11_1_one_drop_fails() {
        let src = sample_fps(2);
        let snk = sample_fps(1);
        let out = reconcile(&src, &snk, &default_policy(), &Sha256Hasher).unwrap();
        assert_eq!(out.result.verdict, crate::model::Verdict::Fail);
    }

    #[test]
    fn ac_b11_2_benign_dup_allowed_passes() {
        let src = sample_fps(2);
        let mut snk = sample_fps(2);
        snk.push(snk[0].clone());
        let mut policy = default_policy();
        policy.tolerances.duplicates = DuplicatePolicy::AllowIfSinkIdempotent;
        let out = reconcile(&src, &snk, &policy, &Sha256Hasher).unwrap();
        assert_eq!(out.result.verdict, crate::model::Verdict::Pass);
        assert!(!out.result.duplicated.is_empty());
    }

    #[test]
    fn ac_b2_3_mutation_and_drop_independent_ids() {
        let policy = default_policy();
        let fields = vec![
            "order_id".into(),
            "line_id".into(),
            "amount".into(),
            "status".into(),
        ];
        let src = sample_fps(3);
        let mut snk = sample_fps(3);

        // id 1002: mutation (change amount on sink)
        let mut rec: Record = BTreeMap::new();
        rec.insert("order_id".into(), CanonValue::String("1002".into()));
        rec.insert("line_id".into(), CanonValue::String("1".into()));
        rec.insert("amount".into(), CanonValue::String("dec:999.99".into()));
        rec.insert("status".into(), CanonValue::String("shipped".into()));
        snk[2] = build_fingerprint(&rec, &fields, &SALT, kafka_pos(2), &policy).unwrap();

        // id 1000: pure drop (remove sink copy)
        snk.remove(0);

        let out = reconcile(&src, &snk, &policy, &Sha256Hasher).unwrap();
        assert_eq!(out.result.mutated.len(), 1);
        assert_eq!(out.result.missing.len(), 1);
        assert_eq!(out.result.missing[0].id_hash, src[0].id_hash);
    }

    #[test]
    fn ac_b11_3_unverified_never_pass() {
        let v = derive_verdict(
            0,
            0,
            0,
            Some("incomparable boundary".into()),
            &Tolerances::default(),
        );
        assert_eq!(v, crate::model::Verdict::Unverified);
    }
}
