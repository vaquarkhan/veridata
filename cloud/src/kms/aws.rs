use aws_sdk_kms::primitives::Blob;
use aws_sdk_kms::types::{MessageType, SigningAlgorithmSpec};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use veridata_proof::{Signer, VrpError};

pub struct AwsKmsSigner {
    client: aws_sdk_kms::Client,
    key_id: String,
    public_key_b64: String,
}

impl AwsKmsSigner {
    pub fn new(key_id: impl Into<String>, region: Option<&str>) -> Result<Self, VrpError> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| VrpError::Invalid(e.to_string()))?;
        rt.block_on(async {
            let mut loader = aws_config::defaults(aws_config::BehaviorVersion::latest());
            if let Some(r) = region {
                loader = loader.region(aws_config::Region::new(r.to_string()));
            }
            let config = loader.load().await;
            let client = aws_sdk_kms::Client::new(&config);
            let pk = client
                .get_public_key()
                .key_id(key_id.into())
                .send()
                .await
                .map_err(|e| VrpError::Invalid(format!("aws kms get_public_key: {e}")))?;
            let bytes = pk
                .public_key()
                .ok_or_else(|| VrpError::Invalid("aws kms: no public key".into()))?
                .as_ref()
                .to_vec();
            if bytes.len() != 32 {
                return Err(VrpError::Invalid(format!(
                    "aws kms: expected 32-byte Ed25519 public key, got {}",
                    bytes.len()
                )));
            }
            Ok(Self {
                client,
                key_id: pk.key_id().unwrap_or_default().to_string(),
                public_key_b64: B64.encode(bytes),
            })
        })
    }
}

impl Signer for AwsKmsSigner {
    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, VrpError> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| VrpError::Invalid(e.to_string()))?;
        rt.block_on(async {
            let out = self
                .client
                .sign()
                .key_id(&self.key_id)
                .message(Blob::new(payload))
                .message_type(MessageType::Raw)
                .signing_algorithm(SigningAlgorithmSpec::EddsaPure)
                .send()
                .await
                .map_err(|e| VrpError::Invalid(format!("aws kms sign: {e}")))?;
            Ok(out
                .signature()
                .ok_or_else(|| VrpError::Invalid("aws kms: empty signature".into()))?
                .as_ref()
                .to_vec())
        })
    }

    fn public_key_b64(&self) -> String {
        self.public_key_b64.clone()
    }

    fn key_id(&self) -> Option<&str> {
        Some(&self.key_id)
    }

    fn kms_provider(&self) -> &'static str {
        "aws"
    }
}
