use axum::Router;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use taceo_merces1_x402::{ConfidentialUSDC, V2Eip155Confidential};
use x402_axum::X402Middleware;

use crate::config::X402ServerServiceConfig;

pub mod config;

const SAVE20: &str = "SAVE20";
const SAVE50: &str = "SAVE50";
const SAVE80: &str = "SAVE80";

pub async fn start(config: X402ServerServiceConfig) -> eyre::Result<Router> {
    let x402 = X402Middleware::new(&config.facilitator_url);

    let router = Router::new()
        .route(
            "/api/protected",
            get(handler).layer(x402.with_dynamic_price(move |headers, _uri, _base_url| {
                let promo_code = headers
                    .get("x-promo-code")
                    .and_then(|value| value.to_str().ok());
                let usdc = if config.environment.is_dev() {
                    ConfidentialUSDC::anvil()
                } else {
                    ConfidentialUSDC::base_sepolia()
                };
                let amount = match promo_code {
                    Some(SAVE20) => usdc.parse("$0.8").expect("valid amount"),
                    Some(SAVE50) => usdc.parse("$0.5").expect("valid amount"),
                    Some(SAVE80) => usdc.parse("$0.2").expect("valid amount"),
                    _ => usdc.parse("$1").expect("valid amount"),
                };
                async move { vec![V2Eip155Confidential::price_tag(config.pay_to, amount)] }
            })),
        )
        .merge(taceo_nodes_common::api::routes(
            taceo_nodes_common::version_info!(),
        ));

    Ok(router)
}

async fn handler() -> impl IntoResponse {
    (StatusCode::OK, "protected content")
}
