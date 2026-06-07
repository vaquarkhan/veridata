//! Key providers for AC-C7 (file-backed and multi-pubkey verification).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD as B64, Engine};
use ed25519_dalek::VerifyingKey;

use super::{KeyPair, Signer};
use crate::VrpError;

/// Sign with a local key file (development / on-prem stand-in for KMS).
pub struct FileKmsSigner {
    key_id: String,
    inner: KeyPair,
}

impl FileKmsSigner {
    pub fn from_key_file(key_id: impl Into<String>, path: &Path) -> Result<Self, VrpError> {
        let text = std::fs::read_to_string(path).map_err(|e| VrpError::Invalid(e.to_string()))?;
        Ok(Self {
            key_id: key_id.into(),
            inner: KeyPair::from_private_key_b64(&text)?,
        })
    }

    pub fn key_id(&self) -> &str {
        &self.key_id
    }
}

impl Signer for FileKmsSigner {
    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, VrpError> {
        self.inner.sign(payload)
    }

    fn public_key_b64(&self) -> String {
        self.inner.public_key_b64()
    }
}

/// Resolve trusted pubkeys by key id for rotation (AC-C7.3).
pub struct PubkeyDirectory {
    keys: HashMap<String, VerifyingKey>,
}

impl PubkeyDirectory {
    pub fn load_dir(dir: &Path) -> Result<Self, VrpError> {
        let mut keys = HashMap::new();
        if !dir.is_dir() {
            return Ok(Self { keys });
        }
        for entry in std::fs::read_dir(dir).map_err(|e| VrpError::Invalid(e.to_string()))? {
            let entry = entry.map_err(|e| VrpError::Invalid(e.to_string()))?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("b64") {
                continue;
            }
            let key_id = path
                .file_stem()
                .and_then(|s| s.to_str())
                .map(|s| s.strip_suffix(".pub").unwrap_or(s))
                .unwrap_or("default")
                .to_string();
            let text =
                std::fs::read_to_string(&path).map_err(|e| VrpError::Invalid(e.to_string()))?;
            let bytes = B64
                .decode(text.trim())
                .map_err(|e| VrpError::Invalid(e.to_string()))?;
            if bytes.len() != 32 {
                continue;
            }
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            if let Ok(pk) = VerifyingKey::from_bytes(&arr) {
                keys.insert(key_id, pk);
            }
        }
        Ok(Self { keys })
    }

    pub fn get(&self, key_id: &str) -> Option<&VerifyingKey> {
        self.keys.get(key_id)
    }

    pub fn resolve_path(&self, key_id: &str, fallback: &Path) -> PathBuf {
        self.keys
            .get(key_id)
            .map(|_| fallback.to_path_buf())
            .unwrap_or_else(|| fallback.to_path_buf())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn ac_c7_2_file_kms_signer_roundtrip() {
        let pair = KeyPair::test_key();
        let dir = tempdir().unwrap();
        let path = dir.path().join("kms.key.b64");
        std::fs::write(&path, pair.private_key_b64()).unwrap();
        let signer = FileKmsSigner::from_key_file("prod-1", &path).unwrap();
        let sig = signer.sign(b"payload").unwrap();
        assert_eq!(sig.len(), 64);
        assert_eq!(signer.key_id(), "prod-1");
    }

    #[test]
    fn ac_c7_3_pubkey_directory_loads_historical() {
        let pair = KeyPair::test_key();
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("2024-01.pub.b64"),
            pair.public_key_b64(),
        )
        .unwrap();
        let store = PubkeyDirectory::load_dir(dir.path()).unwrap();
        assert!(store.get("2024-01").is_some());
    }
}
