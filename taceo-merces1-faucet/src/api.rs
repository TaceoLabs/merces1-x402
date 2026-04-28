use std::time::{Duration, Instant};

use alloy::primitives::Address;
use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
};
use tower_http::{services::ServeDir, trace::TraceLayer};

use crate::AppState;

const COOLDOWN: Duration = Duration::from_secs(24 * 60 * 60);

/// Represents all possible API errors.
#[derive(Debug, thiserror::Error)]
pub(crate) enum Error {
    #[error("Try again in after 24h")]
    TooManyRequests,
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
            Error::TooManyRequests => {
                (StatusCode::TOO_MANY_REQUESTS, self.to_string()).into_response()
            }
        }
    }
}

/// Builds the main API router for the faucet.
///
/// Exposes:
/// - `/health` and `/version` via [`taceo_nodes_common::api::routes_with_services`].
/// - `POST /claim/{address}` — deposits a fixed amount into the given private Merces wallet.
/// - `GET /` and static assets served from the `static/` directory next to the manifest.
pub(crate) fn routes(app_state: AppState) -> Router {
    let static_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/static");
    let serve_static = ServeDir::new(static_dir).append_index_html_on_directories(true);

    Router::new()
        .merge(taceo_nodes_common::api::routes_with_services(
            app_state.started_services.clone(),
            taceo_nodes_common::version_info!(),
        ))
        .route("/claim/{address}", post(claim))
        .fallback_service(serve_static)
        .layer(TraceLayer::new_for_http())
        .with_state(app_state)
}

async fn claim(State(state): State<AppState>, Path(address): Path<Address>) -> Result<(), Error> {
    {
        let mut claims = state.claims.lock().expect("not poisoned");
        claims.retain(|_, claim| claim.elapsed() < COOLDOWN);
        if claims.get(&address).is_some() {
            tracing::debug!("Address {address} already claimed within last 24h, rejecting claim");
            return Err(Error::TooManyRequests);
        }
    }

    tracing::debug!("Claiming {} for {address}", state.amount);
    state.claim(address).await?;
    tracing::debug!("Claim of {address} successful");

    state
        .claims
        .lock()
        .expect("not poisoned")
        .insert(address, Instant::now());

    Ok(())
}
