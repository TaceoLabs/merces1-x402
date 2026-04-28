use std::str::FromStr as _;

use alloy::primitives::Address;
use axum::response::IntoResponse;
use axum::{Router, http::StatusCode, routing::get};
use taceo_merces1_x402::{ConfidentialUSDC, V2Eip155Confidential};
use x402_axum::X402Middleware;

#[tokio::main]
async fn main() -> eyre::Result<()> {
    let address = Address::from_str(
        &std::env::var("ADDRESS")
            .unwrap_or("0x70997970C51812dc3A010C7d01b50e0d17dc79C8".to_string()),
    )?;
    let facilitator_url =
        std::env::var("FACILITATOR_URL").unwrap_or("http://localhost:8080".to_string());

    let usdc = ConfidentialUSDC::anvil();
    let x402 = X402Middleware::new(&facilitator_url);

    let app = Router::new().route(
        "/api/protected",
        get(handler).layer(
            x402.with_price_tag(V2Eip155Confidential::price_tag(address, usdc.parse("$1")?)),
        ),
    );

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8081").await?;
    println!("Listening on http://0.0.0.0:8081");
    axum::serve(listener, app).await?;

    Ok(())
}

async fn handler() -> impl IntoResponse {
    (StatusCode::OK, "protected content")
}
