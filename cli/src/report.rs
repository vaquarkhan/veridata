use veridata_proof::{verify_file, VerifyOutcome, VrpDocument};

pub fn print_report(doc: &VrpDocument, verify: VerifyOutcome) {
    println!("VRP Report");
    println!("==========");
    println!("proof_id:    {}", doc.proof_id);
    println!("created_at:  {}", doc.created_at);
    println!("producer:    {}", doc.producer);
    println!("verdict:     {}", doc.reconciliation.verdict);
    println!("verify:      {:?}", verify);
    println!();
    println!("Source: {} (count={})", doc.source_ref, doc.source_commitment.count);
    println!("Sink:   {} (count={})", doc.sink_ref, doc.sink_commitment.count);
    println!();
    println!("Evidence:");
    println!("  matched:    {}", doc.reconciliation.matched.count);
    println!("  missing:    {}", doc.reconciliation.missing.len());
    println!("  duplicated: {}", doc.reconciliation.duplicated.len());
    println!("  mutated:    {}", doc.reconciliation.mutated.len());
    if let Some(reason) = &doc.reconciliation.unverified_reason {
        println!("  unverified: {reason}");
    }
}

pub fn verify_proof(path: &std::path::Path, pubkey_b64: &str) -> anyhow::Result<VerifyOutcome> {
    verify_file(path, pubkey_b64).map_err(|e| anyhow::anyhow!("{e}"))
}
