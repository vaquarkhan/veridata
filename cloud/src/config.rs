use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum KmsProviderConfig {
    File,
    Aws,
    Gcp,
    Azure,
}

impl Default for KmsProviderConfig {
    fn default() -> Self {
        Self::File
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CloudStoreConfig {
    Local { path: String },
    S3 { bucket: String, prefix: String, region: Option<String> },
    Gcs { bucket: String, prefix: String },
    Adls { account: String, container: String, prefix: String },
}

impl Default for CloudStoreConfig {
    fn default() -> Self {
        Self::Local {
            path: ".veridata/proofs".into(),
        }
    }
}
