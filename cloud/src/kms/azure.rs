use azure_identity::DefaultAzureCredential;
use azure_security_keyvault_keys::KeyClient;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use veridata_proof::{Signer, VrpError};

pub struct AzureKmsSigner {
    client: KeyClient,
    key_name: String,
    public_key_b64: String,
}

impl AzureKmsSigner {
    pub fn new(key_name: impl Into<String>, vault_url: Option<&str>) -> Result<Self, VrpError> {
        let key_name = key_name.into();
        let vault = vault_url
            .map(|s| s.to_string())
            .or_else(|| std::env::var("AZURE_KEY_VAULT_URL").ok())
            .ok_or_else(|| {
                VrpError::Invalid(
                    "azure kms: set crypto.azure_vault_url or AZURE_KEY_VAULT_URL".into(),
                )
            })?;
        let rt = tokio::runtime::Runtime::new().map_err(|e| VrpError::Invalid(e.to_string()))?;
        rt.block_on(async {
            let cred = DefaultAzureCredential::default();
            let client = KeyClient::new(&vault, cred.clone(), None)
                .map_err(|e| VrpError::Invalid(format!("azure key vault client: {e}")))?;
            let key = client
                .get_key(&key_name, None)
                .await
                .map_err(|e| VrpError::Invalid(format!("azure get_key: {e}")))?;
            let jwk = key
                .key
                .jwk
                .ok_or_else(|| VrpError::Invalid("azure key: missing jwk".into()))?;
            let x = jwk
                .x
                .as_ref()
                .ok_or_else(|| VrpError::Invalid("azure key: missing jwk.x".into()))?;
            let bytes = B64
                .decode(x)
                .map_err(|e| VrpError::Invalid(format!("azure jwk.x decode: {e}")))?;
            if bytes.len() != 32 {
                return Err(VrpError::Invalid(format!(
                    "azure key: expected 32-byte Ed25519 public key, got {}",
                    bytes.len()
                )));
            }
            Ok(Self {
                client,
                key_name,
                public_key_b64: B64.encode(bytes),
            })
        })
    }
}

impl Signer for AzureKmsSigner {
    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, VrpError> {
        let rt = tokio::runtime::Runtime::new().map_err(|e| VrpError::Invalid(e.to_string()))?;
        rt.block_on(async {
            use azure_security_keyvault_keys::models::KeySignParameters;
            use azure_security_keyvault_keys::models::SignatureAlgorithm;
            let digest = sha256(payload);
            let params = KeySignParameters {
                algorithm: SignatureAlgorithm::Ed25519,
                value: Some(B64.encode(&digest)),
                ..Default::default()
            };
            let resp = self
                .client
                .sign(&self.key_name, params, None)
                .await
                .map_err(|e| VrpError::Invalid(format!("azure sign: {e}")))?;
            let sig = resp
                .result
                .ok_or_else(|| VrpError::Invalid("azure sign: empty result".into()))?;
            B64.decode(sig.trim())
                .map_err(|e| VrpError::Invalid(format!("azure signature b64: {e}")))
        })
    }

    fn public_key_b64(&self) -> String {
        self.public_key_b64.clone()
    }

    fn key_id(&self) -> Option<&str> {
        Some(&self.key_name)
    }

    fn kms_provider(&self) -> &'static str {
        "azure"
    }
}

fn sha256(data: &[u8]) -> Vec<u8> {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(data);
    h.finalize().to_vec()
}
