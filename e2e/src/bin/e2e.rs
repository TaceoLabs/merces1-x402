use alloy::primitives::U256;
use clap::Parser;
use contract_rs::{merces::MercesContract, token::USDCTokenContract};
use e2e::{
    SEED,
    deployer::{self, Deployer},
    mpc::Mpc,
    proving_keys::ProvingKeys,
    user::User,
    wallets::Wallets,
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
    let contracts = deployer
        .deploy(mpc.get_signer(), mpc.public_keys(), deploytoken)
        .await?;
    if cli.erc20_token != contracts.1.is_some() {
        tracing::error!("ERC20 token deployment mismatch");
        return Ok(ExitCode::FAILURE);
    }

    run_test([alice, bob], deployer, mpc, contracts.0, contracts.1).await?;

    Ok(ExitCode::SUCCESS)
}

async fn run_test(
    users: [User; 2],
    deployer: Deployer,
    mpc: Mpc,
    merces_contract: MercesContract,
    token_contract: Option<USDCTokenContract>,
) -> eyre::Result<()> {
    tracing::info!("\nRunning Testcase...");
    let decimals = e2e::get_decimals(mpc.get_provider(), &token_contract).await?;

    if let Some(token_contract) = &token_contract {
        tracing::info!("Deployer sending tokens to Alice and Bob...");
        let init_amount = e2e::amount_to_wei(U256::from(10), decimals);
        deployer
            .send_tokens(token_contract, users[0].get_signer(), init_amount)
            .await?;
        deployer
            .send_tokens(token_contract, users[1].get_signer(), init_amount)
            .await?;
        tracing::info!("");
    }

    let amount = e2e::amount_to_wei(U256::from(1), decimals);

    let alice_balance = users[0].get_balance(&token_contract).await?;
    let bob_balance = users[1].get_balance(&token_contract).await?;

    e2e::check_action_queue_size(&merces_contract, mpc.get_provider(), 0).await?;

    tracing::info!("Alice deposits on chain...");
    users[0]
        .deposit(amount, &merces_contract, &token_contract)
        .await?;

    tracing::info!("Alice transfers to Bob on chain...");
    users[0]
        .transfer(
            amount,
            users[1].get_signer(),
            mpc.public_keys(),
            &merces_contract,
        )
        .await?;

    tracing::info!("Bob withdraws on chain...");
    users[1].withdraw(amount, &merces_contract).await?;

    tracing::info!("Alice sends again to bob, this should fail...");
    users[0]
        .transfer(
            amount,
            users[1].get_signer(),
            mpc.public_keys(),
            &merces_contract,
        )
        .await?;

    e2e::check_action_queue_size(&merces_contract, mpc.get_provider(), 4).await?;

    todo!("Process MPC");

    e2e::check_action_queue_size(&merces_contract, mpc.get_provider(), 0).await?;

    tracing::info!("Comparing balances with expected changes...");

    let alice_balance_ = users[0].get_balance(&token_contract).await?;
    let bob_balance_ = users[1].get_balance(&token_contract).await?;

    let diff_alice = alice_balance - alice_balance_;
    let diff_bob = bob_balance_ - bob_balance;
    e2e::cmp_balance_with_gas(diff_alice, amount, decimals)?;
    e2e::cmp_balance_with_gas(diff_bob, amount, decimals)?;
    Ok(())
}
