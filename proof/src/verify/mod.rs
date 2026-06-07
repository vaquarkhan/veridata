use base64::{engine::general_purpose::STANDARD as B64, Engine};
use ed25519_dalek::{Signature, Verifier as DalekVerifier, VerifyingKey};
use sha2::{Digest, Sha256};
use veridata_core::hash::{hasher_for, verify_merkle_proof};
use veridata_core::model::Hash32;
use veridata_core::model::{DuplicatePolicy, Verdict};
use veridata_core::recon::derive_verdict;

use crate::format::jcs;
use crate::format::{parse_hash32, parse_verdict, VrpDocument, VrpError, VrpResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VerifyOutcome {
    Pass,
    Fail,
    Unverified,
}

pub struct Verifier {
    pub public_key: VerifyingKey,
}

impl Verifier {
    pub fn from_public_key_b64(b64: &str) -> VrpResult<Self> {
        let bytes = B64
            .decode(b64.trim())
            .map_err(|e| VrpError::Invalid(e.to_string()))?;
        if bytes.len() != 32 {
            return Err(VrpError::Invalid("public key must be 32 bytes".into()));
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self {
            public_key: VerifyingKey::from_bytes(&arr)
                .map_err(|e| VrpError::Invalid(e.to_string()))?,
        })
    }

    pub fn verify(&self, doc: &VrpDocument) -> VrpResult<VerifyOutcome> {
        if doc.vrp_version != "0.1" {
            return Ok(VerifyOutcome::Fail);
        }

        let payload = jcs::signing_payload(doc)?;
        let proof_id = hex::encode(Sha256::digest(&payload));
        if proof_id != doc.proof_id {
            return Ok(VerifyOutcome::Fail);
        }

        let sig_bytes = B64
            .decode(&doc.signature.sig)
            .map_err(|e| VrpError::Invalid(e.to_string()))?;
        let sig = Signature::from_slice(&sig_bytes)
            .map_err(|e| VrpError::Invalid(e.to_string()))?;
        if self.public_key.verify(&payload, &sig).is_err() {
            return Ok(VerifyOutcome::Fail);
        }

        if hasher_for(&doc.hash_algorithm).is_err() {
            return Ok(VerifyOutcome::Unverified);
        }
        if doc.canon_version != 1 {
            return Ok(VerifyOutcome::Unverified);
        }

        let tolerances = parse_tolerances(&doc.policy.tolerances)?;
        let recomputed = derive_verdict(
            doc.reconciliation.missing.len() as u64,
            doc.reconciliation.duplicated.len() as u64,
            doc.reconciliation.mutated.len() as u64,
            doc.reconciliation.unverified_reason.clone(),
            &tolerances,
        );
        let embedded = parse_verdict(&doc.reconciliation.verdict)?;
        if recomputed != embedded {
            return Ok(VerifyOutcome::Fail);
        }

        if embedded == Verdict::Unverified
            && doc
                .reconciliation
                .unverified_reason
                .as_ref()
                .map(|s| s.is_empty())
                .unwrap_or(true)
        {
            return Ok(VerifyOutcome::Fail);
        }

        if embedded == Verdict::Pass {
            if doc.source_commitment.count != doc.sink_commitment.count {
                return Ok(VerifyOutcome::Fail);
            }
            if !doc.reconciliation.missing.is_empty() {
                return Ok(VerifyOutcome::Fail);
            }
        }

        // AC-C5: validate inclusion proofs for missing records
        if !doc.reconciliation.missing.is_empty() {
            let hasher = hasher_for(&doc.hash_algorithm)?;
            let source_root = parse_hash32(&doc.source_commitment.merkle_root)?;
            let leaf_count = doc.source_commitment.count as usize;
            for m in &doc.reconciliation.missing {
                if !verify_missing_inclusion(hasher.as_ref(), &source_root, leaf_count, m)? {
                    return Ok(VerifyOutcome::Fail);
                }
            }
        }

        // Chain linkage
        if let Some(prev) = &doc.chain.prev_proof_hash {
            parse_hash32(prev)?;
        }

        Ok(match embedded {
            Verdict::Pass => VerifyOutcome::Pass,
            Verdict::Fail => VerifyOutcome::Fail,
            Verdict::Unverified => VerifyOutcome::Unverified,
        })
    }
}

fn parse_tolerances(t: &crate::format::TolerancesJson) -> VrpResult<veridata_core::model::Tolerances> {
    let duplicates = match t.duplicates.as_str() {
        "FORBID" => DuplicatePolicy::Forbid,
        "ALLOW_IF_SINK_IDEMPOTENT" => DuplicatePolicy::AllowIfSinkIdempotent,
        _ => return Err(VrpError::Invalid(format!("unknown duplicate policy: {}", t.duplicates))),
    };
    Ok(veridata_core::model::Tolerances {
        max_drops: t.max_drops,
        duplicates,
        max_mutations: t.max_mutations,
    })
}

fn verify_missing_inclusion(
    hasher: &dyn veridata_core::hash::Hasher,
    root: &Hash32,
    leaf_count: usize,
    m: &crate::format::MissingJson,
) -> VrpResult<bool> {
    let leaf = parse_hash32(&m.merkle_leaf)?;
    let proof: Vec<Hash32> = m
        .inclusion_proof
        .iter()
        .map(|h| parse_hash32(h))
        .collect::<VrpResult<_>>()?;
    Ok(verify_merkle_proof(hasher, root, &leaf, &proof, leaf_count))
}

pub fn verify_file(path: &std::path::Path, pubkey_b64: &str) -> VrpResult<VerifyOutcome> {
    let text = std::fs::read_to_string(path).map_err(|e| VrpError::Invalid(e.to_string()))?;
    let doc: VrpDocument = serde_json::from_str(&text)?;
    Verifier::from_public_key_b64(pubkey_b64)?.verify(&doc)
}
