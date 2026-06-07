use serde::{Deserialize, Serialize};
use veridata_core::model::{
    Commitment, DuplicatedRecord, MissingRecord, MutatedRecord, Policy, ReconResult,
};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VrpDocument {
    pub vrp_version: String,
    pub proof_id: String,
    pub created_at: String,
    pub producer: String,
    pub boundary: BoundaryJson,
    pub source_ref: String,
    pub sink_ref: String,
    pub hash_algorithm: String,
    pub canon_version: u32,
    pub salt: String,
    pub source_commitment: CommitmentJson,
    pub sink_commitment: CommitmentJson,
    pub reconciliation: ReconciliationJson,
    pub policy: PolicyJson,
    pub chain: ChainJson,
    pub signature: SignatureJson,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BoundaryJson {
    pub mode: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommitmentJson {
    pub count: u64,
    pub merkle_root: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MissingJson {
    pub id_hash: String,
    pub source_pos: String,
    pub inclusion_proof: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DuplicatedJson {
    pub id_hash: String,
    pub source_multiplicity: u64,
    pub sink_multiplicity: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MutatedJson {
    pub id_hash: String,
    pub source_content_hash: String,
    pub sink_content_hash: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReconciliationJson {
    pub matched: CommitmentJson,
    pub missing: Vec<MissingJson>,
    pub duplicated: Vec<DuplicatedJson>,
    pub mutated: Vec<MutatedJson>,
    pub verdict: String,
    pub unverified_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CanonSpecJson {
    pub version: u32,
    pub timestamp_precision: String,
    pub decimal_scale: u32,
    pub unicode: String,
    pub null_token: String,
    pub field_order: String,
    pub array_as_set: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TolerancesJson {
    pub max_drops: u64,
    pub duplicates: String,
    pub max_mutations: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PolicyJson {
    pub identity_rule: String,
    pub canon: CanonSpecJson,
    pub hash_algorithm: String,
    pub tolerances: TolerancesJson,
    pub late_arrival_window: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChainJson {
    pub prev_proof_hash: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SignatureJson {
    pub alg: String,
    pub public_key: String,
    pub sig: String,
}

impl From<&ReconResult> for ReconciliationJson {
    fn from(r: &ReconResult) -> Self {
        Self {
            matched: CommitmentJson::from(&r.matched),
            missing: r.missing.iter().map(MissingJson::from).collect(),
            duplicated: r.duplicated.iter().map(DuplicatedJson::from).collect(),
            mutated: r.mutated.iter().map(MutatedJson::from).collect(),
            verdict: super::verdict_str(r.verdict).into(),
            unverified_reason: r.unverified_reason.clone(),
        }
    }
}

impl From<&Commitment> for CommitmentJson {
    fn from(c: &Commitment) -> Self {
        Self {
            count: c.count,
            merkle_root: hex::encode(c.merkle_root),
        }
    }
}

impl From<&MissingRecord> for MissingJson {
    fn from(m: &MissingRecord) -> Self {
        use base64::{engine::general_purpose::STANDARD as B64, Engine};
        let pos_json = serde_json::json!({
            "kind": position_kind_str(&m.source_pos.kind),
            "value": hex::encode(&m.source_pos.value),
        });
        Self {
            id_hash: hex::encode(m.id_hash),
            source_pos: B64.encode(pos_json.to_string()),
            inclusion_proof: m.inclusion_proof.iter().map(hex::encode).collect(),
        }
    }
}

fn position_kind_str(kind: &veridata_core::model::PositionKind) -> &'static str {
    use veridata_core::model::PositionKind;
    match kind {
        PositionKind::KafkaOffset => "KAFKA_OFFSET",
        PositionKind::PubsubMsgid => "PUBSUB_MSGID",
        PositionKind::IcebergRow => "ICEBERG_ROW",
        PositionKind::CdcLsn => "CDC_LSN",
        PositionKind::FileRow => "FILE_ROW",
    }
}

impl From<&DuplicatedRecord> for DuplicatedJson {
    fn from(d: &DuplicatedRecord) -> Self {
        Self {
            id_hash: hex::encode(d.id_hash),
            source_multiplicity: d.source_multiplicity,
            sink_multiplicity: d.sink_multiplicity,
        }
    }
}

impl From<&MutatedRecord> for MutatedJson {
    fn from(m: &MutatedRecord) -> Self {
        Self {
            id_hash: hex::encode(m.id_hash),
            source_content_hash: hex::encode(m.source_content_hash),
            sink_content_hash: hex::encode(m.sink_content_hash),
        }
    }
}

impl From<&Policy> for PolicyJson {
    fn from(p: &Policy) -> Self {
        use veridata_core::model::DuplicatePolicy;
        Self {
            identity_rule: p.identity_rule.clone(),
            canon: CanonSpecJson {
                version: p.canon.version,
                timestamp_precision: p.canon.timestamp_precision.clone(),
                decimal_scale: p.canon.decimal_scale,
                unicode: p.canon.unicode.clone(),
                null_token: p.canon.null_token.clone(),
                field_order: p.canon.field_order.clone(),
                array_as_set: p.canon.array_as_set,
            },
            hash_algorithm: p.hash_algorithm.clone(),
            tolerances: TolerancesJson {
                max_drops: p.tolerances.max_drops,
                duplicates: match p.tolerances.duplicates {
                    DuplicatePolicy::Forbid => "FORBID",
                    DuplicatePolicy::AllowIfSinkIdempotent => "ALLOW_IF_SINK_IDEMPOTENT",
                }
                .into(),
                max_mutations: p.tolerances.max_mutations,
            },
            late_arrival_window: p.late_arrival_window.clone(),
        }
    }
}
