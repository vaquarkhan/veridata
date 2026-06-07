use crate::format::{parse_hash32, VrpDocument};

/// Multiset balance and merkle-root consistency (spec §9).
pub fn verify_commitment_structure(doc: &VrpDocument) -> bool {
    let r = &doc.reconciliation;
    let src = doc.source_commitment.count;
    let snk = doc.sink_commitment.count;
    let matched = r.matched.count;
    let missing = r.missing.len() as u64;
    let mutated = r.mutated.len() as u64;

    if matched + missing + mutated != src {
        return false;
    }

    let dup_excess: u64 = r
        .duplicated
        .iter()
        .map(|d| d.sink_multiplicity.saturating_sub(d.source_multiplicity))
        .sum();
    if matched + mutated + dup_excess != snk {
        return false;
    }

    let sink_root = match parse_hash32(&doc.sink_commitment.merkle_root) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let matched_root = match parse_hash32(&r.matched.merkle_root) {
        Ok(v) => v,
        Err(_) => return false,
    };

    if mutated == 0 && dup_excess == 0 && matched == snk && sink_root != matched_root {
        return false;
    }

    if r.verdict == "PASS"
        && missing == 0
        && mutated == 0
        && r.duplicated.is_empty()
        && (doc.source_commitment.merkle_root != doc.sink_commitment.merkle_root
            || doc.source_commitment.merkle_root != r.matched.merkle_root)
    {
        return false;
    }

    true
}
