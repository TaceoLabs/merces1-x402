use ark_bn254::Bn254;
use ark_ec::pairing::Pairing;
use ark_serialize::CanonicalSerialize;
use circom_proof_schema::proof_schema::CircomProofSchema;
use eyre::Context;
use groth16_material::circom::{ArkZkey, CircomGroth16Material, CircomGroth16MaterialBuilder};
use rand::{CryptoRng, Rng};
use std::path::PathBuf;

pub struct CircomConfig {}

impl CircomConfig {
    const ROOT: &str = std::env!("CARGO_MANIFEST_DIR");

    const TRANSFER_R1CS: &str = "/../circom/r1cs/client.r1cs";
    const TRANSFER_GRAPH: &str = "/../circom/graph/client_graph.bin";

    pub const AMOUNT_BITSIZE: usize = 80;

    pub(crate) fn proof_schema_to_zkey_bytes<P: Pairing>(
        proof_schema: CircomProofSchema<P>,
    ) -> eyre::Result<Vec<u8>> {
        let zkey = ArkZkey {
            matrices: proof_schema.matrices.into(),
            pk: proof_schema.pk,
        };
        let mut zkey_bytes = Vec::new();
        zkey.serialize_uncompressed(&mut zkey_bytes)?;
        Ok(zkey_bytes)
    }

    pub fn get_transfer_proof_schema<R: Rng + CryptoRng>(
        rng: &mut R,
    ) -> eyre::Result<CircomProofSchema<Bn254>> {
        let r1cs = format!("{}{}", Self::ROOT, Self::TRANSFER_R1CS);
        CircomProofSchema::from_r1cs_file_circom(PathBuf::from(r1cs), rng)
            .context("while reading r1cs file")
    }

    pub fn get_transfer_graph() -> eyre::Result<Vec<u8>> {
        let graph_path = format!("{}{}", Self::ROOT, Self::TRANSFER_GRAPH);
        std::fs::read(graph_path).context("while reading graph file")
    }

    pub fn get_transfer_key_material<R: Rng + CryptoRng>(
        rng: &mut R,
    ) -> eyre::Result<CircomGroth16Material> {
        let proof_schema = Self::get_transfer_proof_schema(rng)?;
        let graph = Self::get_transfer_graph()?;
        let zkey_bytes = Self::proof_schema_to_zkey_bytes(proof_schema)?;
        CircomGroth16MaterialBuilder::new()
            .bbf_num_2_bits_helper()
            .bbf_inv()
            .build_from_bytes(&zkey_bytes, &graph)
            .context("While building CircomGroth16Material")
    }
}

#[cfg(test)]
mod test {
    use crate::circom::config::CircomConfig;

    #[test]
    fn compile_transfer_groth16() {
        let key_material =
            CircomConfig::get_transfer_key_material(&mut rand::thread_rng()).unwrap();

        println!(
            "Transfer (Client) R1CS size: constraints = {}, witnesses = {}",
            key_material.zkey().matrices.0.num_constraints,
            key_material.zkey().matrices.0.num_witness_variables
        );
    }
}
