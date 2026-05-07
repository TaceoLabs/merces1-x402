//! Configuration types for the x402 server service.

use std::collections::HashMap;

use alloy::primitives::Address;
use serde::Deserialize;
use taceo_nodes_common::Environment;

/// The configuration for the x402 server service.
#[derive(Debug, Clone, Deserialize)]
pub struct X402ServerServiceConfig {
    /// The environment.
    pub environment: Environment,

    /// The url of the x402 facilitator.
    pub facilitator_url: String,

    /// The wallet that receives the payments.
    pub pay_to: Address,

    /// The default price when no promo code is supplied, e.g. `"$1"`.
    #[serde(default = "default_price")]
    pub default_price: String,

    /// Named price tiers selectable via the `x-price-tier` request header.
    #[serde(default = "default_price_tiers")]
    pub price_tiers: HashMap<String, String>,
}

fn default_price() -> String {
    "$1".to_owned()
}

fn default_price_tiers() -> HashMap<String, String> {
    HashMap::from([
        ("ENTERPRISE".to_owned(), "$1.50".to_owned()),
        ("GROWTH".to_owned(), "$0.80".to_owned()),
        ("STARTUP".to_owned(), "$0.20".to_owned()),
    ])
}
