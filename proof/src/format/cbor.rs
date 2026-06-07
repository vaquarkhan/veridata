//! CBOR wire encoding (spec §10). Signatures still apply to JCS JSON bytes in v0.1.

use super::{VrpDocument, VrpError, VrpResult};

pub fn encode_cbor(doc: &VrpDocument) -> VrpResult<Vec<u8>> {
    let mut buf = Vec::new();
    ciborium::into_writer(doc, &mut buf).map_err(|e| VrpError::Invalid(e.to_string()))?;
    Ok(buf)
}

pub fn decode_cbor(bytes: &[u8]) -> VrpResult<VrpDocument> {
    ciborium::from_reader(bytes).map_err(|e| VrpError::Invalid(e.to_string()))
}

/// Logical round-trip: CBOR bytes decode to the same document as JSON.
pub fn roundtrip_equals_json(json_doc: &VrpDocument) -> VrpResult<bool> {
    let bytes = encode_cbor(json_doc)?;
    let decoded: VrpDocument = decode_cbor(&bytes)?;
    Ok(decoded == *json_doc)
}
