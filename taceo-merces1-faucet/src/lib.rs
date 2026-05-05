//! Faucet service for the Merces1.
//!
//! Exposes a `POST /claim/{address}` endpoint that transfers a configurable
//! amount from the faucet  wallet into a given private Merces wallet.

use std::{
    collections::HashMap,
    str::FromStr as _,
    sync::{Arc, Mutex},
    time::Instant,
};

use alloy::{
    network::EthereumWallet,
    primitives::{Address, TxHash, U256},
    providers::fillers::CachedNonceManager,
    signers::local::PrivateKeySigner,
};
use axum::Router;
use client::transfer_compressed::TransferCompressed;
use contract_rs::{merces::MercesContract, token::USDCTokenContract};
use eyre::Context as _;
use groth16_material::circom::{CircomGroth16Material, CircomGroth16MaterialBuilder};
use rand_chacha::rand_core::SeedableRng;
use secrecy::ExposeSecret as _;
use taceo_nodes_common::{
    StartedServices,
    web3::{HttpRpcProvider, HttpRpcProviderBuilder},
};

use crate::config::Merces1FaucetServiceConfig;

mod api;
pub mod config;

/// Shared state injected into axum handlers.
#[derive(Clone)]
pub struct AppState {
    pub(crate) started_services: StartedServices,
    pub(crate) provider: HttpRpcProvider,
    pub(crate) contract: MercesContract,
    pub(crate) amount: U256,
    pub(crate) groth16_material: Arc<CircomGroth16Material>,
    pub(crate) mpc_pks: [ark_babyjubjub::EdwardsAffine; 3],
    pub(crate) processed_mpc_timeout: std::time::Duration,
    pub(crate) claims: Arc<Mutex<HashMap<Address, Instant>>>,
}

impl AppState {
    pub async fn claim(&self, receiver: Address) -> eyre::Result<(TxHash, TxHash)> {
        let mut rng = rand_chacha::ChaCha12Rng::from_entropy();
        let mut transfer = TransferCompressed::new(
            contract_rs::u256_to_field(self.amount)?,
            self.mpc_pks,
            &mut rng,
        );
        let onchain_transfer = transfer.compute_alpha();
        let (proof, public_inputs) = transfer.generate_proof(&self.groth16_material, &mut rng)?;
        let beta = public_inputs[0];
        self.groth16_material.verify_proof(&proof, &public_inputs)?;

        let (action_index, receipt) = self
            .contract
            .transfer(
                &self.provider,
                receiver,
                onchain_transfer.amount_commitment,
                onchain_transfer.ciphertexts,
                onchain_transfer.sender_pk,
                beta,
                proof,
            )
            .await?;

        let queued_tx_hash = receipt.transaction_hash;
        let completed_tx_hash = self
            .get_processed_mpc(receipt.block_number.unwrap_or_default(), action_index)
            .await?;
        Ok((queued_tx_hash, completed_tx_hash))
    }

    async fn get_processed_mpc(
        &self,
        from_block: u64,
        action_index: usize,
    ) -> eyre::Result<TxHash> {
        let (pos, log) = tokio::time::timeout(
            self.processed_mpc_timeout,
            self.contract
                .get_processed_mpc_event(self.provider.as_ref(), from_block, action_index),
        )
        .await??;
        if !log.inner.valid[pos] {
            eyre::bail!("Transaction is invalid");
        }
        Ok(log.transaction_hash.unwrap_or_default())
    }
}

/// Initializes the faucet service and returns the axum [`Router`].
pub async fn start(config: Merces1FaucetServiceConfig) -> eyre::Result<Router> {
    let started_services = StartedServices::new();

    let groth16_material = Arc::new(
        CircomGroth16MaterialBuilder::new()
            .bbf_num_2_bits_helper()
            .bbf_inv()
            .build_from_paths(config.zkey_path, config.graph_path)?,
    );

    let signer = PrivateKeySigner::from_str(config.wallet_private_key.expose_secret())
        .context("while parsing faucet wallet private key")?;
    let wallet_address = signer.address();
    let wallet = EthereumWallet::from(signer);

    let provider = HttpRpcProviderBuilder::with_config(&config.rpc_provider_config)
        .environment(config.environment)
        .wallet(wallet)
        .build_with_nonce_manager(CachedNonceManager::default())?; // NOTE: CachedNonceManager is used here to prevent nonce issues when multiple claim requests are made at the same time.

    let contract = MercesContract {
        contract_address: config.merces_contract,
    };

    let mpc_pks = contract
        .get_mpc_public_keys(&provider)
        .await
        .context("while fetching MPC public keys from contract")?;

    let token = USDCTokenContract::new(config.token);
    // max amount we can deposit at once
    let initial_balance = U256::from(2).pow(U256::from(80)) - U256::from(1);
    if !contract.account_exists(&provider, wallet_address).await? {
        tracing::info!("Initial deposit of {initial_balance} tokens to the faucet wallet");
        token
            .approve(&provider, config.merces_contract, initial_balance)
            .await?;
        let (action_index, receipt) = contract
            .deposit(&provider, initial_balance, U256::ZERO)
            .await?;
        contract
            .get_processed_mpc_event(
                provider.as_ref(),
                receipt.block_number.unwrap_or_default(),
                action_index,
            )
            .await?;
    }

    let app_state = AppState {
        started_services,
        provider,
        contract,
        groth16_material,
        mpc_pks: [mpc_pks.0, mpc_pks.1, mpc_pks.2],
        amount: config.amount,
        processed_mpc_timeout: config.processed_mpc_timeout,
        claims: Arc::new(Mutex::new(HashMap::new())),
    };

    Ok(api::routes(app_state))
}
