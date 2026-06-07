use crate::error::{CoreError, CoreResult};
use crate::model::Hash32;

pub const TAG_MERKLE_LEAF: u8 = 0x00;
pub const TAG_ID_HASH: u8 = 0x01;
pub const TAG_CONTENT_HASH: u8 = 0x02;
pub const TAG_FINGERPRINT: u8 = 0x03;
pub const TAG_MERKLE_NODE: u8 = 0x10;
pub const TAG_COMMUTATIVE: u8 = 0x20;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HashAlgorithm {
    Sha256,
    Blake3,
}

impl HashAlgorithm {
    pub fn parse(s: &str) -> CoreResult<Self> {
        match s {
            "sha256" => Ok(Self::Sha256),
            "blake3" => Ok(Self::Blake3),
            other => Err(CoreError::UnknownHashAlgorithm(other.to_string())),
        }
    }
}

pub trait Hasher {
    fn hash(&self, data: &[u8]) -> Hash32;
    fn algorithm(&self) -> HashAlgorithm;
}

#[derive(Debug, Clone, Copy)]
pub struct Sha256Hasher;

impl Hasher for Sha256Hasher {
    fn hash(&self, data: &[u8]) -> Hash32 {
        use sha2::{Digest, Sha256};
        Sha256::digest(data).into()
    }

    fn algorithm(&self) -> HashAlgorithm {
        HashAlgorithm::Sha256
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Blake3Hasher;

impl Hasher for Blake3Hasher {
    fn hash(&self, data: &[u8]) -> Hash32 {
        blake3::hash(data).into()
    }

    fn algorithm(&self) -> HashAlgorithm {
        HashAlgorithm::Blake3
    }
}

pub fn hasher_for(alg: &str) -> CoreResult<Box<dyn Hasher>> {
    match HashAlgorithm::parse(alg)? {
        HashAlgorithm::Sha256 => Ok(Box::new(Sha256Hasher)),
        HashAlgorithm::Blake3 => Ok(Box::new(Blake3Hasher)),
    }
}

pub fn fingerprint_hashes(
    hasher: &dyn Hasher,
    salt: &[u8],
    id_canon: &[u8],
    content_canon: &[u8],
) -> (Hash32, Hash32, Hash32) {
    let mut id_input = Vec::with_capacity(salt.len() + 1 + id_canon.len());
    id_input.extend_from_slice(salt);
    id_input.push(TAG_ID_HASH);
    id_input.extend_from_slice(id_canon);
    let id_hash = hasher.hash(&id_input);

    let mut content_input = Vec::with_capacity(salt.len() + 1 + content_canon.len());
    content_input.extend_from_slice(salt);
    content_input.push(TAG_CONTENT_HASH);
    content_input.extend_from_slice(content_canon);
    let content_hash = hasher.hash(&content_input);

    let mut fp_input = Vec::with_capacity(1 + 64);
    fp_input.push(TAG_FINGERPRINT);
    fp_input.extend_from_slice(&id_hash);
    fp_input.extend_from_slice(&content_hash);
    let fp = hasher.hash(&fp_input);

    (id_hash, content_hash, fp)
}

pub fn merkle_leaf(hasher: &dyn Hasher, fp: &Hash32) -> Hash32 {
    let mut input = Vec::with_capacity(1 + 32);
    input.push(TAG_MERKLE_LEAF);
    input.extend_from_slice(fp);
    hasher.hash(&input)
}

pub fn merkle_root(hasher: &dyn Hasher, mut leaves: Vec<Hash32>) -> Hash32 {
    if leaves.is_empty() {
        let mut input = vec![TAG_MERKLE_LEAF];
        input.extend_from_slice(&[0u8; 32]);
        return hasher.hash(&input);
    }
    leaves.sort();
    let mut layer = leaves;
    while layer.len() > 1 {
        let mut next = Vec::new();
        for chunk in layer.chunks(2) {
            let left = chunk[0];
            let right = if chunk.len() == 2 { chunk[1] } else { left };
            next.push(merkle_node(hasher, &left, &right));
        }
        layer = next;
    }
    layer[0]
}

fn merkle_node(hasher: &dyn Hasher, left: &Hash32, right: &Hash32) -> Hash32 {
    let mut input = Vec::with_capacity(1 + 64);
    input.push(TAG_MERKLE_NODE);
    input.extend_from_slice(left);
    input.extend_from_slice(right);
    hasher.hash(&input)
}

/// Inclusion proof: sibling hashes from leaf to root (sorted-leaf tree).
pub fn merkle_proof(
    hasher: &dyn Hasher,
    sorted_leaves: &[Hash32],
    target: &Hash32,
) -> CoreResult<Vec<Hash32>> {
    let idx = sorted_leaves
        .iter()
        .position(|l| l == target)
        .ok_or(CoreError::MerkleLeafNotFound)?;

    let mut layer: Vec<Hash32> = sorted_leaves.to_vec();
    let mut index = idx;
    let mut proof = Vec::new();

    while layer.len() > 1 {
        if layer.len() % 2 == 1 {
            layer.push(*layer.last().unwrap());
        }
        let sibling_idx = if index % 2 == 1 {
            index - 1
        } else {
            index + 1
        };
        proof.push(layer[sibling_idx]);

        let mut next = Vec::new();
        for chunk in layer.chunks(2) {
            next.push(merkle_node(hasher, &chunk[0], &chunk[1]));
        }
        index /= 2;
        layer = next;
    }
    Ok(proof)
}

/// Order-independent aggregate over fingerprint hashes (AC-B6).
pub fn commutative_root(hasher: &dyn Hasher, mut fps: Vec<Hash32>) -> Hash32 {
    fps.sort();
    let mut acc = [0u8; 32];
    for fp in fps {
        for (i, b) in fp.iter().enumerate() {
            acc[i] ^= b;
        }
    }
    let mut input = Vec::with_capacity(1 + 32);
    input.push(TAG_COMMUTATIVE);
    input.extend_from_slice(&acc);
    hasher.hash(&input)
}

/// Verify inclusion proof when the sorted leaf index is unknown (try 0..leaf_count).
pub fn verify_merkle_proof(
    hasher: &dyn Hasher,
    root: &Hash32,
    leaf: &Hash32,
    proof: &[Hash32],
    leaf_count: usize,
) -> bool {
    if leaf_count == 0 {
        return false;
    }
    (0..leaf_count).any(|idx| verify_merkle_proof_with_index(hasher, root, leaf, proof, idx))
}

pub fn verify_merkle_proof_with_index(
    hasher: &dyn Hasher,
    root: &Hash32,
    leaf: &Hash32,
    proof: &[Hash32],
    mut index: usize,
) -> bool {
    let mut current = *leaf;
    for sibling in proof {
        let (left, right) = if index % 2 == 0 {
            (current, *sibling)
        } else {
            (*sibling, current)
        };
        current = merkle_node(hasher, &left, &right);
        index /= 2;
    }
    current == *root
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ac_a8_1_blake3_selectable() {
        let h = hasher_for("blake3").unwrap();
        assert_eq!(h.algorithm(), HashAlgorithm::Blake3);
        let digest = h.hash(b"test");
        assert_eq!(digest.len(), 32);
    }

    #[test]
    fn ac_a8_2_unknown_algo_fails() {
        assert!(matches!(
            hasher_for("md5"),
            Err(CoreError::UnknownHashAlgorithm(_))
        ));
    }

    #[test]
    fn ac_b5_1_equal_matched_roots() {
        let h = Sha256Hasher;
        let fp1 = h.hash(b"fp1");
        let fp2 = h.hash(b"fp2");
        let leaves = vec![merkle_leaf(&h, &fp1), merkle_leaf(&h, &fp2)];
        let r1 = merkle_root(&h, leaves.clone());
        let r2 = merkle_root(&h, leaves);
        assert_eq!(r1, r2);
    }

    #[test]
    fn ac_b5_2_inclusion_proof_verifies() {
        let h = Sha256Hasher;
        let fp = h.hash(b"target");
        let leaf = merkle_leaf(&h, &fp);
        let other = merkle_leaf(&h, &h.hash(b"other"));
        let mut leaves = vec![leaf, other];
        let root = merkle_root(&h, leaves.clone());
        leaves.sort();
        let idx = leaves.iter().position(|l| l == &leaf).unwrap();
        let proof = merkle_proof(&h, &leaves, &leaf).unwrap();
        assert!(verify_merkle_proof_with_index(&h, &root, &leaf, &proof, idx));
        assert!(verify_merkle_proof(&h, &root, &leaf, &proof, leaves.len()));
    }

    #[test]
    fn ac_b6_1_commutative_root_order_independent() {
        let h = Sha256Hasher;
        let fp1 = h.hash(b"a");
        let fp2 = h.hash(b"b");
        let fp3 = h.hash(b"c");
        let r1 = commutative_root(&h, vec![fp1, fp2, fp3]);
        let r2 = commutative_root(&h, vec![fp3, fp1, fp2]);
        assert_eq!(r1, r2);
    }

    #[test]
    fn ac_b5_3_tampered_path_fails() {
        let h = Sha256Hasher;
        let fp = h.hash(b"target");
        let leaf = merkle_leaf(&h, &fp);
        let other = merkle_leaf(&h, &h.hash(b"other"));
        let mut leaves = vec![leaf, other];
        let root = merkle_root(&h, leaves.clone());
        leaves.sort();
        let idx = leaves.iter().position(|l| l == &leaf).unwrap();
        let mut proof = merkle_proof(&h, &leaves, &leaf).unwrap();
        proof[0] = h.hash(b"tampered");
        assert!(!verify_merkle_proof_with_index(&h, &root, &leaf, &proof, idx));
        assert!(!verify_merkle_proof(&h, &root, &leaf, &proof, leaves.len()));
    }
}
