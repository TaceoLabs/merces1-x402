//! Server-side price tag generation for V2 EIP-155 confidential scheme.
//!
//! This module provides functionality for servers to create V2 price tags
//! that clients can use to generate payment authorizations. V2 uses CAIP-2
//! chain IDs instead of network names.

use std::sync::Arc;

use alloy::primitives::U256;
use x402_chain_eip155::chain::ChecksummedAddress;
use x402_types::chain::{ChainId, DeployedTokenAmount};
use x402_types::proto::{self, v2};

use crate::{
    ConfidentialScheme, Eip155ConfidentialTokenDeployment, PaymentRequirementsExtra,
    V2Eip155Confidential,
};

impl V2Eip155Confidential {
    /// Creates a V2 price tag for an ERC-3009 payment on an EVM chain.
    ///
    /// This function generates a V2 price tag that specifies the payment requirements
    /// for a resource. Unlike V1, V2 uses CAIP-2 chain IDs (e.g., `eip155:8453`) instead
    /// of network names, and embeds the requirements directly in the price tag.
    ///
    /// # Parameters
    ///
    /// - `pay_to`: The recipient address (can be any type convertible to [`ChecksummedAddress`])
    /// - `asset`: The token deployment and amount required
    ///
    /// # Returns
    ///
    /// A [`v2::PriceTag`] that can be included in a `PaymentRequired` response.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use taceo_merces1_x402::{ConfidentialUSDC, V2Eip155Confidential};
    /// use alloy::primitives::address;
    ///
    /// let pay_to = address!("0x70997970C51812dc3A010C7d01b50e0d17dc79C8");
    /// let usdc = ConfidentialUSDC::anvil();
    /// let price_tag = V2Eip155Confidential::price_tag(
    ///     pay_to,
    ///     usdc.amount(1_000_000u64), // 1 USDC
    /// );
    /// ```
    pub fn price_tag<A: Into<ChecksummedAddress>>(
        pay_to: A,
        asset: DeployedTokenAmount<U256, Eip155ConfidentialTokenDeployment>,
    ) -> v2::PriceTag {
        let chain_id: ChainId = asset.token.chain_reference.into();
        let requirements = v2::PaymentRequirements {
            scheme: ConfidentialScheme.to_string(),
            pay_to: pay_to.into().to_string(),
            asset: asset.token.address.to_string(),
            network: chain_id,
            amount: asset.amount.to_string(),
            max_timeout_seconds: 300,
            extra: None,
        };
        v2::PriceTag {
            requirements,
            enricher: Some(Arc::new(eip155_confidential_enricher)),
        }
    }
}

pub fn eip155_confidential_enricher(
    price_tag: &mut v2::PriceTag,
    supported: &proto::SupportedResponse,
) {
    if price_tag.requirements.extra.is_some() {
        return;
    }

    let extra = supported
        .kinds
        .iter()
        .find(|kind| {
            v2::X402Version2 == kind.x402_version
                && kind.scheme == ConfidentialScheme.to_string()
                && kind.network == price_tag.requirements.network.to_string()
        })
        .and_then(|kind| kind.extra.as_ref())
        .and_then(|extra| serde_json::from_value::<PaymentRequirementsExtra>(extra.clone()).ok());

    if let Some(extra) = extra {
        price_tag.requirements.extra = serde_json::to_value(&extra).ok();
    }
}
