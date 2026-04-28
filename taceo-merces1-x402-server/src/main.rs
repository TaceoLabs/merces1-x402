use std::{net::SocketAddr, process::ExitCode, time::Duration};

use config::Config;
use eyre::Context as _;
use serde::Deserialize;
use taceo_merces1_x402_server::config::X402ServerServiceConfig;

/// Top-level configuration for the x402 server binary.
///
/// Configured via environment variables using the `MERCES1_X402_SERVER__` prefix
/// and `__` as the separator.
#[derive(Clone, Debug, Deserialize)]
struct X402ServerConfig {
    /// Bind address for the Axum server.
    #[serde(default = "default_bind_addr")]
    pub bind_addr: SocketAddr,

    /// Max wait time during graceful shutdown.
    #[serde(default = "default_max_wait_shutdown")]
    #[serde(with = "humantime_serde")]
    pub max_wait_time_shutdown: Duration,

    /// The server service config.
    #[serde(rename = "service")]
    pub service_config: X402ServerServiceConfig,
}

fn default_bind_addr() -> SocketAddr {
    "0.0.0.0:4321".parse().expect("valid SocketAddr")
}

fn default_max_wait_shutdown() -> Duration {
    Duration::from_secs(10)
}

async fn run() -> eyre::Result<()> {
    let config = Config::builder()
        .add_source(
            config::Environment::with_prefix("MERCES1_X402_SERVER")
                .separator("__")
                .try_parsing(true),
        )
        .build()?
        .try_deserialize::<X402ServerConfig>()?;

    let (cancellation_token, _) =
        taceo_nodes_common::spawn_shutdown_task(taceo_nodes_common::default_shutdown_signal());

    let bind_addr = config.bind_addr;
    let max_wait_time_shutdown = config.max_wait_time_shutdown;

    let router = taceo_merces1_x402_server::start(config.service_config)
        .await
        .context("while initializing x402 server")?;

    let server = tokio::spawn({
        let cancellation_token = cancellation_token.clone();
        async move {
            let _drop_guard = cancellation_token.drop_guard_ref();
            tracing::info!("starting axum server on {bind_addr}");
            let tcp_listener = tokio::net::TcpListener::bind(bind_addr)
                .await
                .context("while binding tcp-listener")?;
            let axum_result = axum::serve(tcp_listener, router)
                .with_graceful_shutdown({
                    let cancellation_token = cancellation_token.clone();
                    async move { cancellation_token.cancelled().await }
                })
                .await
                .context("while running axum");
            tracing::info!("axum server shutdown");
            axum_result
        }
    });

    tracing::info!("x402 server started - waiting for shutdown...");
    cancellation_token.cancelled().await;

    tracing::info!("waiting for shutdown (max {max_wait_time_shutdown:?})..");

    match tokio::time::timeout(max_wait_time_shutdown, server).await {
        Ok(Ok(Ok(_))) => {
            tracing::info!("successfully finished shutdown in time");
            Ok(())
        }
        Ok(Ok(Err(err))) => Err(err),
        Ok(Err(join_err)) => eyre::bail!("server task panicked: {join_err}"),
        Err(_) => eyre::bail!("could not finish shutdown in time"),
    }
}

#[tokio::main]
async fn main() -> ExitCode {
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Can install");
    let tracing_config =
        taceo_nodes_observability::TracingConfig::try_from_env().expect("Can create TracingConfig");
    let _tracing_handle = taceo_nodes_observability::initialize_tracing(&tracing_config)
        .expect("Can get tracing handle");
    match run().await {
        Ok(_) => {
            tracing::info!("good night");
            ExitCode::SUCCESS
        }
        Err(err) => {
            tracing::error!("did shutdown: {err:?}");
            tracing::error!("good night anyways");
            ExitCode::FAILURE
        }
    }
}
