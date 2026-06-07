use proptest::prelude::*;
use veridata_core::hash::Sha256Hasher;
use veridata_core::model::Verdict;
use veridata_core::recon::reconcile;
use veridata_core::testutil::{default_policy, fingerprints_from_records, sample_records};

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn clean_sets_always_pass(n in 1usize..20) {
        let policy = default_policy();
        let recs = sample_records(n);
        let fps = fingerprints_from_records(&recs, &policy, 0);
        let out = reconcile(&fps, &fps, &policy, &Sha256Hasher).unwrap();
        prop_assert_eq!(out.result.verdict, Verdict::Pass);
    }

    #[test]
    fn source_count_equals_matched_plus_missing_plus_mutated(n in 2usize..12) {
        let policy = default_policy();
        let recs = sample_records(n);
        let src = fingerprints_from_records(&recs, &policy, 0);
        let mut snk = src.clone();
        if !snk.is_empty() {
            let mut recs2 = recs.clone();
            recs2[0].insert(
                "amount".into(),
                veridata_core::canon::CanonValue::String("dec:99999.0".into()),
            );
            snk[0] = veridata_core::recon::build_fingerprint(
                &recs2[0],
                &veridata_core::policy_util::effective_content_fields(
                    &policy.content_fields,
                    &policy.exclude_fields,
                ),
                &veridata_core::testutil::TEST_SALT,
                veridata_core::recon::kafka_pos(0),
                &policy,
            )
            .unwrap();
        }
        let out = reconcile(&src, &snk, &policy, &Sha256Hasher).unwrap();
        let m = out.result.matched.count;
        let missing = out.result.missing.len() as u64;
        let mutated = out.result.mutated.len() as u64;
        prop_assert_eq!(m + missing + mutated, src.len() as u64);
    }

    #[test]
    fn drop_always_detected(n in 2usize..15) {
        let policy = default_policy();
        let recs = sample_records(n);
        let src = fingerprints_from_records(&recs, &policy, 0);
        let snk: Vec<_> = src.iter().take(n - 1).cloned().collect();
        let out = reconcile(&src, &snk, &policy, &Sha256Hasher).unwrap();
        prop_assert_eq!(out.result.verdict, Verdict::Fail);
        prop_assert!(!out.result.missing.is_empty());
    }
}
