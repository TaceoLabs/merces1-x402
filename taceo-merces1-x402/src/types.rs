//! Type definitions for the V2 EIP-155 "confidential" payment scheme.
//!
//! This module defines the wire format types for ERC-3009 based payments
//! on EVM chains using the V2 x402 protocol.

use std::ops::Mul;

use alloy::primitives::{Address, B256, Bytes, U256};
use serde::{Deserialize, Serialize};
use x402_chain_eip155::chain::Eip155ChainReference;
use x402_types::chain::DeployedTokenAmount;
use x402_types::lit_str;
use x402_types::proto::v2;
use x402_types::timestamp::UnixTimestamp;
use x402_types::util::money_amount::{MoneyAmount, MoneyAmountParseError};

lit_str!(ConfidentialScheme, "confidential");

/// Type alias for V2 payment requirements with EVM-specific types.
///
/// V2 uses CAIP-2 chain IDs and embeds requirements directly in the payload,
/// unlike V1 which uses network names and separate requirement objects.
pub type PaymentRequirements =
    v2::PaymentRequirements<ConfidentialScheme, U256, Address, PaymentRequirementsExtra>;

/// Type alias for V2 payment payloads with embedded requirements and EVM-specific data.
pub type PaymentPayload = v2::PaymentPayload<PaymentRequirements, ConfidentialEvmPayload>;

/// Type alias for V2 verify requests using the confidential EVM payment scheme.
pub type VerifyRequest = v2::VerifyRequest<PaymentPayload, PaymentRequirements>;

/// Type alias for V2 settle requests (same structure as verify requests).
pub type SettleRequest = VerifyRequest;

/// Full payload required to authorize an ERC-3009 transfer.
///
/// This struct contains both the EIP-712 signature and the structured authorization
/// data that was signed. Together, they provide everything needed to execute a
/// `transferWithAuthorization` call on an ERC-3009 compliant token contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfidentialEvmPayload {
    /// The cryptographic signature authorizing the transfer.
    ///
    /// This can be:
    /// - An EOA signature (64-65 bytes, split into r, s, v components)
    /// - An EIP-1271 signature (arbitrary length, validated by contract)
    /// - An EIP-6492 signature (wrapped with deployment data and magic suffix)
    pub signature: Bytes,

    /// The structured authorization data that was signed.
    pub authorization: ConfidentialEvmPayloadAuthorization,
}

/// EIP-712 structured data for ERC-3009 transfer authorization.
///
/// This struct defines the parameters of a `transferWithAuthorization` call:
/// who can transfer tokens, to whom, how much, and during what time window.
/// The struct is signed using EIP-712 typed data signing.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfidentialEvmPayloadAuthorization {
    /// The address authorizing the transfer (token owner).
    pub from: Address,

    /// The recipient address for the transfer.
    pub to: Address,

    /// The commitment to the transfer amount
    #[serde(with = "ark_serde_compat::field")]
    pub amount_commitment: ark_bn254::Fr,

    #[serde(with = "ark_serde_compat::field")]
    pub beta: ark_bn254::Fr,

    #[serde(serialize_with = "ark_serde_compat::serialize_f_seq")]
    #[serde(deserialize_with = "ark_serde_compat::deserialize_f_array")]
    pub ciphertexts: [ark_bn254::Fr; 6],

    #[serde(with = "ark_serde_compat::babyjubjub::affine")]
    pub sender_pk: ark_babyjubjub::EdwardsAffine,

    pub proof: taceo_circom_types::groth16::Proof<ark_bn254::Bn254>,

    /// The authorization expires at this timestamp (exclusive).
    pub deadline: UnixTimestamp,

    /// A unique 32-byte nonce to prevent replay attacks.
    pub nonce: B256,
}

/// Extra EIP-712 domain parameters for token contracts.
///
/// Some token contracts require specific `name` and `version` values in their
/// EIP-712 domain for signature verification. This struct allows servers to
/// specify these values in the payment requirements, avoiding the need for
/// the facilitator to query them from the contract.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Eip712Domain {
    /// The token name as used in the EIP-712 domain.
    pub name: String,

    /// The token version as used in the EIP-712 domain.
    pub version: String,
}

/// Extra EIP-712 domain parameters for token contracts.
///
/// Some token contracts require specific `name` and `version` values in their
/// EIP-712 domain for signature verification. This struct allows servers to
/// specify these values in the payment requirements, avoiding the need for
/// the facilitator to query them from the contract.
#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRequirementsExtra {
    pub eip_712_domain: Eip712Domain,

    /// The address of the confidential token contract.
    pub confidential_token: Address,

    /// The public keys of the MPC parties
    #[serde(serialize_with = "ark_serde_compat::babyjubjub::serialize_affine_seq")]
    #[serde(deserialize_with = "ark_serde_compat::babyjubjub::deserialize_affine_array")]
    pub mpc_pks: [ark_babyjubjub::EdwardsAffine; 3],
}

/// Information about a token deployment on an EVM chain.
///
/// This type contains all the information needed to interact with a token contract,
/// including its address, decimal places, and optional EIP-712 domain parameters
/// for signature verification.
///
/// # Example
///
/// ```ignore
/// use taceo_merces1_x402::{ConfidentialUSDC, Eip155ConfidentialTokenDeployment};
///
/// // Fetch token deployment from a running facilitator
/// let usdc: Eip155ConfidentialTokenDeployment =
///     ConfidentialUSDC::anvil("http://localhost:8080").await.unwrap();
/// assert_eq!(usdc.decimals, 6);
///
/// // Parse a human-readable amount to token units
/// let amount = usdc.parse("10.50").unwrap();
/// assert_eq!(amount.amount, U256::from(10_500_000u64));
/// ```
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub struct Eip155ConfidentialTokenDeployment {
    /// The chain this token is deployed on.
    pub chain_reference: Eip155ChainReference,
    /// The token contract address.
    pub address: Address,
    /// Number of decimal places for the token (e.g., 6 for USDC, 18 for most ERC-20s).
    pub decimals: u8,
}

impl Eip155ConfidentialTokenDeployment {
    /// Creates a token amount from a raw value.
    ///
    /// The value should already be in the token's smallest unit (e.g., wei).
    pub fn amount<V: Into<u64>>(
        &self,
        v: V,
    ) -> DeployedTokenAmount<U256, Eip155ConfidentialTokenDeployment> {
        DeployedTokenAmount {
            amount: U256::from(v.into()),
            token: self.clone(),
        }
    }

    /// Parses a human-readable amount string into token units.
    ///
    /// Accepts formats like `"10.50"`, `"$10.50"`, `"1,000"`, etc.
    /// The amount is scaled by the token's decimal places.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The input cannot be parsed as a number
    /// - The input has more decimal places than the token supports
    /// - The value is out of range
    ///
    /// # Example
    ///
    /// ```ignore
    /// use taceo_merces1_x402::{ConfidentialUSDC, Eip155ConfidentialTokenDeployment};
    ///
    /// let usdc: Eip155ConfidentialTokenDeployment =
    ///     ConfidentialUSDC::anvil("http://localhost:8080").await.unwrap();
    /// let amount = usdc.parse("10.50").unwrap();
    /// // 10.50 USDC = 10,500,000 units (6 decimals)
    /// assert_eq!(amount.amount, U256::from(10_500_000u64));
    /// ```
    pub fn parse<V>(
        &self,
        v: V,
    ) -> Result<DeployedTokenAmount<U256, Eip155ConfidentialTokenDeployment>, MoneyAmountParseError>
    where
        V: TryInto<MoneyAmount>,
        MoneyAmountParseError: From<<V as TryInto<MoneyAmount>>::Error>,
    {
        let money_amount = v.try_into()?;
        let scale = money_amount.scale();
        let token_scale = self.decimals as u32;
        if scale > token_scale {
            return Err(MoneyAmountParseError::WrongPrecision {
                money: scale,
                token: token_scale,
            });
        }
        let scale_diff = token_scale - scale;
        let multiplier = U256::from(10).pow(U256::from(scale_diff));
        let digits = money_amount.mantissa();
        let value = U256::from(digits).mul(multiplier);
        Ok(DeployedTokenAmount {
            amount: value,
            token: self.clone(),
        })
    }
}
