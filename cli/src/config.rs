use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use veridata_core::model::{CanonSpec, DuplicatePolicy, Policy, Tolerances};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReconConfig {
    pub producer: String,
    pub source: SourceConfig,
    pub sink: SinkConfig,
    pub policy: PolicyConfig,
    pub crypto: CryptoConfig,
    pub store: StoreConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceConfig {
    #[serde(rename = "type")]
    pub kind: String,
    pub topic: String,
    pub boundary: KafkaBoundaryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KafkaBoundaryConfig {
    pub partitions: Vec<PartitionRange>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartitionRange {
    pub id: i32,
    pub start: i64,
    pub end: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinkConfig {
    #[serde(rename = "type")]
    pub kind: String,
    pub warehouse: PathBuf,
    pub table: String,
    pub boundary: IcebergBoundaryConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcebergBoundaryConfig {
    pub snapshot_from: i64,
    pub snapshot_to: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub identity_rule: String,
    pub hash_algorithm: String,
    pub content_fields: Vec<String>,
    pub tolerances: TolerancesConfig,
    pub late_arrival_window: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TolerancesConfig {
    pub max_drops: u64,
    pub max_mutations: u64,
    pub duplicates: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoConfig {
    pub private_key_file: PathBuf,
    pub public_key_file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreConfig {
    pub proofs_dir: PathBuf,
}

impl ReconConfig {
    pub fn default_template() -> Self {
        Self {
            producer: "veridata/0.1.0".into(),
            source: SourceConfig {
                kind: "memory_kafka".into(),
                topic: "orders".into(),
                boundary: KafkaBoundaryConfig {
                    partitions: vec![PartitionRange {
                        id: 0,
                        start: 0,
                        end: 4,
                    }],
                },
            },
            sink: SinkConfig {
                kind: "iceberg".into(),
                warehouse: PathBuf::from(".veridata/warehouse"),
                table: "orders".into(),
                boundary: IcebergBoundaryConfig {
                    snapshot_from: 1,
                    snapshot_to: 1,
                },
            },
            policy: PolicyConfig {
                identity_rule: "composite:[order_id,line_id]".into(),
                hash_algorithm: "sha256".into(),
                content_fields: vec![
                    "order_id".into(),
                    "line_id".into(),
                    "amount".into(),
                    "status".into(),
                ],
                tolerances: TolerancesConfig {
                    max_drops: 0,
                    max_mutations: 0,
                    duplicates: "FORBID".into(),
                },
                late_arrival_window: Some("900s".into()),
            },
            crypto: CryptoConfig {
                private_key_file: PathBuf::from(".veridata/keys/signing.key.b64"),
                public_key_file: PathBuf::from(".veridata/keys/signing.pub.b64"),
            },
            store: StoreConfig {
                proofs_dir: PathBuf::from(".veridata/proofs"),
            },
        }
    }

    pub fn load(path: &Path) -> anyhow::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        Ok(serde_yaml::from_str(&text)?)
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let text = serde_yaml::to_string(self)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, text)?;
        Ok(())
    }

    pub fn to_core_policy(&self) -> anyhow::Result<Policy> {
        let duplicates = match self.policy.tolerances.duplicates.as_str() {
            "FORBID" => DuplicatePolicy::Forbid,
            "ALLOW_IF_SINK_IDEMPOTENT" => DuplicatePolicy::AllowIfSinkIdempotent,
            other => anyhow::bail!("unknown duplicate policy: {other}"),
        };
        Ok(Policy {
            identity_rule: self.policy.identity_rule.clone(),
            canon: CanonSpec::default(),
            hash_algorithm: self.policy.hash_algorithm.clone(),
            tolerances: Tolerances {
                max_drops: self.policy.tolerances.max_drops,
                duplicates,
                max_mutations: self.policy.tolerances.max_mutations,
            },
            late_arrival_window: self
                .policy
                .late_arrival_window
                .clone()
                .unwrap_or_else(|| "900s".into()),
        })
    }
}
