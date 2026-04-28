//! High-level client library for interacting with the Merces2 protocol.
//!
//! The [`Client`] type encapsulates:
//! - id registration and index lookup via the id-registry service,
//! - local proof generation for private/confidential operations,
//! - on-chain contract calls and event synchronization,
//! - gateway and history service API calls.

use std::str::FromStr as _;

use alloy::{
    primitives::{Address, TxHash, U256, utils::Unit},
    signers::local::PrivateKeySigner,
    transports::http::reqwest::Url,
};
use client::transfer_compressed::TransferCompressed;
use contract_rs::{merces::MercesContract, token::USDCTokenContract};
use eyre::Context as _;
use groth16_material::circom::CircomGroth16Material;
use rand::{CryptoRng, Rng};
use secrecy::ExposeSecret;
use taceo_nodes_common::web3::{HttpRpcProvider, HttpRpcProviderBuilder};

use crate::config::Merces1ClientConfig;

/// Configuration types for constructing a [`Client`].
pub mod config;

/// High-level protocol client bound to a single signer identity.
///
/// A `Client` holds all long-lived state required to interact with Merces2:
/// derived cryptographic identity material, the user's id-registry index,
/// HTTP/WebSocket connectivity, contract handles, and MPC event tracking.
///
/// Use one instance per signer and reuse it across operations to submit private
/// or confidential transactions and to query balances, addresses, and history.
pub struct Client {
    pub config: Merces1ClientConfig,
    groth16_material: CircomGroth16Material,
    mpc_pks: [ark_babyjubjub::EdwardsAffine; 3],
    address: Address,
    contract: MercesContract,
    provider: HttpRpcProvider,
}

impl Client {
    /// Creates a new client instance and ensures the signer is either already
    /// registered or gets registered as a new user.
    ///
    /// This method derives the protocol secret key from the signer, connects to
    /// the RPC provider, fetches MPC public keys from the contract, starts the
    /// `ProcessedMPC` event watcher, and either fetches or creates the user's
    /// id-registry entry.
    pub async fn new(
        config: Merces1ClientConfig,
        groth16_material: CircomGroth16Material,
        signer: PrivateKeySigner,
    ) -> eyre::Result<Self> {
        let address = signer.address();

        let contract = MercesContract {
            contract_address: config.contract_address,
        };

        let http_rpc_url = Url::from_str(config.http_rpc_url.expose_secret())
            .context("invalid HTTP RPC URL in configuration")?;
        let provider = HttpRpcProviderBuilder::with_default_values(vec![http_rpc_url])
            .confirmations_poll_interval(config.confirmations_poll_interval)
            .environment(config.environment)
            .wallet(signer.into())
            .build()?;

        let mpc_pks = contract
            .get_mpc_public_keys(&provider)
            .await
            .context("while fetching MPC public keys from contract")?;

        Ok(Self {
            config,
            groth16_material,
            mpc_pks: [mpc_pks.0, mpc_pks.1, mpc_pks.2],
            address,
            contract,
            provider,
        })
    }

    /// Submits a deposit directly to the contract.
    pub async fn deposit(&self, amount: U256) -> eyre::Result<(TxHash, TxHash)> {
        match self.config.token {
            None => {
                let (action_index, receipt) = self
                    .contract
                    .deposit(&self.provider, amount, amount)
                    .await
                    .context("while confidential deposit tx")?;

                let queued_tx_hash = receipt.transaction_hash;
                let completed_tx_hash = self
                    .get_processed_mpc(receipt.block_number.unwrap_or_default(), action_index)
                    .await?;
                Ok((queued_tx_hash, completed_tx_hash))
            }
            Some(address) => {
                let token_contract = USDCTokenContract::new(address);
                token_contract
                    .approve(&self.provider, self.config.contract_address, amount)
                    .await
                    .context("while approving token transfer for confidential deposit")?;
                let (action_index, receipt) = self
                    .contract
                    .deposit(&self.provider, amount, U256::ZERO)
                    .await
                    .context("while confidential deposit tx")?;

                let queued_tx_hash = receipt.transaction_hash;
                let completed_tx_hash = self
                    .get_processed_mpc(receipt.block_number.unwrap_or_default(), action_index)
                    .await?;
                Ok((queued_tx_hash, completed_tx_hash))
            }
        }
    }

    /// Submits a withdrawal transaction.
    pub async fn withdraw(&self, amount: U256) -> eyre::Result<(TxHash, TxHash)> {
        let (action_index, receipt) = self.contract.withdraw(&self.provider, amount).await?;
        let queued_tx_hash = receipt.transaction_hash;
        let completed_tx_hash = self
            .get_processed_mpc(receipt.block_number.unwrap_or_default(), action_index)
            .await?;
        Ok((queued_tx_hash, completed_tx_hash))
    }

    /// Submits a transfer to another registered user.
    pub async fn transfer<R: Rng + CryptoRng>(
        &self,
        receiver: Address,
        amount: U256,
        rng: &mut R,
    ) -> eyre::Result<(TxHash, TxHash)> {
        let mut transfer =
            TransferCompressed::new(contract_rs::u256_to_field(amount)?, self.mpc_pks, rng);
        let onchain_transfer = transfer.compute_alpha();
        let (proof, public_inputs) = transfer.generate_proof(&self.groth16_material, rng)?;
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

    /// Fetches and reconstructs the current balance for an address.
    pub async fn get_balance(&self, address: Address) -> eyre::Result<U256> {
        tracing::debug!("Fetching balance for user with address {}", address);
        let url0 = format!("{}/balance/{address}", self.config.node_urls[0]);
        let url1 = format!("{}/balance/{address}", self.config.node_urls[1]);
        let url2 = format!("{}/balance/{address}", self.config.node_urls[2]);

        let (res0, res1, res2) = tokio::join!(
            reqwest::get(&url0),
            reqwest::get(&url1),
            reqwest::get(&url2),
        );

        let res0 = res0
            .context("while sending request to node0")?
            .error_for_status()
            .context("node0 returned error")?;
        let res1 = res1
            .context("while sending request to node1")?
            .error_for_status()
            .context("node1 returned error")?;
        let res2 = res2
            .context("while sending request to node2")?
            .error_for_status()
            .context("node2 returned error")?;

        let (res0, res1, res2) = tokio::join!(res0.text(), res1.text(), res2.text(),);

        let res0 = res0.context("while receiving response from node 0")?;
        let res1 = res1.context("while receiving response from node 1")?;
        let res2 = res2.context("while receiving response from node 2")?;

        let share0 = ark_bn254::Fr::from_str(&res0)
            .map_err(|_| eyre::eyre!("invalid balance from node 0"))?;
        let share1 = ark_bn254::Fr::from_str(&res1)
            .map_err(|_| eyre::eyre!("invalid balance from node 1"))?;
        let share2 = ark_bn254::Fr::from_str(&res2)
            .map_err(|_| eyre::eyre!("invalid balance from node 2"))?;

        let balance = share0 + share1 + share2;

        Ok(balance.into())
    }

    /// Returns the signer address associated with this client.
    pub fn address(&self) -> Address {
        self.address
    }

    pub async fn decimals(&self) -> eyre::Result<u8> {
        if let Some(token_address) = self.config.token {
            tracing::debug!("Using token at address {token_address}");
            let token = USDCTokenContract::new(token_address);
            token
                .decimals(&self.provider)
                .await
                .context("while fetching token decimals")
        } else {
            tracing::debug!("Using native token");
            Ok(Unit::ETHER.get())
        }
    }

    async fn get_processed_mpc(
        &self,
        from_block: u64,
        action_index: usize,
    ) -> eyre::Result<TxHash> {
        let (pos, log) = tokio::time::timeout(
            self.config.processed_mpc_timeout,
            self.contract
                .get_processed_mpc_event(self.provider.as_ref(), from_block, action_index),
        )
        .await??;
        if !log.inner.valid[pos] {
            eyre::bail!("Transaction is invalid");
        } else {
            tracing::debug!("got event, returning transaction hash");
            Ok(log.transaction_hash.unwrap_or_default())
        }
    }
}
