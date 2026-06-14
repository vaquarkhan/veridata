use base64::{engine::general_purpose::STANDARD as B64, Engine};
use reqwest::header::{AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
use veridata_proof::{Signer, VrpError};

/// GCP Cloud KMS Ed25519 signer via REST (asymmetricSign).
pub struct GcpKmsSigner {
    http: reqwest::Client,
    resource_name: String,
    public_key_b64: String,
}

#[derive(Serialize)]
struct SignRequest<'a> {
    #[serde(rename = "digest")]
    digest: Digest<'a>,
}

#[derive(Serialize)]
struct Digest<'a> {
    #[serde(rename = "sha256")]
    sha256: &'a str,
}

#[derive(Deserialize)]
struct SignResponse {
    signature: String,
}

#[derive(Deserialize)]
struct PublicKeyResponse {
    pem: String,
}

impl GcpKmsSigner {
    pub fn new(resource_name: impl Into<String>) -> Result<Self, VrpError> {
        let resource_name = resource_name.into();
        let token = gcp_access_token()?;
        let http = reqwest::Client::new();
        let pk_url = format!(
            "https://cloudkms.googleapis.com/v1/{}:getPublicKey",
            resource_name
        );
        let rt = tokio::runtime::Runtime::new().map_err(|e| VrpError::Invalid(e.to_string()))?;
        let pem: String = rt.block_on(async {
            let resp = http
                .get(&pk_url)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .send()
                .await
                .map_err(|e| VrpError::Invalid(format!("gcp kms getPublicKey: {e}")))?;
            let body: PublicKeyResponse = resp
                .json()
                .await
                .map_err(|e| VrpError::Invalid(format!("gcp kms getPublicKey parse: {e}")))?;
            Ok::<_, VrpError>(body.pem)
        })?;
        let public_key_b64 = pem_ed25519_to_b64(&pem)?;
        Ok(Self {
            http,
            resource_name,
            public_key_b64,
        })
    }

    fn access_token(&self) -> Result<String, VrpError> {
        gcp_access_token()
    }
}

impl Signer for GcpKmsSigner {
    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, VrpError> {
        let digest = sha256_hex(payload);
        let url = format!(
            "https://cloudkms.googleapis.com/v1/{}:asymmetricSign",
            self.resource_name
        );
        let token = self.access_token()?;
        let body = SignRequest {
            digest: Digest { sha256: &digest },
        };
        let rt = tokio::runtime::Runtime::new().map_err(|e| VrpError::Invalid(e.to_string()))?;
        rt.block_on(async {
            let resp = self
                .http
                .post(&url)
                .header(AUTHORIZATION, format!("Bearer {token}"))
                .header(CONTENT_TYPE, "application/json")
                .json(&body)
                .send()
                .await
                .map_err(|e| VrpError::Invalid(format!("gcp kms sign: {e}")))?;
            let out: SignResponse = resp
                .json()
                .await
                .map_err(|e| VrpError::Invalid(format!("gcp kms sign parse: {e}")))?;
            B64.decode(out.signature.trim())
                .map_err(|e| VrpError::Invalid(format!("gcp kms signature b64: {e}")))
        })
    }

    fn public_key_b64(&self) -> String {
        self.public_key_b64.clone()
    }

    fn key_id(&self) -> Option<&str> {
        Some(&self.resource_name)
    }

    fn kms_provider(&self) -> &'static str {
        "gcp"
    }
}

fn gcp_access_token() -> Result<String, VrpError> {
    std::env::var("GOOGLE_APPLICATION_CREDENTIALS").map_err(|_| {
        VrpError::Invalid(
            "gcp kms: set GOOGLE_APPLICATION_CREDENTIALS to a service account JSON path".into(),
        )
    })?;
    let rt = tokio::runtime::Runtime::new().map_err(|e| VrpError::Invalid(e.to_string()))?;
    rt.block_on(async {
        let out = std::process::Command::new("gcloud")
            .args(["auth", "application-default", "print-access-token"])
            .output()
            .map_err(|e| VrpError::Invalid(format!("gcp auth: {e}")))?;
        if !out.status.success() {
            return Err(VrpError::Invalid(
                "gcp kms: run `gcloud auth application-default login` or set workload identity"
                    .into(),
            ));
        }
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    })
}

fn sha256_hex(data: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(data);
    hex::encode(h.finalize())
}

fn pem_ed25519_to_b64(pem: &str) -> Result<String, VrpError> {
    let lines: String = pem
        .lines()
        .filter(|l| !l.starts_with("-----"))
        .collect();
    let der = B64
        .decode(lines)
        .map_err(|e| VrpError::Invalid(format!("gcp pem decode: {e}")))?;
    let raw = der
        .get(der.len().saturating_sub(32)..)
        .ok_or_else(|| VrpError::Invalid("gcp pem: short der".into()))?;
    if raw.len() != 32 {
        return Err(VrpError::Invalid("gcp pem: expected Ed25519 raw key".into()));
    }
    Ok(B64.encode(raw))
}
