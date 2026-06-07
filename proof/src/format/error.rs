use thiserror::Error;
use veridata_core::CoreError;

#[derive(Debug, Error)]
pub enum VrpError {
    #[error(transparent)]
    Core(#[from] CoreError),
    #[error("invalid VRP: {0}")]
    Invalid(String),
    #[error("verification failed: {0}")]
    VerifyFailed(String),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("sign error: {0}")]
    Sign(String),
}

pub type VrpResult<T> = Result<T, VrpError>;
