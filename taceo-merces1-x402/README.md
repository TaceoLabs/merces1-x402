# Confidential x402

> Confidential x402 payment scheme for Rust — extends the [x402 protocol](https://www.x402.org) with privacy-preserving payment verification using MPC and ZK proofs.

`taceo-merces1-x402` provides a custom x402 scheme (`V2Eip155Confidential`) that integrates with [`x402-rs`](https://github.com/x402-rs/x402-rs).

## Installation

Add the following to your `Cargo.toml`:

```toml
taceo-merces1-x402 = { git = "https://github.com/TaceoLabs/merces1-x402.git", version = "0.1", features = ["server", "client"] }
```

## Quick Start

### Protect Routes (Server)

Use `V2Eip155Confidential` with `x402-axum` to gate routes behind confidential on-chain payments:

```rust
use std::str::FromStr as _;

use alloy::primitives::Address;
use axum::response::IntoResponse;
use axum::{Router, http::StatusCode, routing::get};
use taceo_merces1_x402::{ConfidentialUSDC, V2Eip155Confidential};
use x402_axum::X402Middleware;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let address = Address::from_str(&std::env::var("ADDRESS")?)?;
    let facilitator_url = std::env::var("FACILITATOR_URL")?;

    let usdc = ConfidentialUSDC::base_sepolia();
    let x402 = X402Middleware::new(&facilitator_url);

    let app = Router::new().route(
        "/api/protected",
        get(handler).layer(
            x402.with_price_tag(V2Eip155Confidential::price_tag(address, usdc.parse("$1")?)),
        ),
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8080").await?;
    println!("Listening on http://0.0.0.0:8080");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn handler() -> impl IntoResponse {
    (StatusCode::OK, "protected content")
}
```

See [`x402-axum` documentation](https://github.com/x402-rs/x402-rs/blob/main/crates/x402-axum/README.md) for more details.

### Send Payments (Client)

Use `V2Eip155ConfidentialClient` with `x402-reqwest` to automatically handle confidential payments:

```rust
use alloy::signers::local::PrivateKeySigner;
use reqwest::Client;
use taceo_merces1_x402::V2Eip155ConfidentialClient;
use x402_reqwest::{ReqwestWithPayments, ReqwestWithPaymentsBuild, X402Client};

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let private_key = std::env::var("PRIVATE_KEY")?;
    let signer: PrivateKeySigner = private_key.parse()?;
    let server_url = std::env::var("SERVER_URL")?;

    let x402_client = X402Client::new().register(V2Eip155ConfidentialClient::new(signer));

    let http_client = Client::new().with_payments(x402_client).build();

    let response = http_client
        .get(format!("{server_url}/api/protected"))
        .send()
        .await?;

    println!("Status: {}", response.status());
    println!("Body: {}", response.text().await?);

    Ok(())
}
```

See [`x402-reqwest` documentation](https://github.com/x402-rs/x402-rs/blob/main/crates/x402-reqwest/README.md) for more details.
