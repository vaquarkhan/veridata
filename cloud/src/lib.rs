//! P4 cloud integrations: KMS signers (AWS/GCP/Azure) and object-store proof backends.

pub mod config;
pub mod kms;
pub mod store;

pub use config::{CloudStoreConfig, KmsProviderConfig};
pub use kms::build_signer;
pub use store::CloudProofStore;
