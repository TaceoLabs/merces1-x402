use clap::Parser;
use e2e::{SEED, proving_keys::ProvingKeys};
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;
use std::process::ExitCode;

/// Cli arguments
#[derive(Debug, Parser)]
struct Cli {
    #[arg(short, long, default_value_t = false, action = clap::ArgAction::SetTrue)]
    pub write_solidity: bool,

    #[arg(short, long, default_value_t = false, action = clap::ArgAction::SetTrue)]
    pub erc20_token: bool,
}

pub fn install_tracing() {
    use tracing_subscriber::prelude::*;
    use tracing_subscriber::{
        EnvFilter,
        fmt::{self, format::FmtSpan},
    };

    let fmt_layer = fmt::layer()
        .with_target(false)
        .with_line_number(false)
        .with_span_events(FmtSpan::CLOSE | FmtSpan::ENTER);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("e2e=info,contract_rs=info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();
}

#[tokio::main]
async fn main() -> eyre::Result<ExitCode> {
    type R = ChaCha12Rng;

    let cli = Cli::parse();
    install_tracing();

    let mut seed = [0u8; 32];
    if SEED.len() > 32 {
        tracing::error!("Seed too long");
        return Ok(ExitCode::FAILURE);
    }
    seed[0..SEED.len()].copy_from_slice(SEED.as_bytes());
    let mut rng = R::from_seed(seed);

    tracing::info!("Loading proving keys...");
    let proving_keys = ProvingKeys::load(&mut rng)?;

    if cli.write_solidity {
        tracing::info!("Writing solidity verifiers...");
        e2e::write_solidity_verifiers(&proving_keys)?;
    } else {
        tracing::info!("Skipping writing solidity verifiers...");
    }

    Ok(ExitCode::SUCCESS)
}
