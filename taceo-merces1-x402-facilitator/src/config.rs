//! Configuration types for the x402 facilitator service.

use alloy::primitives::Address;
use secrecy::SecretString;
use serde::Deserialize;
use taceo_nodes_common::{Environment, web3};

/// The configuration for the x402 facilitator service.
#[derive(Debug, Clone, Deserialize)]
pub struct X402FacilitatorServiceConfig {
    /// The environment.
    pub environment: Environment,

    /// Hex-encoded wallet private key (with or without 0x prefix).
    pub wallet_private_key: SecretString,

    /// The Address of the `Merces` contract.
    pub merces_contract: Address,

    /// The addresses of the MPC nodes
    pub node_urls: [String; 3],

    /// The BabyJubJub public keys of the MPC nodes.
    #[serde(deserialize_with = "ark_serde_compat::deserialize_f_array")]
    pub mpc_pks: [ark_babyjubjub::Fq; 6],

    /// The blockchain RPC config
    #[serde(rename = "rpc")]
    pub rpc_provider_config: web3::HttpRpcProviderConfig,
}
