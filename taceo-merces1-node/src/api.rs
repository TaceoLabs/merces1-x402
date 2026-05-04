use alloy::primitives::{Address, Bytes, U256};
use ark_ff::AdditiveGroup;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use contract_rs::merces::{Merces::MercesInstance, MercesContract};
use eyre::Context;
use mpc_core::protocols::rep3::Rep3PrimeFieldShare;
use serde::Deserialize;
use taceo_merces1_x402::ConfidentialEvmPayloadAuthorization;
use tower_http::trace::TraceLayer;
use uuid::Uuid;

use crate::AppState;

/// Represents all possible API errors.
#[expect(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("Bad request: \"{0}\"")]
    BadRequest(String),
    #[error("Unauthorized: \"{0}\"")]
    Unauthorized(String),
    #[error(transparent)]
    InternalServerError(#[from] eyre::Report),
}

impl IntoResponse for Error {
    fn into_response(self) -> axum::response::Response {
        tracing::debug!("{self:?}");
        match self {
            Error::InternalServerError(error) => {
                tracing::error!("internal server error: {error:?}");
                (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()).into_response()
            }
            Error::BadRequest(message) => (StatusCode::BAD_REQUEST, message).into_response(),
            Error::Unauthorized(message) => (StatusCode::FORBIDDEN, message).into_response(),
        }
    }
}

/// Builds the main API router for the Merces1 node instance.
///
/// This function sets up:
///
/// - The common `/health` and `/version` endpoints via [`taceo_nodes_common::api::routes`].
/// - The `/balance` endpoint.
/// - The `/balance-ge-amount` endpoint.
/// - An HTTP trace layer via [`TraceLayer`].
///
/// The returned [`Router`] can be incorporated into another router or be served directly by axum.
/// Implementations don't need to configure anything in their `State`, the service is
/// inlined as [`Extension`](https://docs.rs/axum/latest/axum/struct.Extension.html).
pub(crate) fn routes(app_state: AppState) -> Router {
    Router::new()
        .merge(taceo_nodes_common::api::routes_with_services(
            app_state.started_services.clone(),
            taceo_nodes_common::version_info!(),
        ))
        .route("/balance/{address}", get(balance))
        .route("/balance-ge-amount", post(balance_ge_amount))
        .layer(TraceLayer::new_for_http())
        .with_state(app_state)
}

async fn balance(
    State(state): State<AppState>,
    Path(address): Path<Address>,
) -> Result<String, Error> {
    let map = state.map.read().await;
    let balance = map
        .get(&address)
        .map(|share| share.amount.a)
        .unwrap_or(ark_bn254::Fr::ZERO);
    Ok(balance.to_string())
}

#[derive(Debug, Clone, Deserialize)]
struct BalanceGEAmountRequest {
    amount_share: Rep3PrimeFieldShare<ark_bn254::Fr>,
    session_id: Uuid,
    authorization: ConfidentialEvmPayloadAuthorization,
    signature: Bytes,
}

async fn balance_ge_amount(
    State(state): State<AppState>,
    Json(request): Json<BalanceGEAmountRequest>,
) -> Result<String, Error> {
    let signing_hash = MercesContract::transfer_from_signing_hash(
        state.chain_id,
        state.config.merces_contract,
        request.authorization.from,
        request.authorization.to,
        request.authorization.amount_commitment,
        request
            .authorization
            .ciphertexts
            .as_chunks()
            .0
            .try_into()
            .expect("can split into 3 chunks of 2"),
        request.authorization.sender_pk,
        request.authorization.beta,
        request.authorization.nonce.into(),
        U256::from(request.authorization.deadline.as_secs()),
    );

    let signature = alloy::primitives::Signature::try_from(request.signature.as_ref())
        .map_err(|e| Error::BadRequest(e.to_string()))?;
    let recovered = signature
        .recover_address_from_prehash(&signing_hash)
        .map_err(|e| Error::BadRequest(e.to_string()))?;
    if recovered != request.authorization.from {
        return Err(Error::Unauthorized(
            "Recovered address does not match".to_string(),
        ));
    }

    tracing::debug!("Checking if nonce is unique...");
    let contract = MercesInstance::new(state.config.merces_contract, state.http_provider.inner());
    let nonce_used = contract
        .isNonceUsed(
            request.authorization.from,
            request.authorization.nonce.into(),
        )
        .call()
        .await
        .context("while isNonceUsed")?;
    if nonce_used {
        return Err(Error::Unauthorized("Nonce already used".to_string()));
    }

    tracing::debug!("Initializing MPC network session...");
    let net = tokio::time::timeout(
        state.config.mpc_net_init_session_timeout,
        state.network.init_session(request.session_id.as_u128()),
    )
    .await
    .context("timeout while init_session")??;

    tracing::debug!("Running MPC balance check...");
    let map = state.map.read().await;
    let share = tokio::task::block_in_place(|| {
        mpc_nodes::mpc_balance_ge_amount(
            request.authorization.from,
            request.amount_share,
            &map,
            &net,
        )
    })?;

    tracing::debug!("balance >= amount check done");
    Ok(share.a.to_string())
}
