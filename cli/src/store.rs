use std::path::{Path, PathBuf};

use veridata_cloud::config::CloudStoreConfig;
use veridata_cloud::CloudProofStore;
use veridata_proof::VrpDocument;

use crate::config::{StoreConfig, StoreKind};

pub struct ProofStore {
    inner: CloudProofStore,
    local_dir: PathBuf,
}

impl ProofStore {
    pub fn from_config(config: &StoreConfig) -> anyhow::Result<Self> {
        let cloud_cfg = match config.kind {
            StoreKind::Local => CloudStoreConfig::Local {
                path: config.proofs_dir.to_string_lossy().into(),
            },
            StoreKind::S3 => CloudStoreConfig::S3 {
                bucket: config
                    .bucket
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("store.bucket required for s3"))?,
                prefix: config.prefix.clone().unwrap_or_default(),
                region: config.region.clone(),
            },
            StoreKind::Gcs => CloudStoreConfig::Gcs {
                bucket: config
                    .bucket
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("store.bucket required for gcs"))?,
                prefix: config.prefix.clone().unwrap_or_default(),
            },
            StoreKind::Adls => CloudStoreConfig::Adls {
                account: config
                    .account
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("store.account required for adls"))?,
                container: config
                    .container
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("store.container required for adls"))?,
                prefix: config.prefix.clone().unwrap_or_default(),
            },
        };
        let inner = CloudProofStore::from_config(&cloud_cfg)
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        Ok(Self {
            inner,
            local_dir: config.proofs_dir.clone(),
        })
    }

    pub fn new(dir: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let dir = dir.into();
        Self::from_config(&StoreConfig {
            kind: StoreKind::Local,
            proofs_dir: dir,
            bucket: None,
            prefix: None,
            region: None,
            account: None,
            container: None,
        })
    }

    pub fn save(&self, doc: &VrpDocument) -> anyhow::Result<PathBuf> {
        self.inner.save(doc).map_err(|e| anyhow::anyhow!("{e}"))
    }

    pub fn resolve(&self, selector: &str) -> anyhow::Result<PathBuf> {
        self.inner.resolve(selector).map_err(|e| anyhow::anyhow!("{e}"))
    }

    pub fn load(&self, path: &Path) -> anyhow::Result<VrpDocument> {
        self.inner.load(path).map_err(|e| anyhow::anyhow!("{e}"))
    }

    pub fn proofs_dir(&self) -> &Path {
        &self.local_dir
    }
}
