//! Per-proof salt generation (AC-A5.2).

/// Generate 32 cryptographically random bytes for a single proof.
pub fn generate_proof_salt() -> [u8; 32] {
    let mut salt = [0u8; 32];
    getrandom::fill(&mut salt).expect("OS RNG");
    salt
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac_a5_2_different_proofs_get_different_salts() {
        let a = generate_proof_salt();
        let b = generate_proof_salt();
        assert_ne!(a, b);
    }
}
