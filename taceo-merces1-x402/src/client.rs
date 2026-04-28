//! Client-side payment signing for the V2 EIP-155 "confidential" scheme.
//!
//! This module provides [`V2Eip155ConfidentialClient`] for signing ERC-3009
//! `transferWithAuthorization` payments on EVM chains using the V2 protocol.
//!
//! # Usage
//!
//! ```ignore
//! use taceo_merces1_x402::V2Eip155ConfidentialClient;
//! use alloy::signers::local::PrivateKeySigner;
//! use std::sync::Arc;
//!
//! let signer = PrivateKeySigner::random();
//! let groth16_material = Arc::new(/* built from paths */);
//! let client = V2Eip155ConfidentialClient::new(signer, groth16_material);
//! ```

use std::sync::Arc;
use std::time::Instant;

use alloy::primitives::{Address, FixedBytes, U256};
use alloy::signers::Signer;
use async_trait::async_trait;
use client::transfer_compressed::TransferCompressed;
use contract_rs::merces::MercesContract;
use groth16_material::circom::{CircomGroth16Material, CircomGroth16MaterialBuilder};
use rand::SeedableRng as _;
use x402_chain_eip155::chain::Eip155ChainReference;
use x402_types::proto::v2::ResourceInfo;
use x402_types::proto::{OriginalJson, PaymentRequired, v2};
use x402_types::scheme::X402SchemeId;
use x402_types::scheme::client::{
    PaymentCandidate, PaymentCandidateSigner, X402Error, X402SchemeClient,
};
use x402_types::timestamp::UnixTimestamp;
use x402_types::util::Base64Bytes;

use crate::{
    ConfidentialEvmPayload, ConfidentialEvmPayloadAuthorization, PaymentRequirements,
    PaymentRequirementsExtra, V2Eip155Confidential,
};

const ZKEY_BYTES: &[u8] = include_bytes!("../../circom/artifacts/client.arks.zkey");
const GRAPH_BYTES: &[u8] = include_bytes!("../../circom/graph/client_graph.bin");

/// Client for signing V2 EIP-155 confidential scheme payments.
///
/// This client handles the creation and signing of ERC-3009 `transferWithAuthorization`
/// payments for EVM chains using the V2 protocol. Unlike V1, V2 uses CAIP-2 chain IDs
/// and embeds the accepted requirements directly in the payment payload.
///
/// # Type Parameters
///
/// - `S`: The signer type, which must implement [`alloy::signers::Signer`]
///
/// # Example
///
/// ```ignore
/// use taceo_merces1_x402::V2Eip155ConfidentialClient;
/// use alloy::signers::local::PrivateKeySigner;
///
/// let signer = PrivateKeySigner::random();
/// let client = V2Eip155ConfidentialClient::new(signer);
/// ```
pub struct V2Eip155ConfidentialClient<S> {
    signer: S,
    groth16_material: Arc<CircomGroth16Material>,
}

impl<S> V2Eip155ConfidentialClient<S> {
    /// Creates a new V2 EIP-155 confidential scheme client with the given signer.
    pub fn new(signer: S) -> Self {
        let groth16_material = Arc::new(
            CircomGroth16MaterialBuilder::new()
                .bbf_num_2_bits_helper()
                .bbf_inv()
                .build_from_bytes(ZKEY_BYTES, GRAPH_BYTES)
                .expect("can build groth16 material from bytes"),
        );
        Self {
            signer,
            groth16_material,
        }
    }
}

impl<S> X402SchemeId for V2Eip155ConfidentialClient<S> {
    fn namespace(&self) -> &str {
        V2Eip155Confidential.namespace()
    }

    fn scheme(&self) -> &str {
        V2Eip155Confidential.scheme()
    }
}

impl<S> X402SchemeClient for V2Eip155ConfidentialClient<S>
where
    S: Signer + Clone + Send + Sync + 'static,
{
    fn accept(&self, payment_required: &PaymentRequired) -> Vec<PaymentCandidate> {
        let payment_required = match payment_required {
            PaymentRequired::V2(payment_required) => payment_required,
            PaymentRequired::V1(_) => {
                return vec![];
            }
        };
        payment_required
            .accepts
            .iter()
            .filter_map(|original_requirements_json| {
                let requirements =
                    PaymentRequirements::try_from(original_requirements_json).ok()?;
                let chain_reference = Eip155ChainReference::try_from(&requirements.network).ok()?;
                let candidate = PaymentCandidate {
                    chain_id: requirements.network.clone(),
                    asset: requirements.asset.to_string(),
                    amount: requirements.amount,
                    scheme: self.scheme().to_string(),
                    x402_version: self.x402_version(),
                    pay_to: requirements.pay_to.to_string(),
                    signer: Box::new(PayloadSigner {
                        resource_info: payment_required.resource.clone(),
                        signer: self.signer.clone(),
                        chain_reference,
                        requirements,
                        requirements_json: original_requirements_json.clone(),
                        groth16_material: self.groth16_material.clone(),
                    }),
                };
                Some(candidate)
            })
            .collect::<Vec<_>>()
    }
}

/// Shared EIP-712 signing parameters for ERC-3009 confidential authorization.
#[derive(Clone)]
pub struct ConfidentialSigningParams {
    /// The EIP-155 chain ID (numeric)
    pub chain_id: u64,
    /// The token contract address (verifying contract for EIP-712)
    pub asset_address: Address,
    /// The recipient address for the transfer
    pub pay_to: Address,
    /// The amount to transfer
    pub amount: U256,
    /// Maximum timeout in seconds for the authorization validity window
    pub max_timeout_seconds: u64,
    /// Material for generating and verifying Groth16 proofs, including the proving key and verification key.
    pub groth16_material: Arc<CircomGroth16Material>,
    /// Extra parameters for signing, mpc public keys for proof generation, etc.
    pub extra: PaymentRequirementsExtra,
}

/// Signs an ERC-3009 TransferWithAuthorization using EIP-712 for the confidential scheme.
///
/// This is the signing logic for the V2 EIP-155 confidential scheme client.
/// It constructs the EIP-712 domain, builds the authorization struct with appropriate
/// timing parameters, generates a ZK proof for the confidential transfer, and signs
/// the resulting hash.
pub async fn sign_confidential_authorization<S: Signer + Sync>(
    signer: &S,
    params: &ConfidentialSigningParams,
) -> Result<ConfidentialEvmPayload, X402Error> {
    // Build authorization with timing
    let now = UnixTimestamp::now();
    let deadline = now + params.max_timeout_seconds;
    let nonce: [u8; 32] = rand::random();
    let nonce = FixedBytes(nonce);

    let mut rng = rand_chacha::ChaCha12Rng::from_entropy();
    let mut transfer = TransferCompressed::new(
        contract_rs::u256_to_field(params.amount).expect("amount fits in field"),
        params.extra.mpc_pks,
        &mut rng,
    );
    let onchain_transfer = transfer.compute_alpha();
    let start = Instant::now();
    let (proof, public_inputs) = transfer
        .generate_proof(&params.groth16_material, &mut rng)
        .expect("correct proof generation");
    tracing::info!("Proof generation took {:.2?}", start.elapsed());
    let beta = public_inputs[0];
    params
        .groth16_material
        .verify_proof(&proof, &public_inputs)
        .expect("proof verification");

    let authorization = ConfidentialEvmPayloadAuthorization {
        from: signer.address(),
        to: params.pay_to,
        amount_commitment: onchain_transfer.amount_commitment,
        deadline,
        nonce,
        beta,
        ciphertexts: onchain_transfer
            .ciphertexts
            .as_flattened()
            .try_into()
            .expect("can flatten to len 6"),
        sender_pk: onchain_transfer.sender_pk,
        proof: proof.into(),
    };

    // Compute the EIP-712 digest the same way the contract does.
    let eip712_hash = MercesContract::transfer_from_signing_hash(
        params.chain_id,
        params.extra.confidential_token,
        signer.address(),
        params.pay_to,
        onchain_transfer.amount_commitment,
        onchain_transfer.ciphertexts,
        onchain_transfer.sender_pk,
        beta,
        nonce.into(),
        U256::from(deadline.as_secs()),
    );

    let signature = signer
        .sign_hash(&eip712_hash)
        .await
        .map_err(|e| X402Error::SigningError(format!("{e:?}")))?;

    Ok(ConfidentialEvmPayload {
        signature: signature.as_bytes().into(),
        authorization,
    })
}

struct PayloadSigner<S> {
    signer: S,
    resource_info: Option<ResourceInfo>,
    chain_reference: Eip155ChainReference,
    requirements: PaymentRequirements,
    requirements_json: OriginalJson,
    groth16_material: Arc<CircomGroth16Material>,
}

#[async_trait]
impl<S> PaymentCandidateSigner for PayloadSigner<S>
where
    S: Sync + Signer,
{
    async fn sign_payment(&self) -> Result<String, X402Error> {
        let params = ConfidentialSigningParams {
            chain_id: self.chain_reference.inner(),
            asset_address: self.requirements.asset,
            pay_to: self.requirements.pay_to,
            amount: self.requirements.amount,
            max_timeout_seconds: self.requirements.max_timeout_seconds,
            extra: self.requirements.extra.clone(),
            groth16_material: self.groth16_material.clone(),
        };

        let evm_payload = sign_confidential_authorization(&self.signer, &params).await?;

        let payload = v2::PaymentPayload {
            x402_version: v2::X402Version2,
            accepted: self.requirements_json.clone(),
            resource: self.resource_info.clone(),
            payload: evm_payload,
            extensions: None,
        };
        let json = serde_json::to_vec(&payload)?;
        let b64 = Base64Bytes::encode(&json);

        Ok(b64.to_string())
    }
}
