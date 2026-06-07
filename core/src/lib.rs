//! Pure reconciliation engine — no I/O, no network, no connectors.

pub mod canon;
pub mod error;
pub mod hash;
pub mod identity;
pub mod model;
pub mod policy_util;
pub mod recon;
pub mod salt;

#[cfg(any(test, feature = "test-util"))]
pub mod testutil;

pub use canon::{canon_content, canon_identity, CanonValue, Record};
pub use error::{CoreError, CoreResult};
pub use hash::{
    commutative_root, fingerprint_hashes, merkle_leaf, merkle_proof, merkle_root,
    verify_merkle_proof, verify_merkle_proof_with_index, HashAlgorithm, Hasher,
};
pub use identity::{identity_fields, IdentityRule};
pub use model::*;
pub use policy_util::{effective_content_fields, late_arrival_window_secs};
pub use recon::{derive_verdict, reconcile, ReconcileOutput};
pub use salt::generate_proof_salt;
