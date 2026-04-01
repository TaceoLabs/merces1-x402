pub mod circom;
pub mod cryptography;
pub mod transfer;
pub mod transfer_compressed;

use ark_ff::{BigInteger, PrimeField};
use num_bigint::BigUint;
use sha2::{Digest, Sha256};

pub fn compute_alpha(hash_inputs: Vec<ark_bn254::Fr>) -> ark_bn254::Fr {
    let hash_inputs = hash_inputs
        .iter()
        .flat_map(|x| {
            let mut bytes = x.into_bigint().to_bytes_be();
            if bytes.len() < 32 {
                let mut padded = vec![0u8; 32 - bytes.len()];
                padded.extend_from_slice(&bytes);
                bytes = padded;
            }
            bytes
        })
        .collect::<Vec<u8>>();

    let mut hasher = Sha256::new();
    hasher.update(hash_inputs);
    let sha_hash = hasher.finalize();
    let mut alpha = BigUint::from_bytes_be(&sha_hash);
    let mask = (BigUint::from(1u8) << 253) - BigUint::from(1u8);
    alpha &= mask; // Drop three bits from the calculated hash
    ark_bn254::Fr::from(alpha)
}
