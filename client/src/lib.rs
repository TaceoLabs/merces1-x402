pub mod circom;
pub mod cryptography;
pub mod transfer;
pub mod transfer_compressed;

use ark_ff::{BigInteger, PrimeField};
use num_bigint::BigUint;
use sha2::{Digest, Sha256};

pub fn compute_alpha(hash_inputs: &[ark_bn254::Fr]) -> ark_bn254::Fr {
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

/// Computes gamma via the universal hash function (UHF) over the BN254 scalar field.
///
/// Matches `_computeUhfClient` / `_computeUhfServer` in Merces.sol. `seed = alpha + beta`
/// Horner evaluation: for i from x.len()-1 down to 1: `mul = (seed * (mul + x[i])) % PRIME`
/// Final: `gamma = (mul + x[0]) % PRIME`
pub fn compute_gamma(
    alpha: ark_bn254::Fr,
    beta: ark_bn254::Fr,
    x: &[ark_bn254::Fr],
) -> ark_bn254::Fr {
    let seed = alpha + beta;

    let mut mul = ark_bn254::Fr::default();
    for i in (1..x.len()).rev() {
        mul = seed * (mul + x[i]);
    }

    mul + x[0]
}
