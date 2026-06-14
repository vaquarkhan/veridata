use std::path::{Path, PathBuf};

use futures::StreamExt;
use object_store::path::Path as ObjectPath;
use object_store::{ObjectStore, PutPayload};
use veridata_proof::VrpDocument;

use crate::config::CloudStoreConfig;

pub struct CloudProofStore {
    backend: StoreBackend,
}

enum StoreBackend {
    Local(PathBuf),
    ObjectStore(Box<dyn ObjectStore>),
    Prefix(Box<dyn ObjectStore>, String),
}

impl CloudProofStore {
    pub fn from_config(config: &CloudStoreConfig) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        match config {
            CloudStoreConfig::Local { path } => {
                std::fs::create_dir_all(path)?;
                Ok(Self {
                    backend: StoreBackend::Local(PathBuf::from(path)),
                })
            }
            CloudStoreConfig::S3 { bucket, prefix, region } => {
                #[cfg(feature = "aws")]
                {
                    let url = format!("s3://{bucket}");
                    let mut builder = object_store::aws::AmazonS3Builder::from_env().with_bucket_name(bucket);
                    if let Some(r) = region {
                        builder = builder.with_region(r);
                    }
                    let store = builder.build()?;
                    Ok(Self {
                        backend: StoreBackend::Prefix(Box::new(store), prefix.trim_matches('/').to_string()),
                    })
                }
                #[cfg(not(feature = "aws"))]
                {
                    let _ = (bucket, prefix, region);
                    Err("s3 proof store requires veridata-cloud/aws feature".into())
                }
            }
            CloudStoreConfig::Gcs { bucket, prefix } => {
                #[cfg(feature = "gcp")]
                {
                    let store = object_store::gcp::GoogleCloudStorageBuilder::from_env()
                        .with_bucket_name(bucket)
                        .build()?;
                    Ok(Self {
                        backend: StoreBackend::Prefix(Box::new(store), prefix.trim_matches('/').to_string()),
                    })
                }
                #[cfg(not(feature = "gcp"))]
                {
                    let _ = (bucket, prefix);
                    Err("gcs proof store requires veridata-cloud/gcp feature".into())
                }
            }
            CloudStoreConfig::Adls { account, container, prefix } => {
                #[cfg(feature = "azure")]
                {
                    let store = object_store::azure::MicrosoftAzureBuilder::from_env()
                        .with_account(account)
                        .with_container_name(container)
                        .build()?;
                    Ok(Self {
                        backend: StoreBackend::Prefix(Box::new(store), prefix.trim_matches('/').to_string()),
                    })
                }
                #[cfg(not(feature = "azure"))]
                {
                    let _ = (account, container, prefix);
                    Err("adls proof store requires veridata-cloud/azure feature".into())
                }
            }
        }
    }

    pub fn save(&self, doc: &VrpDocument) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        let name = format!("{}.vrp.json", doc.proof_id);
        let json = serde_json::to_string_pretty(doc)?;
        match &self.backend {
            StoreBackend::Local(dir) => {
                let path = dir.join(&name);
                std::fs::write(&path, json)?;
                Ok(path)
            }
            StoreBackend::ObjectStore(store) | StoreBackend::Prefix(store, _) => {
                let path = self.object_path(&name)?;
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(async {
                    store
                        .put(&path, PutPayload::from(json.into_bytes()))
                        .await
                        .map_err(|e| format!("object store put: {e}"))?;
                    Ok::<_, Box<dyn std::error::Error + Send + Sync>>(PathBuf::from(path.to_string()))
                })
            }
        }
    }

    pub fn resolve(&self, selector: &str) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        if selector == "latest" {
            return self.latest();
        }
        let path = PathBuf::from(selector);
        if path.exists() {
            return Ok(path);
        }
        match &self.backend {
            StoreBackend::Local(dir) => {
                let in_store = dir.join(format!("{selector}.vrp.json"));
                if in_store.exists() {
                    return Ok(in_store);
                }
            }
            StoreBackend::ObjectStore(store) | StoreBackend::Prefix(store, _) => {
                let obj = self.object_path(&format!("{selector}.vrp.json"))?;
                let rt = tokio::runtime::Runtime::new()?;
                if rt.block_on(store.head(&obj)).is_ok() {
                    return Ok(PathBuf::from(obj.to_string()));
                }
            }
        }
        Err(format!("proof not found: {selector}").into())
    }

    pub fn latest(&self) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
        match &self.backend {
            StoreBackend::Local(dir) => local_latest(dir),
            StoreBackend::ObjectStore(store) | StoreBackend::Prefix(store, _) => {
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(async {
                    let prefix = match &self.backend {
                        StoreBackend::Prefix(_, p) if !p.is_empty() => {
                            ObjectPath::from(format!("{p}/"))
                        }
                        _ => ObjectPath::from(""),
                    };
                    let mut stream = store.list(Some(&prefix));
                    let mut latest: Option<(object_store::path::Path, u64)> = None;
                    while let Some(meta) = stream.next().await {
                        let meta = meta?;
                        if meta.location.as_ref().ends_with(".vrp.json") {
                            let ts = meta.last_modified.timestamp() as u64;
                            if latest.as_ref().map(|(_, t)| ts > *t).unwrap_or(true) {
                                latest = Some((meta.location, ts));
                            }
                        }
                    }
                    latest
                        .map(|(p, _)| PathBuf::from(p.to_string()))
                        .ok_or_else(|| "no proofs in object store".into())
                })
            }
        }
    }

    pub fn load(&self, path: &Path) -> Result<VrpDocument, Box<dyn std::error::Error + Send + Sync>> {
        if path.exists() {
            let text = std::fs::read_to_string(path)?;
            return Ok(serde_json::from_str(&text)?);
        }
        match &self.backend {
            StoreBackend::Local(_) => Err(format!("proof not found: {}", path.display()).into()),
            StoreBackend::ObjectStore(store) | StoreBackend::Prefix(store, _) => {
                let obj = ObjectPath::from(path.to_string_lossy().as_ref());
                let rt = tokio::runtime::Runtime::new()?;
                rt.block_on(async {
                    let data = store.get(&obj).await?.bytes().await?;
                    Ok(serde_json::from_slice(&data)?)
                })
            }
        }
    }

    fn object_path(&self, name: &str) -> Result<ObjectPath, Box<dyn std::error::Error + Send + Sync>> {
        match &self.backend {
            StoreBackend::ObjectStore(_) => Ok(ObjectPath::from(name)),
            StoreBackend::Prefix(_, prefix) => {
                if prefix.is_empty() {
                    Ok(ObjectPath::from(name))
                } else {
                    Ok(ObjectPath::from(format!("{prefix}/{name}")))
                }
            }
            StoreBackend::Local(_) => Err("not an object store".into()),
        }
    }
}

fn local_latest(dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error + Send + Sync>> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "json").unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
    entries
        .pop()
        .map(|e| e.path())
        .ok_or_else(|| "no proofs in store".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_store_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let store = CloudProofStore::from_config(&CloudStoreConfig::Local {
            path: dir.path().to_string_lossy().into(),
        })
        .unwrap();
        let doc = serde_json::from_str::<VrpDocument>(include_str!(
            "../../conformance/valid.vrp.json"
        ))
        .unwrap();
        let path = store.save(&doc).unwrap();
        let loaded = store.load(&path).unwrap();
        assert_eq!(loaded.proof_id, doc.proof_id);
    }
}
