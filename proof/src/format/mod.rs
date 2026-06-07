mod error;
pub mod jcs;
mod types;

pub use error::{VrpError, VrpResult};
pub use types::*;

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use sha2::{Digest, Sha256};
use veridata_core::hash::{hasher_for, merkle_leaf, merkle_root};
use veridata_core::model::{Boundary, BoundaryMode, Fingerprint, Hash32, Policy, Verdict};
use veridata_core::recon::reconcile;

use crate::sign::Signer;

pub fn build_vrp(
    source: &[Fingerprint],
    sink: &[Fingerprint],
    policy: &Policy,
    boundary: Boundary,
    source_ref: &str,
    sink_ref: &str,
    salt: &[u8],
    producer: &str,
    created_at: &str,
    prev_proof_hash: Option<Hash32>,
    signer: &dyn Signer,
) -> VrpResult<VrpDocument> {
    if salt.len() != 32 {
        return Err(VrpError::Invalid("salt must be 32 bytes".into()));
    }
    let hasher = hasher_for(&policy.hash_algorithm)?;
    let recon = reconcile(source, sink, policy, hasher.as_ref())?;

    let mut source_leaves: Vec<Hash32> = source
        .iter()
        .map(|f| merkle_leaf(hasher.as_ref(), &f.fp))
        .collect();
    source_leaves.sort();
    let mut sink_leaves: Vec<Hash32> = sink
        .iter()
        .map(|f| merkle_leaf(hasher.as_ref(), &f.fp))
        .collect();
    sink_leaves.sort();

    let mut doc = VrpDocument {
        vrp_version: "0.1".into(),
        proof_id: "0".repeat(64),
        created_at: created_at.into(),
        producer: producer.into(),
        boundary: BoundaryJson {
            mode: boundary_mode_str(&boundary.mode).into(),
            value: B64.encode(&boundary.value),
        },
        source_ref: source_ref.into(),
        sink_ref: sink_ref.into(),
        hash_algorithm: policy.hash_algorithm.clone(),
        canon_version: policy.canon.version,
        salt: B64.encode(salt),
        source_commitment: CommitmentJson {
            count: source.len() as u64,
            merkle_root: hex::encode(merkle_root(hasher.as_ref(), source_leaves)),
        },
        sink_commitment: CommitmentJson {
            count: sink.len() as u64,
            merkle_root: hex::encode(merkle_root(hasher.as_ref(), sink_leaves)),
        },
        reconciliation: ReconciliationJson::from(&recon.result),
        policy: PolicyJson::from(policy),
        chain: ChainJson {
            prev_proof_hash: prev_proof_hash.map(hex::encode),
        },
        signature: SignatureJson {
            alg: "ed25519".into(),
            public_key: signer.public_key_b64(),
            sig: String::new(),
        },
    };

    let payload = jcs::signing_payload(&doc)?;
    doc.proof_id = hex::encode(Sha256::digest(&payload));
    let sig = signer.sign(&payload)?;
    doc.signature.sig = B64.encode(sig);

    Ok(doc)
}

fn boundary_mode_str(mode: &BoundaryMode) -> &'static str {
    match mode {
        BoundaryMode::OffsetRange => "OFFSET_RANGE",
        BoundaryMode::TimeWindow => "TIME_WINDOW",
        BoundaryMode::BatchId => "BATCH_ID",
    }
}

pub fn verdict_str(v: Verdict) -> &'static str {
    match v {
        Verdict::Pass => "PASS",
        Verdict::Fail => "FAIL",
        Verdict::Unverified => "UNVERIFIED",
    }
}

pub fn parse_verdict(s: &str) -> VrpResult<Verdict> {
    match s {
        "PASS" => Ok(Verdict::Pass),
        "FAIL" => Ok(Verdict::Fail),
        "UNVERIFIED" => Ok(Verdict::Unverified),
        _ => Err(VrpError::Invalid(format!("unknown verdict: {s}"))),
    }
}

pub fn parse_hash32(hex_str: &str) -> VrpResult<Hash32> {
    let bytes = hex::decode(hex_str).map_err(|e| VrpError::Invalid(e.to_string()))?;
    if bytes.len() != 32 {
        return Err(VrpError::Invalid("hash must be 32 bytes".into()));
    }
    let mut out = [0u8; 32];
    out.copy_from_slice(&bytes);
    Ok(out)
}
