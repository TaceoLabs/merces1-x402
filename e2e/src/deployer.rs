use alloy::{
    network::EthereumWallet,
    primitives::{Address, U256},
    providers::DynProvider,
};
use contract_rs::{DeployToken, merces::MercesContract, token::USDCTokenContract};
use secrecy::{ExposeSecret, SecretString};

use crate::keys::PublicKeys;

pub struct Deployer {
    provider: DynProvider,
    signer: Address,
}

impl Deployer {
    pub async fn from_wallet(rpc: &SecretString, wallet: EthereumWallet) -> eyre::Result<Self> {
        let signer = wallet.default_signer().address();
        let provider = crate::connect_rpc(rpc.expose_secret(), wallet)
            .await
            .expect("Failed to connect to RPC");
        Ok(Self { provider, signer })
    }

    pub fn get_provider(&self) -> &DynProvider {
        &self.provider
    }

    pub fn get_signer(&self) -> Address {
        self.signer
    }

    pub async fn deploy(
        &self,
        mpc_address: Address,
        keys: PublicKeys,
        token: DeployToken,
    ) -> eyre::Result<(MercesContract, Option<USDCTokenContract>)> {
        contract_rs::deploy(
            &self.provider,
            mpc_address,
            keys.mpc_pk1,
            keys.mpc_pk2,
            keys.mpc_pk3,
            token,
            U256::from(crate::TOKEN_SUPPLY),
        )
        .await
    }

    pub async fn send_tokens(
        &self,
        token_contract: &USDCTokenContract,
        receiver: Address,
        amount: U256,
    ) -> eyre::Result<()> {
        token_contract
            .approve(&self.provider, receiver, amount)
            .await?;
        token_contract
            .transfer(&self.provider, receiver, amount)
            .await?;
        Ok(())
    }
}
