use alloy::{
    network::EthereumWallet,
    primitives::{Address, U256},
    providers::DynProvider,
};
use client::transfer_compressed::TransferCompressed;
use contract_rs::{merces::MercesContract, token::USDCTokenContract};
use groth16_material::circom::CircomGroth16Material;
use rand::thread_rng;
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;

use crate::keys::PublicKeys;

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

    pub async fn get_balance(&self, token: &Option<USDCTokenContract>) -> eyre::Result<U256> {
        match token {
            Some(token_contract) => {
                crate::get_erc20_balance(&self.provider, token_contract, self.signer).await
            }
            None => crate::get_native_balance(&self.provider, self.signer).await,
        }
    }

    pub async fn deposit(
        &self,
        amount: U256,
        merces_contract: &MercesContract,
        token_contract: &Option<USDCTokenContract>,
    ) -> eyre::Result<usize> {
        let native_amount = match token_contract {
            Some(contract) => {
                contract
                    .approve(&self.provider, merces_contract.contract_address, amount)
                    .await?;
                Default::default()
            }
            None => amount,
        };
        let res = merces_contract
            .deposit(&self.provider, amount, native_amount)
            .await?;
        Ok(res.0)
    }

    pub async fn withdraw(
        &self,
        amount: U256,
        merces_contract: &MercesContract,
    ) -> eyre::Result<usize> {
        let res = merces_contract.withdraw(&self.provider, amount).await?;
        Ok(res.0)
    }

    pub async fn transfer(
        &self,
        amount: U256,
        recipient: Address,
        mpc_pks: PublicKeys,
        merces_contract: &MercesContract,
    ) -> eyre::Result<usize> {
        let mut rng = thread_rng();
        let mut transfer = TransferCompressed::new(
            contract_rs::u256_to_field(amount)?,
            mpc_pks.into(),
            &mut rng,
        );
        let onchain_transfer = transfer.compute_alpha();

        let (proof, public_inputs) = transfer.generate_proof(&self.proving_key, &mut rng)?;
        let beta = public_inputs[0];
        self.proving_key.verify_proof(&proof, &public_inputs)?;

        let res = merces_contract
            .transfer(
                &self.provider,
                recipient,
                onchain_transfer.amount_commitment,
                onchain_transfer.ciphertexts,
                onchain_transfer.sender_pk,
                beta,
                proof,
            )
            .await?;
        Ok(res.0)
    }
}
