use alloy::{network::EthereumWallet, primitives::Address, providers::DynProvider};
use groth16_material::circom::CircomGroth16Material;
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;

pub struct User {
    provider: DynProvider,
    signer: Address,
    proving_key: Arc<CircomGroth16Material>,
}

impl User {
    pub async fn from_wallet(
        rpc: &SecretString,
        wallet: EthereumWallet,
        proving_key: Arc<CircomGroth16Material>,
    ) -> eyre::Result<Self> {
        let signer = wallet.default_signer().address();
        let provider = crate::connect_rpc(rpc.expose_secret(), wallet)
            .await
            .expect("Failed to connect to RPC");
        Ok(Self {
            provider,
            signer,
            proving_key,
        })
    }

    pub fn get_provider(&self) -> &DynProvider {
        &self.provider
    }

    pub fn get_signer(&self) -> Address {
        self.signer
    }
}
