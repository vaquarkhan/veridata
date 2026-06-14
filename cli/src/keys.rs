use std::path::Path;

use veridata_cloud::config::KmsProviderConfig;
use veridata_cloud::kms::build_signer;
use veridata_proof::Signer;

use crate::config::{CryptoConfig, KmsProvider};

pub fn load_signer(crypto: &CryptoConfig) -> anyhow::Result<Box<dyn Signer + Send + Sync>> {
    let provider = match crypto.kms_provider {
        KmsProvider::File => KmsProviderConfig::File,
        KmsProvider::Aws => KmsProviderConfig::Aws,
        KmsProvider::Gcp => KmsProviderConfig::Gcp,
        KmsProvider::Azure => KmsProviderConfig::Azure,
    };
    build_signer(
        &provider,
        crypto.kms_key_id.as_deref(),
        &crypto.private_key_file,
        crypto.aws_region.as_deref(),
    )
    .map_err(|e| anyhow::anyhow!("{e}"))
}

pub fn load_pubkey_b64(path: &Path) -> anyhow::Result<String> {
    Ok(std::fs::read_to_string(path)?.trim().to_string())
}

pub fn write_keypair(dir: &Path, pair: &veridata_proof::KeyPair) -> anyhow::Result<()> {
    std::fs::create_dir_all(dir)?;
    std::fs::write(
        dir.join("signing.key.b64"),
        format!("{}\n", pair.private_key_b64()),
    )?;
    std::fs::write(
        dir.join("signing.pub.b64"),
        format!("{}\n", pair.public_key_b64()),
    )?;
    Ok(())
}
