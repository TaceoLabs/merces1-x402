//! HTTP MPC service for the x402 confidential payments demo.
//!
//! Wraps `e2e::mpc::Mpc` + `contract_rs::MercesContract` behind a thin HTTP server.
//! On boot: generates MPC keys and loads the server proving key (~10s cold start),
//! then idles until a Merces contract address is posted to `/start`.
//!
//! Endpoints:
//!   GET  /pubkeys           — returns the 3 BabyJubJub MPC pubkeys (decimal strings)
//!   POST /start             — body `{ "merces": "0x..." }` — starts polling that contract
//!   GET  /health            — health check
//!
//! After `/start`, the service polls the Merces action queue roughly once per second.
//! When the queue is non-empty it invokes `Mpc::process_mpc()` which runs the 3-party
//! MPC in-process and submits the resulting batch proof on-chain.
//!
//! Usage:
//!   mpc_service                       # starts on :4025
//!   MPC_SERVICE_PORT=4025 mpc_service
//!   RPC_URL=http://... MPC_PRIVATE_KEY=0x... mpc_service

use alloy::{
    network::EthereumWallet,
    primitives::Address,
    signers::local::PrivateKeySigner,
};
use ark_babyjubjub::EdwardsAffine;
use ark_ff::PrimeField;
use contract_rs::merces::MercesContract;
use e2e::{ANVIL_RPC, ANVIL_SKS, mpc::Mpc, proving_keys::ProvingKeys};
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::{str::FromStr, sync::Arc, time::Duration};
use tokio::{
    io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
    sync::Mutex,
};

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Serialize)]
struct PubkeysResponse {
    x1: String,
    y1: String,
    x2: String,
    y2: String,
    x3: String,
    y3: String,
}

#[derive(Deserialize)]
struct StartRequest {
    merces: String,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn fq_to_decimal(f: ark_babyjubjub::Fq) -> String {
    let bigint = f.into_bigint();
    let n = num_bigint::BigUint::new(
        bigint
            .0
            .iter()
            .flat_map(|limb| vec![*limb as u32, (*limb >> 32) as u32])
            .collect(),
    );
    n.to_string()
}

fn encode_pubkeys(pks: [EdwardsAffine; 3]) -> PubkeysResponse {
    PubkeysResponse {
        x1: fq_to_decimal(pks[0].x),
        y1: fq_to_decimal(pks[0].y),
        x2: fq_to_decimal(pks[1].x),
        y2: fq_to_decimal(pks[1].y),
        x3: fq_to_decimal(pks[2].x),
        y3: fq_to_decimal(pks[2].y),
    }
}

// ── Polling loop ─────────────────────────────────────────────────────────────

/// Spawned once `/start` is called. Owns the `Mpc` mutex and the Merces contract
/// address; loops forever, draining the on-chain queue in batches.
async fn poll_loop(mpc: Arc<Mutex<Mpc>>, contract: MercesContract) {
    tracing::info!(
        "[mpc-service] polling loop started for Merces {:?}",
        contract.contract_address
    );
    loop {
        // Scope the lock so we don't hold it across the sleep.
        let should_process = {
            let mpc = mpc.lock().await;
            match contract.get_queue_size(mpc.get_provider()).await {
                Ok(size) => size > 0,
                Err(e) => {
                    tracing::warn!("[mpc-service] get_queue_size failed: {e:#}");
                    false
                }
            }
        };

        if should_process {
            let mut mpc = mpc.lock().await;
            if let Err(e) = mpc.process_mpc(&contract).await {
                tracing::warn!("[mpc-service] process_mpc failed: {e:#}");
            }
        }

        tokio::time::sleep(Duration::from_secs(1)).await;
    }
}

// ── HTTP dispatch ────────────────────────────────────────────────────────────

struct AppState {
    mpc: Arc<Mutex<Mpc>>,
    /// Cached pubkeys (immutable after boot) — avoids locking `mpc` on /pubkeys.
    pubkeys: Arc<PubkeysResponse>,
    /// `Some` once /start has been called. Used to reject a second /start.
    started: Arc<Mutex<Option<Address>>>,
}

async fn handle_request(state: Arc<AppState>, method: &str, path: &str, body: &str) -> (u16, String) {
    match (method, path) {
        ("GET", "/health") => (
            200,
            r#"{"status":"ok","service":"mpc-service"}"#.to_string(),
        ),
        ("GET", "/pubkeys") => (200, serde_json::to_string(&*state.pubkeys).unwrap()),
        ("POST", "/start") => {
            let req: StartRequest = match serde_json::from_str(body) {
                Ok(r) => r,
                Err(e) => return (400, format!(r#"{{"error":"invalid JSON: {e}"}}"#)),
            };
            let merces_addr = match Address::from_str(req.merces.trim()) {
                Ok(a) => a,
                Err(e) => return (400, format!(r#"{{"error":"invalid merces address: {e}"}}"#)),
            };
            let mut started = state.started.lock().await;
            if let Some(existing) = *started {
                return (
                    409,
                    format!(
                        r#"{{"error":"already started with merces={existing:?}"}}"#,
                        existing = existing
                    ),
                );
            }
            *started = Some(merces_addr);
            let contract = MercesContract {
                contract_address: merces_addr,
            };
            let mpc = state.mpc.clone();
            tokio::spawn(poll_loop(mpc, contract));
            (
                200,
                format!(r#"{{"status":"started","merces":"{merces_addr:?}"}}"#),
            )
        }
        _ => (404, r#"{"error":"not found"}"#.to_string()),
    }
}

async fn handle_connection(state: Arc<AppState>, mut stream: TcpStream) -> eyre::Result<()> {
    let (read_half, mut write_half) = stream.split();
    let mut reader = BufReader::new(read_half);

    // Request line
    let mut request_line = String::new();
    reader.read_line(&mut request_line).await?;
    let parts: Vec<&str> = request_line.trim().split(' ').collect();
    if parts.len() < 2 {
        return Ok(());
    }
    let method = parts[0].to_string();
    let path = parts[1].to_string();

    // Headers (only Content-Length matters)
    let mut content_length: usize = 0;
    loop {
        let mut header = String::new();
        let n = reader.read_line(&mut header).await?;
        if n == 0 || header.trim().is_empty() {
            break;
        }
        if header.to_lowercase().starts_with("content-length:") {
            content_length = header
                .split(':')
                .nth(1)
                .unwrap_or("0")
                .trim()
                .parse()
                .unwrap_or(0);
        }
    }

    // Body
    let mut body = vec![0u8; content_length];
    if content_length > 0 {
        reader.read_exact(&mut body).await?;
    }
    let body_str = String::from_utf8_lossy(&body).to_string();

    let (status, response_body) = handle_request(state, &method, &path, &body_str).await;
    let response = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        status,
        if status < 400 { "OK" } else { "Error" },
        response_body.len(),
        response_body,
    );
    write_half.write_all(response.as_bytes()).await?;
    Ok(())
}

// ── Tracing ──────────────────────────────────────────────────────────────────

fn install_tracing() {
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{EnvFilter, fmt};

    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("mpc_service=info,e2e=info,contract_rs=info"))
        .unwrap();
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt::layer().with_target(false))
        .init();
}

// ── Main ─────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> eyre::Result<()> {
    install_tracing();

    let port: u16 = std::env::var("MPC_SERVICE_PORT")
        .unwrap_or_else(|_| "4025".to_string())
        .parse()?;
    let rpc_url: SecretString = std::env::var("RPC_URL")
        .unwrap_or_else(|_| ANVIL_RPC.to_string())
        .into();
    // Anvil account index 1 (matches deploy.ts MPC_PRIVATE_KEY default)
    let mpc_sk = std::env::var("MPC_PRIVATE_KEY").unwrap_or_else(|_| ANVIL_SKS[1].to_string());

    // Deterministic seed — same as e2e so proving key / verifier on-chain match.
    let seed_bytes = b"Solidity_MERCES1";
    let mut seed = [0u8; 32];
    seed[..seed_bytes.len()].copy_from_slice(seed_bytes);
    let mut rng = ChaCha12Rng::from_seed(seed);

    // NOTE: must call ProvingKeys::load (which draws client key first, then server) to match
    // the RNG order used when VerifierServer.sol was generated. Drawing server-only here produces
    // a DIFFERENT proving key → the verifier reverts with ProofInvalid(). See Merces1_updated
    // e2e/src/proving_keys.rs.
    tracing::info!("[mpc-service] loading proving keys (~20s, server + unused client)...");
    let proving_keys = ProvingKeys::load(&mut rng)?;
    let server_key = proving_keys.server.clone();
    tracing::info!("[mpc-service] proving keys loaded");

    // Build the MPC wallet from the provided private key.
    let signer = PrivateKeySigner::from_str(&mpc_sk)?;
    let wallet = EthereumWallet::from(signer);
    let mpc = Mpc::from_wallet(&rpc_url, wallet, server_key, &mut rng).await?;
    let pubkeys = encode_pubkeys(mpc.public_keys().into());
    tracing::info!("[mpc-service] MPC keys generated");

    let state = Arc::new(AppState {
        mpc: Arc::new(Mutex::new(mpc)),
        pubkeys: Arc::new(pubkeys),
        started: Arc::new(Mutex::new(None)),
    });

    let listener = TcpListener::bind(format!("127.0.0.1:{port}")).await?;
    tracing::info!("[mpc-service] listening on http://127.0.0.1:{port}");
    tracing::info!("[mpc-service] GET  /pubkeys  — 3 BabyJubJub pubkeys");
    tracing::info!("[mpc-service] POST /start    — body {{\"merces\":\"0x...\"}}");
    tracing::info!("[mpc-service] GET  /health");

    loop {
        let (stream, _peer) = listener.accept().await?;
        let state = state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_connection(state, stream).await {
                tracing::debug!("[mpc-service] connection error: {e:#}");
            }
        });
    }
}
