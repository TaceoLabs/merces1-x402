//! Configuration types for a Merces1 node services.

use std::{net::SocketAddr, path::PathBuf, time::Duration};

use alloy::primitives::Address;
use secrecy::SecretString;
use serde::Deserialize;
use taceo_nodes_common::{
    Environment,
    postgres::PostgresConfig,
    web3::{self},
};

/// The configuration for Merces1 node services.
#[derive(Debug, Clone, Deserialize)]
pub struct Merces1NodeServiceConfig {
    /// The environment.
    pub environment: Environment,

    /// Hex-encoded wallet private key (with or without 0x prefix).
    pub wallet_private_key: SecretString,

    /// The Address of the `Merces` contract.
    pub merces_contract: Address,

    /// The blockchain RPC config
    #[serde(rename = "rpc")]
    pub rpc_provider_config: web3::HttpRpcProviderConfig,

    /// The websocket RPC url
    pub ws_rpc_url: String,

    /// The addresses of the other Merces1 nodes
    pub node_addrs: Vec<String>,

    /// The bind addr of the MPC tcp server
    #[serde(default = "default_mpc_bind_addr")]
    pub mpc_bind_addr: SocketAddr,

    /// The party id of the node
    pub party_id: usize,

    /// Max wait time the MPC net waits while accepting connections form other nodes
    #[serde(with = "humantime_serde")]
    #[serde(default = "default_mpc_net_init_session_timeout")]
    pub mpc_net_init_session_timeout: Duration,

    /// Secret key for encrypting/decrypting data for smart contract
    pub mpc_sk: SecretString,

    /// The postgres config for the secret-manager
    #[serde(rename = "postgres")]
    pub postgres_config: PostgresConfig,

    /// The path to the server proof zkey
    pub zkey_path: PathBuf,

    /// The path to the server proof circuit
    pub circuit_path: PathBuf,

    /// The path to the circom lib directory
    pub circom_lib_path: PathBuf,
}

fn default_mpc_bind_addr() -> SocketAddr {
    "0.0.0.0:5432".parse().expect("valid SocketAddr")
}

fn default_mpc_net_init_session_timeout() -> Duration {
    Duration::from_secs(30)
}
