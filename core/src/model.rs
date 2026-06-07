use serde::{Deserialize, Serialize};

pub type Hash32 = [u8; 32];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PositionKind {
    KafkaOffset,
    PubsubMsgid,
    IcebergRow,
    CdcLsn,
    FileRow,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Position {
    pub kind: PositionKind,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fingerprint {
    pub id_hash: Hash32,
    pub content_hash: Hash32,
    pub fp: Hash32,
    pub pos: Position,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Side {
    Source,
    Sink,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BoundaryMode {
    OffsetRange,
    TimeWindow,
    BatchId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Boundary {
    pub mode: BoundaryMode,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonSpec {
    pub version: u32,
    pub timestamp_precision: String,
    pub decimal_scale: u32,
    pub unicode: String,
    pub null_token: String,
    pub field_order: String,
    pub array_as_set: bool,
}

impl Default for CanonSpec {
    fn default() -> Self {
        Self {
            version: 1,
            timestamp_precision: "micros".into(),
            decimal_scale: 6,
            unicode: "NFC".into(),
            null_token: "\u{0000}".into(),
            field_order: "lexicographic".into(),
            array_as_set: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum DuplicatePolicy {
    Forbid,
    AllowIfSinkIdempotent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tolerances {
    pub max_drops: u64,
    pub duplicates: DuplicatePolicy,
    pub max_mutations: u64,
}

impl Default for Tolerances {
    fn default() -> Self {
        Self {
            max_drops: 0,
            duplicates: DuplicatePolicy::Forbid,
            max_mutations: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Policy {
    pub identity_rule: String,
    pub canon: CanonSpec,
    pub hash_algorithm: String,
    pub tolerances: Tolerances,
    pub late_arrival_window: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commitment {
    pub count: u64,
    pub merkle_root: Hash32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissingRecord {
    pub id_hash: Hash32,
    pub source_pos: Position,
    pub inclusion_proof: Vec<Hash32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DuplicatedRecord {
    pub id_hash: Hash32,
    pub source_multiplicity: u64,
    pub sink_multiplicity: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MutatedRecord {
    pub id_hash: Hash32,
    pub source_content_hash: Hash32,
    pub sink_content_hash: Hash32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Verdict {
    Pass,
    Fail,
    Unverified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReconResult {
    pub matched: Commitment,
    pub missing: Vec<MissingRecord>,
    pub duplicated: Vec<DuplicatedRecord>,
    pub mutated: Vec<MutatedRecord>,
    pub verdict: Verdict,
    pub unverified_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FingerprintSet {
    pub side: Side,
    pub boundary: Boundary,
    pub fingerprints: Vec<Fingerprint>,
}

impl FingerprintSet {
    pub fn count(&self) -> u64 {
        self.fingerprints.len() as u64
    }
}
