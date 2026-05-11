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
