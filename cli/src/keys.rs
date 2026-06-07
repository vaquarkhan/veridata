use std::fs;
use std::path::Path;

use veridata_proof::sign::KeyPair;
use veridata_proof::{FileKmsSigner, Signer};

use crate::config::CryptoConfig;

pub fn write_keypair(dir: &Path, pair: &KeyPair) -> anyhow::Result<()> {
    fs::create_dir_all(dir)?;
    fs::write(
        dir.join("signing.key.b64"),
        format!("{}\n", pair.private_key_b64()),
    )?;
    fs::write(
        dir.join("signing.pub.b64"),
        format!("{}\n", pair.public_key_b64()),
    )?;
    Ok(())
}

pub fn load_signer_file(path: &Path) -> anyhow::Result<KeyPair> {
    let b64 = fs::read_to_string(path)?;
    KeyPair::from_private_key_b64(&b64).map_err(|e| anyhow::anyhow!("{e}"))
}

pub fn load_signer(crypto: &CryptoConfig) -> anyhow::Result<Box<dyn Signer + Send + Sync>> {
    if let Some(kid) = &crypto.kms_key_id {
        Ok(Box::new(FileKmsSigner::from_key_file(
            kid,
            &crypto.private_key_file,
        )?))
    } else {
        Ok(Box::new(load_signer_file(&crypto.private_key_file)?))
    }
}

pub fn load_pubkey_b64(path: &Path) -> anyhow::Result<String> {
    Ok(fs::read_to_string(path)?.trim().to_string())
}
