use veridata_core::model::{Boundary, Fingerprint, Policy};

use crate::schema::boundary_value;
use crate::ConnectorResult;

/// Source connector: produces fingerprints for records in boundary scope.
pub trait SourceConnector: Send + Sync {
    fn source_ref(&self) -> &str;

    fn fingerprint_boundary(
        &self,
        boundary: &Boundary,
        policy: &Policy,
        salt: &[u8],
        content_fields: &[String],
    ) -> ConnectorResult<Vec<Fingerprint>>;

    fn schema_snapshot(&self) -> ConnectorResult<crate::SchemaSnapshot> {
        Ok(crate::SchemaSnapshot { fields: vec![] })
    }
}

/// Helper for connectors parsing boundary bytes.
pub fn read_boundary_json(boundary: &Boundary) -> ConnectorResult<Vec<u8>> {
    Ok(boundary_value(boundary)?.to_vec())
}
