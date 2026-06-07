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
