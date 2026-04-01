use clap::Parser;
use e2e::{
    SEED, deployer::Deployer, mpc::Mpc, proving_keys::ProvingKeys, user::User, wallets::Wallets,
};
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;
use secrecy::SecretString;
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

    tracing::info!("Initializing wallets...");
    let wallets = Wallets::new_anvil(4)?;

    tracing::info!("Connecting providers...");
    let anvil_rpc: SecretString = e2e::ANVIL_RPC.to_string().into();
    let deployer = Deployer::from_wallet(&anvil_rpc, wallets.wallets[0].clone()).await?;
    let mpc = Mpc::from_wallet(
        &anvil_rpc,
        wallets.wallets[1].clone(),
        proving_keys.server.clone(),
        &mut rng,
    )
    .await?;
    let alice = User::from_wallet(
        &anvil_rpc,
        wallets.wallets[2].clone(),
        proving_keys.client.clone(),
    )
    .await?;
    let bob = User::from_wallet(
        &anvil_rpc,
        wallets.wallets[3].clone(),
        proving_keys.client.clone(),
    )
    .await?;

    tracing::info!("Deploying contracts...");
    let deploytoken = if cli.erc20_token {
        contract_rs::DeployToken::ERC20
    } else {
        contract_rs::DeployToken::Native
    };
    deployer
        .deploy(mpc.get_signer(), mpc.public_keys(), deploytoken)
        .await?;

    Ok(ExitCode::SUCCESS)
}
