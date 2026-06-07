use std::fs;
use std::path::{Path, PathBuf};

use veridata_proof::VrpDocument;

pub struct ProofStore {
    dir: PathBuf,
}

impl ProofStore {
    pub fn new(dir: impl Into<PathBuf>) -> anyhow::Result<Self> {
        let dir = dir.into();
        fs::create_dir_all(&dir)?;
        Ok(Self { dir })
    }

    pub fn save(&self, doc: &VrpDocument) -> anyhow::Result<PathBuf> {
        let path = self.dir.join(format!("{}.vrp.json", doc.proof_id));
        let json = serde_json::to_string_pretty(doc)?;
        fs::write(&path, json)?;
        Ok(path)
    }

    pub fn resolve(&self, selector: &str) -> anyhow::Result<PathBuf> {
        if selector == "latest" {
            return self.latest();
        }
        let path = PathBuf::from(selector);
        if path.exists() {
            return Ok(path);
        }
        let in_store = self.dir.join(format!("{selector}.vrp.json"));
        if in_store.exists() {
            return Ok(in_store);
        }
        anyhow::bail!("proof not found: {selector}")
    }

    pub fn latest(&self) -> anyhow::Result<PathBuf> {
        let mut entries: Vec<_> = fs::read_dir(&self.dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|x| x == "json")
                    .unwrap_or(false)
            })
            .collect();
        entries.sort_by_key(|e| e.metadata().and_then(|m| m.modified()).ok());
        entries
            .pop()
            .map(|e| e.path())
            .ok_or_else(|| anyhow::anyhow!("no proofs in store"))
    }

    pub fn load(&self, path: &Path) -> anyhow::Result<VrpDocument> {
        let text = fs::read_to_string(path)?;
        Ok(serde_json::from_str(&text)?)
    }
}
