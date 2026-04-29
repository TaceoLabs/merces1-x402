//! Configuration types for the Merces1 registry service.

use std::{path::PathBuf, time::Duration};

use alloy::primitives::{Address, U256};
use secrecy::SecretString;
use serde::Deserialize;
use taceo_nodes_common::{Environment, web3};

/// The configuration for the Merces1 faucet service.
#[derive(Debug, Clone, Deserialize)]
pub struct Merces1FaucetServiceConfig {
    /// The environment.
    pub environment: Environment,

    /// Hex-encoded wallet private key for the registry (with or without 0x prefix).
    pub wallet_private_key: SecretString,

    /// The address of the `Merces` contract.
    pub merces_contract: Address,

    /// ERC20 token address
    pub token: Address,

    /// Amount (in wei / token base units) that can be claimed per request.
    #[serde(default = "default_amount")]
    pub amount: U256,

    /// The blockchain RPC config.
    #[serde(rename = "rpc")]
    pub rpc_provider_config: web3::HttpRpcProviderConfig,

    /// Timeout while waiting for `ProcessedMPC` events after a deposit.
    #[serde(with = "humantime_serde")]
    #[serde(default = "default_processed_mpc_timeout")]
    pub processed_mpc_timeout: Duration,

    /// The path to the client proof zkey
    pub zkey_path: PathBuf,

    /// The path to the client proof graph
    pub graph_path: PathBuf,
}

fn default_amount() -> U256 {
    U256::from(1_000_000_000u64) // 1k USDC 
}

fn default_processed_mpc_timeout() -> Duration {
    Duration::from_secs(300)
}
