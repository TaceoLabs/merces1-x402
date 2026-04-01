use crate::keys::{Keys, PublicKeys};
use alloy::{network::EthereumWallet, primitives::Address, providers::DynProvider};
use mpc_nodes::circom::groth16::Groth16Material;
use rand::{CryptoRng, Rng};
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;

pub struct Mpc {
    provider: DynProvider,
    signer: Address,
    mpc_keys: Keys,
    proving_key: Arc<Groth16Material>,
}

impl Mpc {
    pub async fn from_wallet<R: Rng + CryptoRng>(
        rpc: &SecretString,
        wallet: EthereumWallet,
        proving_key: Arc<Groth16Material>,
        rng: &mut R,
    ) -> eyre::Result<Self> {
        let signer = wallet.default_signer().address();
        let provider = crate::connect_rpc(rpc.expose_secret(), wallet)
            .await
            .expect("Failed to connect to RPC");
        Ok(Self {
            provider,
            signer,
            proving_key,
            mpc_keys: Keys::random(rng),
        })
    }

    pub fn get_provider(&self) -> &DynProvider {
        &self.provider
    }

    pub fn get_signer(&self) -> Address {
        self.signer
    }

    pub fn public_keys(&self) -> PublicKeys {
        self.mpc_keys.public_keys()
    }
}
