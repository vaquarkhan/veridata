use ed25519_dalek::{Signer as DalekSigner, SigningKey, VerifyingKey};
use base64::{engine::general_purpose::STANDARD as B64, Engine};

use super::VrpError;

pub trait Signer: Send + Sync {
    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, VrpError>;
    fn public_key_b64(&self) -> String;
}

pub struct KeyPair {
    signing: SigningKey,
    verifying: VerifyingKey,
}

impl KeyPair {
    /// Deterministic P0/P1 test key — never use in production.
    pub fn test_key() -> Self {
        Self::from_seed([0u8; 32])
    }

    pub fn from_seed(seed: [u8; 32]) -> Self {
        let signing = SigningKey::from_bytes(&seed);
        let verifying = signing.verifying_key();
        Self { signing, verifying }
    }

    /// Generate a random production key pair.
    pub fn generate() -> Self {
        let mut seed = [0u8; 32];
        getrandom::fill(&mut seed).expect("OS RNG");
        Self::from_seed(seed)
    }

    pub fn from_private_key_b64(b64: &str) -> Result<Self, VrpError> {
        let bytes = B64
            .decode(b64.trim())
            .map_err(|e| VrpError::Invalid(e.to_string()))?;
        if bytes.len() != 32 {
            return Err(VrpError::Invalid("private key must be 32 bytes".into()));
        }
        let mut seed = [0u8; 32];
        seed.copy_from_slice(&bytes);
        Ok(Self::from_seed(seed))
    }

    pub fn private_key_b64(&self) -> String {
        B64.encode(self.signing.to_bytes())
    }

    pub fn verifying_key(&self) -> &VerifyingKey {
        &self.verifying
    }
}

impl Signer for KeyPair {
    fn sign(&self, payload: &[u8]) -> Result<Vec<u8>, VrpError> {
        Ok(self.signing.sign(payload).to_vec())
    }

    fn public_key_b64(&self) -> String {
        B64.encode(self.verifying.as_bytes())
    }
}
