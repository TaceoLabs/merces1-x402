use alloy::primitives::Address;
use ark_ff::AdditiveGroup;
use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};
use tower_http::trace::TraceLayer;

use crate::AppState;

/// Represents all possible API errors.
#[expect(clippy::enum_variant_names)]
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("Cannot find resource: \"{0}\"")]
    #[expect(dead_code)]
    NotFound(String),
    #[error("Bad request: \"{0}\"")]
    #[expect(dead_code)]
    BadRequest(String),
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
            Error::NotFound(message) => (StatusCode::NOT_FOUND, message).into_response(),
            Error::BadRequest(message) => (StatusCode::BAD_REQUEST, message).into_response(),
        }
    }
}

/// Builds the main API router for the Merces1 node instance.
///
/// This function sets up:
///
/// - The common `/health` and `/version` endpoints via [`taceo_nodes_common::api::routes`].
/// - The `/balance` endpoint.
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
