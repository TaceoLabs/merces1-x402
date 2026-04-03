use crate::circom::groth16::Groth16Material;
use ark_bn254::Bn254;
use circom_proof_schema::proof_schema::CircomProofSchema;
use co_circom::CoCircomCompilerParsed;
use eyre::Context;
use rand::{CryptoRng, Rng};
use std::path::PathBuf;

pub struct CircomConfig {}

impl CircomConfig {
    const ROOT: &str = std::env!("CARGO_MANIFEST_DIR");
    const CIRCOM_LIB: &str = "/../circom";

    const TRANSFER_CIRCUIT: &str = "/../circom/main/server.circom";
    const TRANSFER_R1CS: &str = "/../circom/r1cs/server.r1cs";

    pub const NUM_TRANSACTIONS: usize = 50;
    pub const NUM_COMMITMENTS: usize = 5;
    pub const NUM_TOTAL_COMMITMENTS: usize = Self::NUM_TRANSACTIONS * Self::NUM_COMMITMENTS;
    pub const NUM_PUBLIC_INPUTS: usize = Self::NUM_TOTAL_COMMITMENTS + Self::NUM_TRANSACTIONS;
    pub const DOMAIN_SEPARATOR: u64 = 0xDEADBEEFu64;
    pub const TRANSFER_BALANCE_BITSIZE: usize = 100;
    pub const POSEIDON2_SPONGE_T: usize = 16;
    pub const COMPRESSION: bool = false;

    pub fn get_transfer_circom() -> eyre::Result<CoCircomCompilerParsed<ark_bn254::Fr>> {
        let lib = format!("{}{}", Self::ROOT, Self::CIRCOM_LIB);
        let circuit = format!("{}{}", Self::ROOT, Self::TRANSFER_CIRCUIT);
        CircomProofSchema::<Bn254>::read_circuit_co_circom(
            PathBuf::from(circuit),
            PathBuf::from(lib),
        )
    }

    pub fn get_transfer_proof_schema<R: Rng + CryptoRng>(
        rng: &mut R,
    ) -> eyre::Result<CircomProofSchema<Bn254>> {
        let r1cs = format!("{}{}", Self::ROOT, Self::TRANSFER_R1CS);
        CircomProofSchema::from_r1cs_file_circom(PathBuf::from(r1cs), rng)
            .context("while reading r1cs file")
    }

    pub fn get_transfer_key_material<R: Rng + CryptoRng>(
        rng: &mut R,
    ) -> eyre::Result<Groth16Material> {
        let circuit: CoCircomCompilerParsed<
            ark_ff::Fp<ark_ff::MontBackend<ark_bn254::FrConfig, 4>, 4>,
        > = CircomConfig::get_transfer_circom()?;
        let proof_schema = CircomConfig::get_transfer_proof_schema(rng)?;
        Ok(Groth16Material::new(proof_schema, circuit))
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn compile_transfer_groth16() {
        let groth16 = CircomConfig::get_transfer_key_material(&mut rand::thread_rng()).unwrap();
        let size = groth16.size();
        println!(
            "Transfer (MPC) R1CS size: constraints = {}, witnesses = {}",
            size.0, size.1
        );
    }
}
