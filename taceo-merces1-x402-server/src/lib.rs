use axum::Router;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use taceo_merces1_x402::{ConfidentialUSDC, V2Eip155Confidential};
use x402_axum::X402Middleware;

use crate::config::X402ServerServiceConfig;

pub mod config;

pub async fn start(config: X402ServerServiceConfig) -> eyre::Result<Router> {
    let x402 = X402Middleware::new(&config.facilitator_url);

    let router = Router::new()
        .route(
            "/api/protected",
            get(handler).layer(x402.with_dynamic_price(move |headers, _uri, _base_url| {
                let price_tier = headers
                    .get("x-price-tier")
                    .and_then(|value| value.to_str().ok())
                    .map(str::to_owned);
                let usdc = if config.environment.is_dev() {
                    ConfidentialUSDC::anvil()
                } else {
                    ConfidentialUSDC::base_sepolia()
                };
                let price_str = price_tier
                    .as_deref()
                    .and_then(|code| config.price_tiers.get(code).map(String::as_str))
                    .unwrap_or(&config.default_price);
                let amount = usdc.parse(price_str).expect("valid amount");
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
