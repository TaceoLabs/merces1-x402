//! Facilitator-side payment verification and settlement for V2 EIP-155 confidential scheme.
//!
//! This module implements the facilitator logic for V2 protocol payments on EVM chains.
//! It handles V2-specific payload structures with embedded requirements, CAIP-2 chain IDs,
//! and confidential transfer verification via ZK proofs.

use std::str::FromStr;

use alloy::primitives::{Address, TxHash, U256};
use alloy::providers::{PendingTransactionError, Provider};
use alloy::transports::TransportError;
use ark_groth16::VerifyingKey;
use contract_rs::merces::Merces::MercesInstance;
use contract_rs::merces::MercesContract;
use std::collections::HashMap;
use x402_chain_eip155::v2_eip155_exact::eip3009::assert_requirements_match;
use x402_facilitator_local::FacilitatorLocalError;
use x402_types::chain::ChainId;
use x402_types::facilitator::Facilitator;
use x402_types::proto::v2;
use x402_types::proto::{self, PaymentVerificationError};
use x402_types::scheme::X402SchemeFacilitatorError;
use x402_types::timestamp::UnixTimestamp;

use crate::{
    ConfidentialEvmPayload, ConfidentialScheme, Eip712Domain, PaymentPayload, PaymentRequirements,
    PaymentRequirementsExtra, SettleRequest, VerifyRequest,
};

#[derive(Debug, thiserror::Error)]
pub enum Eip155ConfidentialError {
    #[error(transparent)]
    Transport(#[from] TransportError),
    #[error(transparent)]
    PendingTransaction(#[from] PendingTransactionError),
    #[error("Transaction {0} reverted")]
    TransactionReverted(TxHash),
    #[error("Contract call failed: {0}")]
    ContractCall(String),
    #[error(transparent)]
    PaymentVerification(#[from] PaymentVerificationError),
}

impl From<Eip155ConfidentialError> for X402SchemeFacilitatorError {
    fn from(value: Eip155ConfidentialError) -> Self {
        match value {
            Eip155ConfidentialError::Transport(_) => Self::OnchainFailure(value.to_string()),
            Eip155ConfidentialError::PendingTransaction(_) => {
                Self::OnchainFailure(value.to_string())
            }
            Eip155ConfidentialError::TransactionReverted(_) => {
                Self::OnchainFailure(value.to_string())
            }
            Eip155ConfidentialError::ContractCall(_) => Self::OnchainFailure(value.to_string()),
            Eip155ConfidentialError::PaymentVerification(e) => Self::PaymentVerification(e),
        }
    }
}

// impl From<MetaTransactionSendError> for Eip155ConfidentialError {
//     fn from(e: MetaTransactionSendError) -> Self {
//         match e {
//             MetaTransactionSendError::Transport(e) => Self::Transport(e),
//             MetaTransactionSendError::PendingTransaction(e) => Self::PendingTransaction(e),
//             MetaTransactionSendError::Custom(e) => Self::ContractCall(e),
//         }
//     }
// }

// impl<P> X402SchemeFacilitatorBuilder<P> for V2Eip155Confidential
// where
//     P: Eip155MetaTransactionProvider + ChainProviderOps + Send + Sync + 'static,
//     Eip155ConfidentialError: From<P::Error>,
// {
//     fn build(
//         &self,
//         provider: P,
//         config: Option<serde_json::Value>,
//     ) -> Result<Box<dyn X402SchemeFacilitator>, Box<dyn std::error::Error>> {
//         let config: V2Eip155ConfidentialFacilitatorConfig = config
//             .and_then(|config| serde_json::from_value(config).ok())
//             .unwrap_or_default();
//         Ok(Box::new(V2Eip155ConfidentialFacilitator::new(
//             provider, config,
//         )))
//     }
// }

/// Facilitator for V2 EIP-155 confidential scheme payments.
///
/// This struct implements the `X402SchemeFacilitator` trait to provide payment
/// verification and settlement services for confidential ERC-3009 based payments
/// on EVM chains using the V2 protocol.
pub struct V2Eip155ConfidentialFacilitator<P> {
    provider: P,
    chain_id: u64,
    signer_address: Address,
    contract_address: Address,
    node_urls: [String; 3],
    mpc_pks: [ark_babyjubjub::EdwardsAffine; 3],
    verifying_key: VerifyingKey<ark_bn254::Bn254>,
}

impl<P> V2Eip155ConfidentialFacilitator<P> {
    /// Creates a new V2 EIP-155 confidential scheme facilitator with the given provider.
    pub fn new(
        provider: P,
        chain_id: u64,
        signer_address: Address,
        contract_address: Address,
        node_urls: [String; 3],
        mpc_pks: [ark_babyjubjub::EdwardsAffine; 3],
        verifying_key: VerifyingKey<ark_bn254::Bn254>,
    ) -> Self {
        Self {
            provider,
            chain_id,
            signer_address,
            contract_address,
            node_urls,
            mpc_pks,
            verifying_key,
        }
    }
}

impl<P: Provider> V2Eip155ConfidentialFacilitator<P> {
    async fn verify(
        &self,
        request: &proto::VerifyRequest,
    ) -> Result<proto::VerifyResponse, Eip155ConfidentialError> {
        tracing::debug!("Received verify request");
        let request = VerifyRequest::from_proto(request.clone())?;
        let payload = &request.payment_payload;
        let requirements = &request.payment_requirements;
        let payer = payload.payload.authorization.from;
        assert_requirements_match(&payload.accepted, requirements)?;
        assert_valid_payment(
            &self.provider,
            self.chain_id,
            self.contract_address,
            &self.node_urls,
            &self.verifying_key,
            &self.mpc_pks,
            payload,
            requirements,
        )
        .await?;
        Ok(v2::VerifyResponse::valid(payer.to_string()).into())
    }

    async fn settle(
        &self,
        request: &proto::SettleRequest,
    ) -> Result<proto::SettleResponse, Eip155ConfidentialError> {
        tracing::debug!("Received settle request");
        let request = SettleRequest::from_proto(request.clone())?;
        let payload = &request.payment_payload;
        let requirements = &request.payment_requirements;
        assert_requirements_match(&payload.accepted, requirements)?;
        assert_valid_payment(
            &self.provider,
            self.chain_id,
            self.contract_address,
            &self.node_urls,
            &self.verifying_key,
            &self.mpc_pks,
            payload,
            requirements,
        )
        .await?;
        let tx_hash = settle_payment(&self.provider, self.contract_address, payload).await?;
        Ok(v2::SettleResponse::Success {
            payer: payload.payload.authorization.from.to_string(),
            transaction: tx_hash.to_string(),
            network: payload.accepted.network.to_string(),
        }
        .into())
    }

    async fn supported(&self) -> proto::SupportedResponse {
        let chain_id = ChainId::new("eip155", self.chain_id.to_string());
        let extra = PaymentRequirementsExtra {
            eip_712_domain: Eip712Domain {
                name: "Merces".to_string(),
                version: "1".to_string(),
            },
            confidential_token: self.contract_address,
            mpc_pks: self.mpc_pks,
        };
        let kinds = vec![proto::SupportedPaymentKind {
            x402_version: proto::v2::X402Version2.into(),
            scheme: ConfidentialScheme.to_string(),
            network: chain_id.to_string(),
            extra: serde_json::to_value(extra).ok(),
        }];
        let signers = {
            let mut signers = HashMap::with_capacity(1);
            signers.insert(chain_id, vec![self.signer_address.to_string()]);
            signers
        };
        proto::SupportedResponse {
            kinds,
            extensions: Vec::new(),
            signers,
        }
    }
}

// #[async_trait::async_trait]
// impl<P> X402SchemeFacilitator for V2Eip155ConfidentialFacilitator<P>
// where
//     P: Eip155MetaTransactionProvider + ChainProviderOps + Send + Sync,
//     P::Inner: Provider,
//     Eip155ConfidentialError: From<P::Error>,
// {
//     async fn verify(
//         &self,
//         request: &proto::VerifyRequest,
//     ) -> Result<proto::VerifyResponse, X402SchemeFacilitatorError> {
//         tracing::info!("Received verify request: {:?}", request);
//         let request = VerifyRequest::from_proto(request.clone())?;
//         let payload = &request.payment_payload;
//         let requirements = &request.payment_requirements;
//         assert_requirements_match(&payload.accepted, requirements)?;
//         tracing::info!("Requirements match, checking payment validity on-chain...");
//         assert_valid_payment(
//             &self.provider.inner(),
//             self.config.contract_address,
//             &self.config.node_urls,
//             payload,
//             requirements,
//         )
//         .await?;
//         tracing::info!("Payment is valid, recovering payer address from signature...");
//         let payer = verify_payment(
//             self.provider
//                 .chain_id()
//                 .reference()
//                 .parse()
//                 .expect("valid chain reference"),
//             self.config.contract_address,
//             &payload.payload,
//         )
//         .await?;
//         tracing::info!("Recovered payer address: {payer}");
//         Ok(v2::VerifyResponse::valid(payer.to_string()).into())
//     }

//     async fn settle(
//         &self,
//         request: &proto::SettleRequest,
//     ) -> Result<proto::SettleResponse, X402SchemeFacilitatorError> {
//         let request = SettleRequest::from_proto(request.clone())?;
//         tracing::info!("Received settle request: {:?}", request);
//         let payload = &request.payment_payload;
//         let requirements = &request.payment_requirements;
//         assert_requirements_match(&payload.accepted, requirements)?;
//         assert_valid_payment(
//             &self.provider.inner(),
//             self.config.contract_address,
//             &self.config.node_urls,
//             payload,
//             requirements,
//         )
//         .await?;
//         let tx_hash = settle_payment(
//             &self.provider.inner(),
//             self.config.contract_address,
//             payload,
//         )
//         .await?;
//         Ok(v2::SettleResponse::Success {
//             payer: payload.payload.authorization.from.to_string(),
//             transaction: tx_hash.to_string(),
//             network: payload.accepted.network.to_string(),
//         }
//         .into())
//     }

//     async fn supported(&self) -> Result<proto::SupportedResponse, X402SchemeFacilitatorError> {
//         let chain_id = self.provider.chain_id();
//         let kinds = vec![proto::SupportedPaymentKind {
//             x402_version: proto::v2::X402Version2.into(),
//             scheme: ConfidentialScheme.to_string(),
//             network: chain_id.to_string(),
//             extra: None,
//         }];
//         let signers = {
//             let mut signers = HashMap::with_capacity(1);
//             signers.insert(chain_id, self.provider.signer_addresses());
//             signers
//         };
//         Ok(proto::SupportedResponse {
//             kinds,
//             extensions: Vec::new(),
//             signers,
//         })
//     }
// }

impl<P: Provider> Facilitator for V2Eip155ConfidentialFacilitator<P> {
    type Error = FacilitatorLocalError;

    async fn verify(
        &self,
        request: &proto::VerifyRequest,
    ) -> Result<proto::VerifyResponse, Self::Error> {
        self.verify(request)
            .await
            .map_err(|e| FacilitatorLocalError::Verification(e.into()))
    }

    async fn settle(
        &self,
        request: &proto::SettleRequest,
    ) -> Result<proto::SettleResponse, Self::Error> {
        self.settle(request)
            .await
            .map_err(|e| FacilitatorLocalError::Settlement(e.into()))
    }

    async fn supported(&self) -> Result<proto::SupportedResponse, Self::Error> {
        Ok(self.supported().await)
    }
}

async fn assert_valid_proof(
    verifying_key: &VerifyingKey<ark_bn254::Bn254>,
    mpc_pks: &[ark_babyjubjub::EdwardsAffine; 3],
    payload: &ConfidentialEvmPayload,
) -> Result<(), Eip155ConfidentialError> {
    let public_inputs = [
        payload.authorization.sender_pk.x,
        payload.authorization.sender_pk.y,
        payload.authorization.amount_commitment,
        payload.authorization.ciphertexts[0],
        payload.authorization.ciphertexts[1],
        payload.authorization.ciphertexts[2],
        payload.authorization.ciphertexts[3],
        payload.authorization.ciphertexts[4],
        payload.authorization.ciphertexts[5],
        mpc_pks[0].x,
        mpc_pks[0].y,
        mpc_pks[1].x,
        mpc_pks[1].y,
        mpc_pks[2].x,
        mpc_pks[2].y,
    ];
    let alpha = client::compute_alpha(&public_inputs);
    let beta = payload.authorization.beta;
    let gamma = client::compute_gamma(alpha, beta, &public_inputs);
    taceo_groth16::Groth16::<ark_bn254::Bn254>::verify(
        verifying_key,
        &payload.authorization.proof.clone().into(),
        &[beta, gamma, alpha],
    )
    .map_err(|_| PaymentVerificationError::TransactionSimulation("Invalid proof".to_string()))?;
    Ok(())
}

// TODO run less-than MPC protocol to hide balance
async fn assert_enough_balance(
    node_urls: &[String; 3],
    address: &Address,
    amount_required: U256,
) -> Result<(), Eip155ConfidentialError> {
    let url0 = format!("{}/balance/{address}", node_urls[0]);
    let url1 = format!("{}/balance/{address}", node_urls[1]);
    let url2 = format!("{}/balance/{address}", node_urls[2]);

    let client = reqwest::Client::new();
    let (res0, res1, res2) = tokio::join!(
        client.get(&url0).send(),
        client.get(&url1).send(),
        client.get(&url2).send(),
    );

    let res0 = res0
        .map_err(|_| PaymentVerificationError::InsufficientFunds)?
        .error_for_status()
        .map_err(|_| PaymentVerificationError::InsufficientFunds)?;
    let res1 = res1
        .map_err(|_| PaymentVerificationError::InsufficientFunds)?
        .error_for_status()
        .map_err(|_| PaymentVerificationError::InsufficientFunds)?;
    let res2 = res2
        .map_err(|_| PaymentVerificationError::InsufficientFunds)?
        .error_for_status()
        .map_err(|_| PaymentVerificationError::InsufficientFunds)?;

    let (res0, res1, res2) = tokio::join!(res0.text(), res1.text(), res2.text(),);

    let res0 = res0.map_err(|_| PaymentVerificationError::InsufficientFunds)?;
    let res1 = res1.map_err(|_| PaymentVerificationError::InsufficientFunds)?;
    let res2 = res2.map_err(|_| PaymentVerificationError::InsufficientFunds)?;

    let share0 =
        ark_bn254::Fr::from_str(&res0).map_err(|_| PaymentVerificationError::InsufficientFunds)?;
    let share1 =
        ark_bn254::Fr::from_str(&res1).map_err(|_| PaymentVerificationError::InsufficientFunds)?;
    let share2 =
        ark_bn254::Fr::from_str(&res2).map_err(|_| PaymentVerificationError::InsufficientFunds)?;

    let balance = contract_rs::bn254_fr_to_u256(share0 + share1 + share2);

    if balance < amount_required {
        return Err(PaymentVerificationError::InsufficientFunds.into());
    }

    Ok(())
}

async fn assert_valid_signature(
    chain_id: u64,
    contract_address: Address,
    payload: &ConfidentialEvmPayload,
) -> Result<(), Eip155ConfidentialError> {
    let signing_hash = MercesContract::transfer_from_signing_hash(
        chain_id,
        contract_address,
        payload.authorization.from,
        payload.authorization.to,
        payload.authorization.amount_commitment,
        payload
            .authorization
            .ciphertexts
            .as_chunks()
            .0
            .try_into()
            .expect("can split into 3 chunks of 2"),
        payload.authorization.sender_pk,
        payload.authorization.beta,
        payload.authorization.nonce.into(),
        U256::from(payload.authorization.deadline.as_secs()),
    );

    let signature = alloy::primitives::Signature::try_from(payload.signature.as_ref())
        .map_err(|e| PaymentVerificationError::InvalidSignature(e.to_string()))?;
    let recovered = signature
        .recover_address_from_prehash(&signing_hash)
        .map_err(|e| PaymentVerificationError::InvalidSignature(e.to_string()))?;
    if recovered != payload.authorization.from {
        return Err(PaymentVerificationError::InvalidSignature(
            "Recovered address does not match".to_string(),
        )
        .into());
    }

    Ok(())
}

#[expect(clippy::too_many_arguments)]
async fn assert_valid_payment<P: Provider>(
    provider: &P,
    chain_id: u64,
    contract_address: Address,
    nodes_urls: &[String; 3],
    verifying_key: &VerifyingKey<ark_bn254::Bn254>,
    mpc_pks: &[ark_babyjubjub::EdwardsAffine; 3],
    payload: &PaymentPayload,
    requirements: &PaymentRequirements,
) -> Result<(), Eip155ConfidentialError> {
    tracing::debug!("Checking if scheme matches...");
    if requirements.scheme != ConfidentialScheme {
        return Err(PaymentVerificationError::UnsupportedScheme.into());
    }

    tracing::debug!("Checking if recipient matches...");
    let authorization = &payload.payload.authorization;
    if authorization.to != requirements.pay_to {
        return Err(PaymentVerificationError::RecipientMismatch.into());
    }

    tracing::debug!("Checking if payment has expired...");
    let deadline = authorization.deadline;
    if deadline < UnixTimestamp::now() + 6 {
        return Err(PaymentVerificationError::Expired.into());
    }

    tracing::debug!("Checking if nonce is unique...");
    let contract = MercesInstance::new(contract_address, provider);
    let nonce_used = contract
        .isNonceUsed(
            payload.payload.authorization.from,
            payload.payload.authorization.nonce.into(),
        )
        .call()
        .await
        .map_err(|e| Eip155ConfidentialError::ContractCall(e.to_string()))?;
    if nonce_used {
        return Err(PaymentVerificationError::TransactionSimulation(
            "Nonce already used".to_string(),
        )
        .into());
    }

    tracing::debug!("Checking signature validity...");
    assert_valid_signature(chain_id, contract_address, &payload.payload).await?;

    tracing::debug!("Verifying proof..");
    assert_valid_proof(verifying_key, mpc_pks, &payload.payload).await?;

    tracing::debug!("Checking balance for address {}...", authorization.from);
    let amount_required = payload.accepted.amount;
    assert_enough_balance(nodes_urls, &authorization.from, amount_required).await?;

    tracing::debug!("Payment is valid");

    Ok(())
}

async fn settle_payment<P: Provider>(
    provider: &P,
    contract_address: Address,
    payload: &PaymentPayload,
) -> Result<TxHash, Eip155ConfidentialError> {
    tracing::debug!("Settling payment on-chain");
    let contract = MercesContract { contract_address };
    let (action_index, receipt) = contract
        .transfer_from(
            provider,
            payload.payload.authorization.from,
            payload.payload.authorization.to,
            payload.payload.authorization.amount_commitment,
            payload
                .payload
                .authorization
                .ciphertexts
                .as_chunks()
                .0
                .try_into()
                .expect("can split into 3 chunks of 2"),
            payload.payload.authorization.sender_pk,
            payload.payload.authorization.beta,
            payload.payload.authorization.proof.clone().into(),
            payload.payload.authorization.nonce.into(),
            U256::from(payload.payload.authorization.deadline.as_secs()),
            payload.payload.signature.clone(),
        )
        .await
        .map_err(|e| Eip155ConfidentialError::ContractCall(e.to_string()))?;

    tracing::debug!("Waiting for ProcessedMPC event with action index {action_index}...");
    let (pos, log) = contract
        .get_processed_mpc_event(
            provider,
            receipt.block_number.unwrap_or_default(),
            action_index,
        )
        .await
        .map_err(|e| Eip155ConfidentialError::ContractCall(e.to_string()))?;
    let valid = log.inner.valid[pos];
    tracing::debug!("Received ProcessedMPC event for action index {action_index}, valid: {valid}",);
    if valid {
        Ok(log.transaction_hash.unwrap_or_default())
    } else {
        // TODO did not actually revert, but from the facilitator's perspective it's a failed settlement, maybe add new error
        Err(Eip155ConfidentialError::TransactionReverted(
            log.transaction_hash.unwrap_or_default(),
        ))
    }
}
