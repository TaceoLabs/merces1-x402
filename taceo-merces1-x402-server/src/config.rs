//! Configuration types for the x402 server service.

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
}
