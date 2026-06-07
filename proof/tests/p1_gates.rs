use std::path::PathBuf;

use veridata_core::model::{Boundary, BoundaryMode, Verdict};
use veridata_core::testutil::{
    default_policy, fingerprints_from_records, sample_records, TEST_SALT,
};
use veridata_proof::format::VrpDocument;
use veridata_proof::sign::KeyPair;
use veridata_proof::{build_vrp, Signer, VerifyOutcome, Verifier};

fn conformance_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../conformance")
}

fn test_pubkey() -> String {
    std::fs::read_to_string(conformance_dir().join("test-key.pub.b64"))
        .unwrap()
        .trim()
        .to_string()
}

#[test]
fn ac_c4_1_valid_conformance_passes() {
    let path = conformance_dir().join("valid.vrp.json");
    let text = std::fs::read_to_string(&path).unwrap();
    let doc: VrpDocument = serde_json::from_str(&text).unwrap();
    let verifier = Verifier::from_public_key_b64(&test_pubkey()).unwrap();
    assert_eq!(verifier.verify(&doc).unwrap(), VerifyOutcome::Pass);
}

#[test]
fn ac_c4_2_tampered_conformance_fails() {
    let path = conformance_dir().join("tampered.vrp.json");
    let text = std::fs::read_to_string(&path).unwrap();
    let doc: VrpDocument = serde_json::from_str(&text).unwrap();
    let verifier = Verifier::from_public_key_b64(&test_pubkey()).unwrap();
    assert_eq!(verifier.verify(&doc).unwrap(), VerifyOutcome::Fail);
}

#[test]
fn ac_c4_3_fail_vectors_detect_bad_verdict_or_sig() {
    for name in ["drop", "dup", "mutated"] {
        let path = conformance_dir().join(format!("{name}.vrp.json"));
        let text = std::fs::read_to_string(&path).unwrap();
        let doc: VrpDocument = serde_json::from_str(&text).unwrap();
        let verifier = Verifier::from_public_key_b64(&test_pubkey()).unwrap();
        assert_eq!(
            verifier.verify(&doc).unwrap(),
            VerifyOutcome::Fail,
            "{name} should FAIL"
        );
    }
}

#[test]
fn ac_c3_1_sign_and_verify_matching_key() {
    let policy = default_policy();
    let recs = sample_records(3);
    let src = fingerprints_from_records(&recs, &policy, 0);
    let snk = src.clone();
    let boundary = Boundary {
        mode: BoundaryMode::OffsetRange,
        value: br#"{"partitions":[{"id":0,"start":0,"end":99}]}"#.to_vec(),
    };
    let signer = KeyPair::test_key();
    let doc = build_vrp(
        &src,
        &snk,
        &policy,
        boundary,
        "kafka:orders",
        "iceberg:warehouse.orders",
        &TEST_SALT,
        "veridata/0.1.0",
        "2026-06-07T00:00:00Z",
        None,
        &signer,
    )
    .unwrap();
    let verifier = Verifier::from_public_key_b64(&signer.public_key_b64()).unwrap();
    assert_eq!(verifier.verify(&doc).unwrap(), VerifyOutcome::Pass);
}

#[test]
fn ac_c3_2_wrong_key_fails() {
    let policy = default_policy();
    let recs = sample_records(2);
    let src = fingerprints_from_records(&recs, &policy, 0);
    let boundary = Boundary {
        mode: BoundaryMode::OffsetRange,
        value: vec![1],
    };
    let signer = KeyPair::test_key();
    let doc = build_vrp(
        &src,
        &src,
        &policy,
        boundary,
        "kafka:a",
        "iceberg:b",
        &TEST_SALT,
        "veridata/0.1.0",
        "2026-06-07T00:00:00Z",
        None,
        &signer,
    )
    .unwrap();
    let other = KeyPair::from_seed([1u8; 32]);
    let verifier = Verifier::from_public_key_b64(&other.public_key_b64()).unwrap();
    assert_eq!(verifier.verify(&doc).unwrap(), VerifyOutcome::Fail);
}

#[test]
fn ac_c1_1_all_fields_present() {
    let policy = default_policy();
    let recs = sample_records(1);
    let src = fingerprints_from_records(&recs, &policy, 0);
    let boundary = Boundary {
        mode: BoundaryMode::OffsetRange,
        value: vec![],
    };
    let signer = KeyPair::test_key();
    let doc = build_vrp(
        &src,
        &src,
        &policy,
        boundary,
        "kafka:a",
        "iceberg:b",
        &TEST_SALT,
        "veridata/0.1.0",
        "2026-06-07T00:00:00Z",
        None,
        &signer,
    )
    .unwrap();
    assert_eq!(doc.vrp_version, "0.1");
    assert_eq!(doc.signature.alg, "ed25519");
    assert!(!doc.proof_id.is_empty());
}

#[test]
fn ac_c1_2_deterministic_except_created_at() {
    let policy = default_policy();
    let recs = sample_records(4);
    let src = fingerprints_from_records(&recs, &policy, 0);
    let boundary = Boundary {
        mode: BoundaryMode::OffsetRange,
        value: vec![0],
    };
    let signer = KeyPair::test_key();
    let build = |ts: &str| {
        build_vrp(
            &src,
            &src,
            &policy,
            boundary.clone(),
            "kafka:orders",
            "iceberg:warehouse.orders",
            &TEST_SALT,
            "veridata/0.1.0",
            ts,
            None,
            &signer,
        )
        .unwrap()
    };
    let mut d1 = build("2026-06-07T00:00:00Z");
    let mut d2 = build("2026-06-07T12:00:00Z");
    d1.created_at = String::new();
    d2.created_at = String::new();
    assert_eq!(d1, d2);
}

#[test]
fn ac_c2_1_chain_links() {
    let policy = default_policy();
    let recs = sample_records(2);
    let src = fingerprints_from_records(&recs, &policy, 0);
    let boundary = Boundary {
        mode: BoundaryMode::OffsetRange,
        value: vec![],
    };
    let signer = KeyPair::test_key();
    let first = build_vrp(
        &src,
        &src,
        &policy,
        boundary.clone(),
        "kafka:a",
        "iceberg:b",
        &TEST_SALT,
        "veridata/0.1.0",
        "2026-06-07T00:00:00Z",
        None,
        &signer,
    )
    .unwrap();
    let prev = veridata_proof::parse_hash32(&first.proof_id).unwrap();
    let second = build_vrp(
        &src,
        &src,
        &policy,
        boundary,
        "kafka:a",
        "iceberg:b",
        &TEST_SALT,
        "veridata/0.1.0",
        "2026-06-07T01:00:00Z",
        Some(prev),
        &signer,
    )
    .unwrap();
    assert_eq!(second.chain.prev_proof_hash.as_deref(), Some(first.proof_id.as_str()));
}

#[test]
fn tamper_gate_any_flipped_byte_fails() {
    let path = conformance_dir().join("valid.vrp.json");
    let text = std::fs::read_to_string(&path).unwrap();
    let mut doc: VrpDocument = serde_json::from_str(&text).unwrap();
    // Flip one hex nibble in proof_id (post-parse tamper)
    let mut chars: Vec<char> = doc.proof_id.chars().collect();
    chars[0] = if chars[0] == 'a' { 'b' } else { 'a' };
    doc.proof_id = chars.into_iter().collect();
    let verifier = Verifier::from_public_key_b64(&test_pubkey()).unwrap();
    assert_eq!(verifier.verify(&doc).unwrap(), VerifyOutcome::Fail);
}

#[test]
fn privacy_gate_no_raw_values_in_built_vrp() {
    let policy = default_policy();
    let recs = sample_records(3);
    let src = fingerprints_from_records(&recs, &policy, 0);
    let boundary = Boundary {
        mode: BoundaryMode::OffsetRange,
        value: vec![],
    };
    let signer = KeyPair::test_key();
    let doc = build_vrp(
        &src,
        &src,
        &policy,
        boundary,
        "kafka:orders",
        "iceberg:warehouse.orders",
        &TEST_SALT,
        "veridata/0.1.0",
        "2026-06-07T00:00:00Z",
        None,
        &signer,
    )
    .unwrap();
    let json = serde_json::to_string(&doc).unwrap();
    assert!(!json.contains("1000"));
    assert!(!json.contains("shipped"));
}

#[test]
fn honesty_gate_unverified_with_reason() {
    use veridata_core::model::Tolerances;
    use veridata_core::recon::derive_verdict;
    let v = derive_verdict(
        0,
        0,
        0,
        Some("boundary mismatch".into()),
        &Tolerances::default(),
    );
    assert_eq!(v, Verdict::Unverified);
}

#[test]
fn ac_c5_1_inclusion_proof_structure_present() {
    let path = conformance_dir().join("drop.vrp.json");
    let text = std::fs::read_to_string(&path).unwrap();
    let doc: VrpDocument = serde_json::from_str(&text).unwrap();
    assert!(!doc.reconciliation.missing.is_empty());
    assert!(!doc.reconciliation.missing[0].inclusion_proof.is_empty());
    assert!(!doc.reconciliation.missing[0].merkle_leaf.is_empty());
}

#[test]
fn ac_c5_2_tampered_inclusion_proof_fails() {
    let path = conformance_dir().join("drop.vrp.json");
    let text = std::fs::read_to_string(&path).unwrap();
    let mut doc: VrpDocument = serde_json::from_str(&text).unwrap();
    let proof = &mut doc.reconciliation.missing[0].inclusion_proof;
    let mut chars: Vec<char> = proof[0].chars().collect();
    chars[0] = if chars[0] == 'a' { 'b' } else { 'a' };
    proof[0] = chars.into_iter().collect();
    let verifier = Verifier::from_public_key_b64(&test_pubkey()).unwrap();
    assert_eq!(verifier.verify(&doc).unwrap(), VerifyOutcome::Fail);
}
