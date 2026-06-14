mod envelope;

#[cfg(feature = "aws")]
mod aws;
#[cfg(feature = "azure")]
mod azure;
#[cfg(feature = "gcp")]
mod gcp;

use std::path::Path;

use veridata_proof::{FileKmsSigner, KeyPair, Signer, VrpError};

use crate::config::KmsProviderConfig;

pub use envelope::producer_with_kms;

/// Build a signer from crypto config (file or cloud KMS).
pub fn build_signer(
    provider: &KmsProviderConfig,
    kms_key_id: Option<&str>,
    private_key_file: &Path,
    region: Option<&str>,
) -> Result<Box<dyn Signer + Send + Sync>, VrpError> {
    match provider {
        KmsProviderConfig::File => {
            if let Some(kid) = kms_key_id {
                Ok(Box::new(FileKmsSigner::from_key_file(kid, private_key_file)?))
            } else {
                let text = std::fs::read_to_string(private_key_file)
                    .map_err(|e| VrpError::Invalid(e.to_string()))?;
                Ok(Box::new(KeyPair::from_private_key_b64(&text)?))
            }
        }
        KmsProviderConfig::Aws => {
            let key_id = kms_key_id.ok_or_else(|| {
                VrpError::Invalid("aws kms requires crypto.kms_key_id (key ARN)".into())
            })?;
            #[cfg(feature = "aws")]
            {
                Ok(Box::new(aws::AwsKmsSigner::new(key_id, region)?))
            }
            #[cfg(not(feature = "aws"))]
            {
                let _ = (key_id, region);
                Err(VrpError::Invalid(
                    "veridata-cli built without aws feature; rebuild with --features cloud".into(),
                ))
            }
        }
        KmsProviderConfig::Gcp => {
            let key_id = kms_key_id.ok_or_else(|| {
                VrpError::Invalid("gcp kms requires crypto.kms_key_id (resource name)".into())
            })?;
            #[cfg(feature = "gcp")]
            {
                Ok(Box::new(gcp::GcpKmsSigner::new(key_id)?))
            }
            #[cfg(not(feature = "gcp"))]
            {
                let _ = key_id;
                Err(VrpError::Invalid(
                    "veridata-cli built without gcp feature; rebuild with --features cloud".into(),
                ))
            }
        }
        KmsProviderConfig::Azure => {
            let key_id = kms_key_id.ok_or_else(|| {
                VrpError::Invalid("azure key vault requires crypto.kms_key_id (key name)".into())
            })?;
            #[cfg(feature = "azure")]
            {
                Ok(Box::new(azure::AzureKmsSigner::new(key_id, region)?))
            }
            #[cfg(not(feature = "azure"))]
            {
                let _ = (key_id, region);
                Err(VrpError::Invalid(
                    "veridata-cli built without azure feature; rebuild with --features cloud".into(),
                ))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_signer_without_kms_id() {
        let dir = tempfile::tempdir().unwrap();
        let pair = KeyPair::test_key();
        let path = dir.path().join("key.b64");
        std::fs::write(&path, pair.private_key_b64()).unwrap();
        let signer = build_signer(&KmsProviderConfig::File, None, &path, None).unwrap();
        assert_eq!(signer.kms_provider(), "file");
    }
}
