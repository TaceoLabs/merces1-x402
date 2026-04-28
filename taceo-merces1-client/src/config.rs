use std::time::Duration;

use alloy::primitives::Address;
use clap::Parser;
use secrecy::SecretString;
use taceo_nodes_common::Environment;

#[derive(Debug, Parser)]
pub struct Merces1ClientConfig {
    /// The environment of Merces1
    #[clap(long, env = "MERCES1_CLIENT_ENVIRONMENT", default_value = "prod")]
    pub environment: Environment,

    /// The node urls
    #[clap(
        long,
        env = "MERCES1_CLIENT_NODE_URLS",
        value_delimiter = ',',
        default_value = "http://localhost:10010,http://localhost:10011,http://localhost:10012"
    )]
    pub node_urls: Vec<String>,

    /// The address of the Merces contract.
    #[clap(long, env = "MERCES1_CLIENT_CONTRACT_ADDRESS")]
    pub contract_address: Address,

    /// ERC20 token address, or use the native token if None.
    #[clap(long, env = "MERCES1_CLIENT_TOKEN_ADDRESS")]
    pub token: Option<Address>,

    /// The http rpc url
    #[clap(
        long,
        env = "MERCES1_CLIENT_HTTP_RPC_URL",
        default_value = "http://127.0.0.1:8545"
    )]
    pub http_rpc_url: SecretString,

    /// The poll interval for tx confirmations
    #[clap(
        long,
        env = "MERCES1_CLIENT_CONFIRMATIONS_POLL_INTERVAL",
        value_parser = humantime::parse_duration,
        default_value = "1s"
    )]
    pub confirmations_poll_interval: Duration,

    /// The wallet private key
    #[clap(long)]
    pub private_key: SecretString,

    /// Timeout while waiting for ProcessedMPC events.
    #[clap(
        long,
        env = "MERCES1_CLIENT_PROCESSED_MPC_TIMEOUT",
        value_parser = humantime::parse_duration,
        default_value= "5min"
    )]
    pub processed_mpc_timeout: Duration,
}
