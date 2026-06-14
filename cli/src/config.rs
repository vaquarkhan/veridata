use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use veridata_core::model::{CanonSpec, DuplicatePolicy, Policy, Tolerances};

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KmsProvider {
    #[default]
    File,
    Aws,
    Gcp,
    Azure,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StoreKind {
    #[default]
    Local,
    S3,
    Gcs,
    Adls,
}

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
    #[serde(default)]
    pub bootstrap_servers: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub project: Option<String>,
    #[serde(default)]
    pub subscription: Option<String>,
    #[serde(default)]
    pub connection_string: Option<String>,
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
    #[serde(default)]
    pub warehouse: Option<PathBuf>,
    #[serde(default)]
    pub warehouse_uri: Option<String>,
    pub table: String,
    #[serde(default)]
    pub dataset: Option<String>,
    #[serde(default)]
    pub catalog: Option<String>,
    #[serde(default)]
    pub schema: Option<String>,
    pub boundary: IcebergBoundaryConfig,
    #[serde(default)]
    pub expected_schema: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IcebergBoundaryConfig {
    pub snapshot_from: i64,
    pub snapshot_to: i64,
    #[serde(default)]
    pub sql_filter: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    pub identity_rule: String,
    pub hash_algorithm: String,
    pub content_fields: Vec<String>,
    #[serde(default)]
    pub exclude_fields: Vec<String>,
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
    #[serde(default)]
    pub kms_provider: KmsProvider,
    #[serde(default)]
    pub kms_key_id: Option<String>,
    #[serde(default)]
    pub aws_region: Option<String>,
    #[serde(default)]
    pub azure_vault_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoreConfig {
    #[serde(default)]
    pub kind: StoreKind,
    #[serde(default)]
    pub proofs_dir: PathBuf,
    #[serde(default)]
    pub bucket: Option<String>,
    #[serde(default)]
    pub prefix: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub account: Option<String>,
    #[serde(default)]
    pub container: Option<String>,
}

impl ReconConfig {
    pub fn default_template() -> Self {
        Self {
            producer: "veridata/0.1.0".into(),
            source: SourceConfig {
                kind: "memory_kafka".into(),
                topic: "orders".into(),
                bootstrap_servers: None,
                region: None,
                project: None,
                subscription: None,
                connection_string: None,
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
                warehouse: Some(PathBuf::from(".veridata/warehouse")),
                warehouse_uri: None,
                table: "orders".into(),
                dataset: None,
                catalog: None,
                schema: None,
                boundary: IcebergBoundaryConfig {
                    snapshot_from: 1,
                    snapshot_to: 1,
                    sql_filter: None,
                },
                expected_schema: None,
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
                exclude_fields: vec![],
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
                kms_provider: KmsProvider::File,
                kms_key_id: None,
                aws_region: None,
                azure_vault_url: None,
            },
            store: StoreConfig {
                kind: StoreKind::Local,
                proofs_dir: PathBuf::from(".veridata/proofs"),
                bucket: None,
                prefix: None,
                region: None,
                account: None,
                container: None,
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
            content_fields: self.policy.content_fields.clone(),
            exclude_fields: self.policy.exclude_fields.clone(),
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
