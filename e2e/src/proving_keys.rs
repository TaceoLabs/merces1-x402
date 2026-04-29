use ark_bn254::Bn254;
use ark_groth16::VerifyingKey;
use groth16_material::circom::CircomGroth16Material;
use groth16_sol::{SolidityVerifierConfig, SolidityVerifierContext, askama::Template};
use mpc_nodes::circom::groth16::Groth16Material;
use rand::{CryptoRng, Rng};
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
    sync::Arc,
};

#[derive(Clone)]
pub struct ProvingKeys {
    pub client: Arc<CircomGroth16Material>,
    pub server: Arc<Groth16Material>,
}

impl ProvingKeys {
    pub fn load<R: Rng + CryptoRng>(rng: &mut R) -> eyre::Result<Self> {
        Ok(Self {
            client: Arc::new(client::circom::config::CircomConfig::get_transfer_key_material(rng)?),
            server: Arc::new(
                mpc_nodes::circom::config::CircomConfig::get_transfer_key_material(rng)?,
            ),
        })
    }

    pub fn from_files() -> eyre::Result<Self> {
        let circom_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../circom");
        Ok(Self {
            client: Arc::new(
                client::circom::config::CircomConfig::get_transfer_key_material_from_files(
                    circom_dir.join("artifacts/client.arks.zkey"),
                    circom_dir.join("graph/client_graph.bin"),
                )?,
            ),
            server: Arc::new(
                mpc_nodes::circom::config::CircomConfig::get_transfer_key_material_from_file(
                    circom_dir.join("artifacts/server.arks.zkey"),
                    circom_dir.join("main/server.circom"),
                    circom_dir,
                )?,
            ),
        })
    }

    fn write_solidity_verifier<P: AsRef<Path>>(
        config: SolidityVerifierConfig,
        vk: VerifyingKey<Bn254>,
        path: P,
    ) -> eyre::Result<()> {
        let contract = SolidityVerifierContext { vk, config };
        let mut file = File::create(path)?;
        contract.write_into(&mut file)?;
        Ok(())
    }

    pub fn write_solidity_verifiers(&self, config: SolidityVerifierConfig) -> eyre::Result<()> {
        fs::create_dir_all(format!("{}{}", crate::ROOT, crate::SOLIDITY_PATH))?;
        Self::write_solidity_verifier(
            config.to_owned(),
            self.client.zkey().pk.vk.to_owned(),
            format!(
                "{}{}{}",
                crate::ROOT,
                crate::SOLIDITY_PATH,
                "/VerifierClient.sol"
            ),
        )?;
        Self::write_solidity_verifier(
            config,
            self.server.get_vk(),
            format!(
                "{}{}{}",
                crate::ROOT,
                crate::SOLIDITY_PATH,
                "/VerifierServer.sol"
            ),
        )?;
        Ok(())
    }
}
