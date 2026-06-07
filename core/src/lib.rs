//! Pure reconciliation engine — no I/O, no network, no connectors.

pub mod canon;
pub mod error;
pub mod hash;
pub mod identity;
pub mod model;
pub mod recon;

#[cfg(any(test, feature = "test-util"))]
pub mod testutil;

pub use canon::{canon_content, canon_identity, CanonValue, Record};
pub use error::{CoreError, CoreResult};
pub use hash::{fingerprint_hashes, merkle_leaf, merkle_proof, merkle_root, HashAlgorithm, Hasher};
pub use identity::{identity_fields, IdentityRule};
pub use model::*;
pub use recon::{derive_verdict, reconcile, ReconcileOutput};
