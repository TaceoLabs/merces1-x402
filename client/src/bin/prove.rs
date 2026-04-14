//! HTTP proof generation sidecar for the x402 TypeScript agent.
//!
//! Loads the proving key once on startup (~10s), then serves compressed Groth16
//! client transfer proofs via HTTP. The TypeScript agent just does:
//!   fetch('http://localhost:4024/prove', { method: 'POST', body: JSON.stringify({ amount, mpcPks }) })
//!
//! Endpoints:
//!   POST /prove  — generate a compressed proof
//!   GET  /health — health check
//!
//! Usage:
//!   prove                    # starts on :4024
//!   prove --port 4024        # explicit port
//!   PROVE_PORT=4024 prove    # env var

use alloy::primitives::U256;
use ark_babyjubjub::EdwardsAffine;
use ark_bn254::Fr;
use ark_ff::PrimeField;
use client::transfer_compressed::TransferCompressed;
use groth16_material::circom::CircomGroth16Material;
use groth16_sol::prepare_compressed_proof;
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// ── Types ────────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct ProveRequest {
    amount: String,
    #[serde(rename = "mpcPks")]
    mpc_pks: MpcPks,
}

#[derive(Deserialize)]
struct MpcPks {
    x1: String,
    y1: String,
    x2: String,
    y2: String,
    x3: String,
    y3: String,
}

#[derive(Serialize)]
struct ProveResponse {
    #[serde(rename = "compressedProof")]
    compressed_proof: [String; 4],
    beta: String,
    #[serde(rename = "amountCommitment")]
    amount_commitment: String,
    ciphertext: CiphertextOut,
}

#[derive(Serialize)]
struct CiphertextOut {
    amount: [String; 3],
    r: [String; 3],
    #[serde(rename = "senderPk")]
    sender_pk: PointOut,
}

#[derive(Serialize)]
struct PointOut {
    x: String,
    y: String,
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn parse_fr(s: &str) -> eyre::Result<Fr> {
    let n = num_bigint::BigUint::parse_bytes(s.trim().as_bytes(), 10)
        .ok_or_else(|| eyre::eyre!("invalid decimal: {s}"))?;
    Ok(Fr::from(n))
}

fn fr_to_decimal(f: Fr) -> String {
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

fn u256_to_decimal(v: U256) -> String {
    v.to_string()
}

fn generate_proof(
    proving_key: &CircomGroth16Material,
    amount: Fr,
    mpc_pks: [EdwardsAffine; 3],
) -> eyre::Result<ProveResponse> {
    let mut rng = rand::thread_rng();
    let mut transfer = TransferCompressed::new(amount, mpc_pks, &mut rng);
    let onchain = transfer.compute_alpha();
    let (proof, public_inputs) = transfer.generate_proof(proving_key, &mut rng)?;
    proving_key.verify_proof(&proof, &public_inputs)?;

    let compressed = prepare_compressed_proof(&proof);
    let beta = public_inputs[0];

    Ok(ProveResponse {
        compressed_proof: [
            u256_to_decimal(compressed[0]),
            u256_to_decimal(compressed[1]),
            u256_to_decimal(compressed[2]),
            u256_to_decimal(compressed[3]),
        ],
        beta: fr_to_decimal(beta),
        amount_commitment: fr_to_decimal(onchain.amount_commitment),
        ciphertext: CiphertextOut {
            amount: [
                fr_to_decimal(onchain.ciphertexts[0][0]),
                fr_to_decimal(onchain.ciphertexts[1][0]),
                fr_to_decimal(onchain.ciphertexts[2][0]),
            ],
            r: [
                fr_to_decimal(onchain.ciphertexts[0][1]),
                fr_to_decimal(onchain.ciphertexts[1][1]),
                fr_to_decimal(onchain.ciphertexts[2][1]),
            ],
            sender_pk: PointOut {
                x: fr_to_decimal(onchain.sender_pk.x),
                y: fr_to_decimal(onchain.sender_pk.y),
            },
        },
    })
}

// ── Server ───────────────────────────────────────────────────────────────────

use std::io::{BufRead, Write};
use std::net::TcpListener;

fn handle_request(
    proving_key: &CircomGroth16Material,
    method: &str,
    path: &str,
    body: &str,
) -> (u16, String) {
    if method == "GET" && path == "/health" {
        return (200, r#"{"status":"ok","service":"prove"}"#.to_string());
    }

    if method == "POST" && path == "/prove" {
        let req: ProveRequest = match serde_json::from_str(body) {
            Ok(r) => r,
            Err(e) => return (400, format!(r#"{{"error":"invalid JSON: {e}"}}"#)),
        };

        let amount = match parse_fr(&req.amount) {
            Ok(a) => a,
            Err(e) => return (400, format!(r#"{{"error":"invalid amount: {e}"}}"#)),
        };

        let mpc_pks = match (|| -> eyre::Result<[EdwardsAffine; 3]> {
            Ok([
                EdwardsAffine::new(parse_fr(&req.mpc_pks.x1)?, parse_fr(&req.mpc_pks.y1)?),
                EdwardsAffine::new(parse_fr(&req.mpc_pks.x2)?, parse_fr(&req.mpc_pks.y2)?),
                EdwardsAffine::new(parse_fr(&req.mpc_pks.x3)?, parse_fr(&req.mpc_pks.y3)?),
            ])
        })() {
            Ok(pks) => pks,
            Err(e) => return (400, format!(r#"{{"error":"invalid mpcPks: {e}"}}"#)),
        };

        match generate_proof(proving_key, amount, mpc_pks) {
            Ok(resp) => (200, serde_json::to_string(&resp).unwrap()),
            Err(e) => (
                500,
                format!(r#"{{"error":"proof generation failed: {e}"}}"#),
            ),
        }
    } else {
        (404, r#"{"error":"not found"}"#.to_string())
    }
}

fn main() -> eyre::Result<()> {
    let port: u16 = std::env::var("PROVE_PORT")
        .unwrap_or_else(|_| "4024".to_string())
        .parse()?;

    // Load proving key (one-time, ~10s)
    let mut seed = [0u8; 32];
    let seed_str = b"Solidity_MERCES1";
    seed[..seed_str.len()].copy_from_slice(seed_str);
    let mut rng = ChaCha12Rng::from_seed(seed);

    eprintln!("[prove] Loading proving key...");
    let proving_key: Arc<CircomGroth16Material> =
        Arc::new(client::circom::config::CircomConfig::get_transfer_key_material(&mut rng)?);
    eprintln!("[prove] Proving key loaded.");

    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))?;
    eprintln!("[prove] Listening on http://127.0.0.1:{port}");
    eprintln!("[prove] POST /prove  — generate proof");
    eprintln!("[prove] GET  /health — health check");

    for stream in listener.incoming() {
        let mut stream = stream?;
        let mut reader = std::io::BufReader::new(&stream);

        // Parse HTTP request line
        let mut request_line = String::new();
        reader.read_line(&mut request_line)?;
        let parts: Vec<&str> = request_line.trim().split(' ').collect();
        if parts.len() < 2 {
            continue;
        }
        let method = parts[0];
        let path = parts[1];

        // Read headers
        let mut content_length: usize = 0;
        loop {
            let mut header = String::new();
            reader.read_line(&mut header)?;
            if header.trim().is_empty() {
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

        // Read body
        let mut body = vec![0u8; content_length];
        if content_length > 0 {
            std::io::Read::read_exact(&mut reader, &mut body)?;
        }
        let body_str = String::from_utf8_lossy(&body);

        let start = std::time::Instant::now();
        let (status, response_body) = handle_request(&proving_key, method, path, &body_str);
        let elapsed = start.elapsed();

        if path == "/prove" {
            eprintln!("[prove] {} {} — {}ms", method, path, elapsed.as_millis());
        }

        // Write HTTP response
        let response = format!(
            "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            status,
            if status == 200 { "OK" } else { "Error" },
            response_body.len(),
            response_body,
        );
        stream.write_all(response.as_bytes())?;
    }

    Ok(())
}
