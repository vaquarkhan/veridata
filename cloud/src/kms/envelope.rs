use veridata_proof::Signer;

/// Append audit-friendly KMS metadata to the VRP producer string.
pub fn producer_with_kms(base: &str, signer: &dyn Signer) -> String {
    match signer.key_id() {
        Some(kid) => format!("{base} kms={}/{}", signer.kms_provider(), kid),
        None => base.to_string(),
    }
}
