use std::{
    str::FromStr as _,
    sync::Arc,
    time::{Duration, Instant},
};

use alloy::{
    network::EthereumWallet, primitives::Address, providers::DynProvider,
    signers::local::PrivateKeySigner,
};
use axum::Router;
use contract_rs::merces::MercesContract;
use mpc_core::protocols::rep3::network::Rep3NetworkExt;
use mpc_net::tcp_session::{TcpNetworkHandler, TcpNetworkHandlerBuilder};
use mpc_nodes::{
    circom::{config::CircomConfig, groth16::Groth16Material},
    map::{DepositValueShare, PrivateDeposit},
};
use secrecy::ExposeSecret as _;
use taceo_nodes_common::{StartedServices, web3::HttpRpcProviderBuilder};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::{config::Merces1NodeServiceConfig, db::DbPool};

mod api;
pub mod config;
mod db;

/// Shared state injected into axum handlers.
#[derive(Clone)]
pub struct AppState {
    /// Tracks which background services have started.
    started_services: StartedServices,
    /// map
    map: Arc<RwLock<PrivateDeposit<Address, DepositValueShare<ark_bn254::Fr>>>>,
}

pub async fn start(
    config: Merces1NodeServiceConfig,
    cancellation_token: CancellationToken,
) -> eyre::Result<(Router, tokio::task::JoinHandle<eyre::Result<()>>)> {
    let started_services = StartedServices::new();

    let signer = PrivateKeySigner::from_str(config.wallet_private_key.expose_secret())?;
    // let wallet_address = signer.address();
    let wallet = EthereumWallet::from(signer);

    let provider = HttpRpcProviderBuilder::with_config(&config.rpc_provider_config)
        .environment(config.environment)
        .wallet(wallet.clone())
        .build()?;

    let network =
        TcpNetworkHandlerBuilder::new(config.party_id, config.mpc_bind_addr, config.node_addrs)
            .time_to_idle(config.mpc_net_init_session_timeout * 2) // remove sessions that are idle for too long, to prevent hanging sessions in case of failures
            .build()
            .await?;

    let mpc_sk = ark_babyjubjub::Fr::from_str(config.mpc_sk.expose_secret()).expect("valid mpc_sk");

    let groth16_material = CircomConfig::get_transfer_key_material_from_file()?;

    let contract = MercesContract {
        contract_address: config.merces_contract,
    };

    let db = DbPool::open(
        &config.postgres_config.connection_string,
        &config.postgres_config.schema,
        config.postgres_config.acquire_timeout,
    )
    .await?;

    let map = db.load_map().await?;
    let map = Arc::new(RwLock::new(map));

    let task = tokio::spawn({
        let map = Arc::clone(&map);
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        async move {
            loop {
                tokio::select! {
                    _ = cancellation_token.cancelled() => {
                        tracing::info!("Cancellation received, stopping task");
                        break;
                    }
                    _ = interval.tick() => {
                        let num_items = contract.get_queue_size(&provider).await?;
                        tracing::info!("Action queue size: {num_items}");
                        if num_items == 0 {
                            continue;
                        }

                        if let Err(error) = process_queue(&contract, &provider, &network, config.party_id, mpc_sk, &groth16_material, &db, &map).await {
                            tracing::error!("Error processing queue: {error:#}");
                            // TODO the action item is likely still in the queue, retry?, remove?
                            break;
                        }
                    }
                }
            }
            Ok(())
        }
    });

    let app_state = AppState {
        started_services,
        map,
    };

    let router = api::routes(app_state);

    Ok((router, task))
}

#[expect(clippy::too_many_arguments)]
async fn process_queue(
    contract: &MercesContract,
    provider: &DynProvider,
    network: &TcpNetworkHandler,
    party_id: usize,
    my_key: ark_babyjubjub::Fr,
    proving_key: &Groth16Material,
    db: &DbPool,
    map: &RwLock<PrivateDeposit<Address, DepositValueShare<ark_bn254::Fr>>>,
) -> eyre::Result<()> {
    tracing::info!("Processing action queue");
    let start = Instant::now();
    // TODO need to do this concurrently
    tracing::debug!("Initializing MPC network sessions");
    let mpc_net_init_start = Instant::now();
    let mut nets = Vec::with_capacity(CircomConfig::NUM_TRANSACTIONS);
    for i in 0..CircomConfig::NUM_TRANSACTIONS {
        let net = network.init_session(i as u128).await?;
        nets.push(net);
    }
    tracing::debug!(
        "MPC network sessions initialized after {:?}",
        mpc_net_init_start.elapsed()
    );

    let (action_indices, actions, ciphertexts) = contract
        .read_queue(CircomConfig::NUM_TRANSACTIONS, provider)
        .await?;
    let queue_size = actions.len();

    tracing::debug!("Read {queue_size} actions from contract, sync with other nodes",);
    let (queue_size_prev, queue_size_next) =
        tokio::task::block_in_place(|| nets[0].broadcast(queue_size))?;
    let queue_size = queue_size.min(queue_size_prev).min(queue_size_next);
    tracing::debug!("Synchronized queue size: {queue_size}");
    let actions = &actions[..queue_size];
    let action_indices = &action_indices[..queue_size];
    let ciphertexts = &ciphertexts[..queue_size];

    let first = action_indices.first().expect("at least one action");
    let last = action_indices.last().expect("at least one action");
    let action_indices = *first..=*last;

    tracing::info!("Processing {} actions {action_indices:?}", actions.len(),);

    let mut map = map.write().await;
    let (applied_transactions, commitments, valid, proof, public_inputs, updated) =
        tokio::task::block_in_place(|| {
            mpc_nodes::mpc_party(
                &my_key,
                actions,
                ciphertexts,
                proving_key,
                &mut map,
                nets.as_slice().try_into().unwrap(),
            )
        })?;

    tracing::info!("Finished MPC computation, verifying proof");

    if !proving_key.verify(&proof, &public_inputs)? {
        eyre::bail!("Proof verification failed");
    }

    tracing::info!(
        "Proof verified successfully, applied {}/{} actions",
        applied_transactions,
        actions.len()
    );

    tracing::info!("Updating DB");
    db.update_map(&updated).await?;

    // only one party needs to submit the result to the contract
    if party_id == 0 {
        tracing::info!("Submitting MPC result to contract");

        let commitments = commitments
            .into_iter()
            .map(contract_rs::bn254_fr_to_u256)
            .collect::<Vec<_>>();
        let beta = public_inputs[0];

        contract
            .process_mpc(
                provider,
                applied_transactions,
                commitments.try_into().unwrap(),
                valid.try_into().unwrap(),
                beta,
                proof,
            )
            .await?;
    }

    tracing::debug!("Sync with other nodes after on-chain update");
    let _ = tokio::time::timeout(
        Duration::from_secs(30),
        tokio::task::spawn_blocking(move || nets[0].broadcast(42u64)),
    )
    .await??;

    // TODO if this fails for some nodes we are stuck in a bad state
    tracing::debug!("Committing map update to DB");
    db.commit_map_update(
        &updated
            .iter()
            .map(|(address, _)| *address)
            .collect::<Vec<_>>(),
    )
    .await?;

    tracing::info!(
        "Finished processing actions {action_indices:?} after {:?}",
        start.elapsed()
    );

    Ok(())
}
