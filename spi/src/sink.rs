use veridata_core::model::{Boundary, Fingerprint, Policy};

use crate::ConnectorResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushdownMode {
    /// Read rows in connector and hash locally (fallback).
    ClientSide,
    /// Compute hashes at sink; only fingerprints cross the boundary (AC-D4.1).
    Pushdown,
}

#[derive(Debug, Clone)]
pub struct SinkFingerprintBatch {
    pub fingerprints: Vec<Fingerprint>,
    pub pushdown_used: bool,
}

/// Sink connector: produces fingerprints for records materialized in the sink.
pub trait SinkConnector: Send + Sync {
    fn sink_ref(&self) -> &str;

    fn fingerprint_boundary(
        &self,
        boundary: &Boundary,
        policy: &Policy,
        salt: &[u8],
        content_fields: &[String],
        mode: PushdownMode,
    ) -> ConnectorResult<SinkFingerprintBatch>;

    fn schema_snapshot(&self) -> ConnectorResult<crate::SchemaSnapshot> {
        Ok(crate::SchemaSnapshot { fields: vec![] })
    }
}
