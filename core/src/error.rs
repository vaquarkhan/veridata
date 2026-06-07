use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum CoreError {
    #[error("missing identity field: {0}")]
    MissingIdentityField(String),
    #[error("invalid identity rule: {0}")]
    InvalidIdentityRule(String),
    #[error("unsupported canon version: {0}")]
    UnsupportedCanonVersion(u32),
    #[error("unknown hash algorithm: {0}")]
    UnknownHashAlgorithm(String),
    #[error("merkle leaf not found")]
    MerkleLeafNotFound,
    #[error("invalid decimal: {0}")]
    InvalidDecimal(String),
    #[error("{0}")]
    Other(String),
}

pub type CoreResult<T> = Result<T, CoreError>;
