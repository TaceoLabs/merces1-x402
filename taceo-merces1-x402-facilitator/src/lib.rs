use std::{str::FromStr as _, sync::Arc};

use alloy::providers::Provider as _;
use alloy::providers::fillers::CachedNonceManager;
use alloy::{network::EthereumWallet, signers::local::PrivateKeySigner};
use axum::Router;
use axum::http::Method;
use secrecy::ExposeSecret as _;
use taceo_circom_types::groth16::VerificationKey;
use taceo_merces1_x402::facilitator::V2Eip155ConfidentialFacilitator;
use taceo_nodes_common::web3::HttpRpcProviderBuilder;
use tower_http::cors;
use x402_facilitator_local::handlers;

use crate::config::X402FacilitatorServiceConfig;

pub mod config;

pub async fn start(config: X402FacilitatorServiceConfig) -> eyre::Result<Router> {
    let signer = PrivateKeySigner::from_str(config.wallet_private_key.expose_secret())?;
    let wallet_address = signer.address();
    let wallet = EthereumWallet::from(signer);

    let provider = HttpRpcProviderBuilder::with_config(&config.rpc_provider_config)
        .environment(config.environment)
        .wallet(wallet.clone())
        .build_with_nonce_manager(CachedNonceManager::default())?; // NOTE: CachedNonceManager is used to prevent nonce issues when multiple payments are settled at the same time.
    let chain_id = provider.get_chain_id().await?;

    let mpc_pks = [
        ark_babyjubjub::EdwardsAffine::new(config.mpc_pks[0], config.mpc_pks[1]),
        ark_babyjubjub::EdwardsAffine::new(config.mpc_pks[2], config.mpc_pks[3]),
        ark_babyjubjub::EdwardsAffine::new(config.mpc_pks[4], config.mpc_pks[5]),
    ];

    let vk_bytes = std::fs::read(config.vk_path)?;
    let verifying_key = serde_json::from_slice::<VerificationKey<ark_bn254::Bn254>>(&vk_bytes)?;

    let facilitator = Arc::new(V2Eip155ConfidentialFacilitator::new(
        provider.inner(),
        chain_id,
        wallet_address,
        config.merces_contract,
        config.node_urls,
        mpc_pks,
        verifying_key.into(),
    ));

    let router = handlers::routes().with_state(facilitator).layer(
        cors::CorsLayer::new()
            .allow_origin(cors::Any)
            .allow_methods([Method::GET, Method::POST])
            .allow_headers(cors::Any),
    );

    Ok(router)
}
