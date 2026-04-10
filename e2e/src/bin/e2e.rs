use alloy::{
    primitives::U256,
    providers::{DynProvider, Provider},
};
use clap::Parser;
use contract_rs::{merces::MercesContract, token::USDCTokenContract};
use e2e::{
    SEED, deployer::Deployer, mpc::Mpc, proving_keys::ProvingKeys, user::User, wallets::Wallets,
};
use rand::SeedableRng;
use rand_chacha::ChaCha12Rng;
use secrecy::{ExposeSecret, SecretString};
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
    let wallets = Wallets::new_anvil(5)?;

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
        wallets.signers[2].clone(),
        proving_keys.client.clone(),
    )
    .await?;
    let bob = User::from_wallet(
        &anvil_rpc,
        wallets.wallets[3].clone(),
        wallets.signers[3].clone(),
        proving_keys.client.clone(),
    )
    .await?;
    // Facilitator: a third-party EOA that submits transferFrom on behalf of a client.
    // It pays gas and is msg.sender, but the client authorizes via EIP-712 signature.
    let facilitator_provider: DynProvider = e2e::connect_rpc_public(anvil_rpc.expose_secret(), wallets.wallets[4].clone()).await?;

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

    run_test(
        [alice, bob],
        deployer,
        mpc,
        facilitator_provider,
        contracts.0,
        contracts.1,
    )
    .await?;

    Ok(ExitCode::SUCCESS)
}

async fn run_test(
    users: [User; 2],
    deployer: Deployer,
    mut mpc: Mpc,
    facilitator_provider: DynProvider,
    merces_contract: MercesContract,
    token_contract: Option<USDCTokenContract>,
) -> eyre::Result<()> {
    testcase_inner(
        &users,
        &deployer,
        &mut mpc,
        &merces_contract,
        &token_contract,
    )
    .await?;

    testcase_inner(
        &users,
        &deployer,
        &mut mpc,
        &merces_contract,
        &token_contract,
    )
    .await?;

    testcase_transfer_from(
        &users,
        &deployer,
        &mut mpc,
        &facilitator_provider,
        &merces_contract,
        &token_contract,
    )
    .await?;

    Ok(())
}

async fn testcase_inner(
    users: &[User; 2],
    deployer: &Deployer,
    mpc: &mut Mpc,
    merces_contract: &MercesContract,
    token_contract: &Option<USDCTokenContract>,
) -> eyre::Result<()> {
    tracing::info!("\nRunning Testcase...");
    let decimals = e2e::get_decimals(mpc.get_provider(), token_contract).await?;

    if let Some(token_contract) = token_contract {
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

    let alice_balance = users[0].get_balance(token_contract).await?;
    let bob_balance = users[1].get_balance(token_contract).await?;

    e2e::check_action_queue_size(merces_contract, mpc.get_provider(), 0).await?;

    tracing::info!("Alice deposits on chain...");
    users[0]
        .deposit(amount, merces_contract, token_contract)
        .await?;

    tracing::info!("Alice transfers to Bob on chain...");
    users[0]
        .transfer(
            amount,
            users[1].get_signer(),
            mpc.public_keys(),
            merces_contract,
        )
        .await?;

    tracing::info!("Bob withdraws on chain...");
    users[1].withdraw(amount, merces_contract).await?;

    tracing::info!("Alice sends again to bob, this should fail...");
    users[0]
        .transfer(
            amount,
            users[1].get_signer(),
            mpc.public_keys(),
            merces_contract,
        )
        .await?;

    e2e::check_action_queue_size(merces_contract, mpc.get_provider(), 4).await?;

    tracing::info!("Processing the batch in MPC...");
    mpc.process_mpc(merces_contract).await?;

    e2e::check_action_queue_size(merces_contract, mpc.get_provider(), 0).await?;

    tracing::info!("Comparing balances with expected changes...");

    let alice_balance_ = users[0].get_balance(token_contract).await?;
    let bob_balance_ = users[1].get_balance(token_contract).await?;

    let diff_alice = alice_balance - alice_balance_;
    let diff_bob = bob_balance_ - bob_balance;
    e2e::cmp_balance_with_gas(diff_alice, amount, decimals)?;
    e2e::cmp_balance_with_gas(diff_bob, amount, decimals)?;

    Ok(())
}

/// Exercises the x402 facilitator-submitted `transferFrom` path.
/// Alice deposits, MPC processes the deposit, Alice authorizes a transferFrom to Bob via EIP-712,
/// the facilitator submits it, Bob withdraws, MPC processes the batch. Balances should match.
async fn testcase_transfer_from(
    users: &[User; 2],
    _deployer: &Deployer,
    mpc: &mut Mpc,
    facilitator_provider: &DynProvider,
    merces_contract: &MercesContract,
    token_contract: &Option<USDCTokenContract>,
) -> eyre::Result<()> {
    tracing::info!("\nRunning TransferFrom Testcase (x402 facilitator path)...");
    let decimals = e2e::get_decimals(mpc.get_provider(), token_contract).await?;
    let amount = e2e::amount_to_wei(U256::from(1), decimals);

    let alice_balance = users[0].get_balance(token_contract).await?;
    let bob_balance = users[1].get_balance(token_contract).await?;

    e2e::check_action_queue_size(merces_contract, mpc.get_provider(), 0).await?;

    tracing::info!("Alice deposits on chain...");
    users[0]
        .deposit(amount, merces_contract, token_contract)
        .await?;

    tracing::info!("Processing the deposit in MPC (so Alice has a confidential balance)...");
    mpc.process_mpc(merces_contract).await?;
    e2e::check_action_queue_size(merces_contract, mpc.get_provider(), 0).await?;

    tracing::info!("Alice signs a transferFrom authorization for Bob...");
    let chain_id = facilitator_provider.get_chain_id().await?;
    let deadline = U256::from(u64::MAX); // effectively never expires for the test
    let nonce = U256::from(1u64);
    let signed = users[0]
        .authorize_transfer_from(
            amount,
            users[1].get_signer(),
            mpc.public_keys(),
            merces_contract,
            chain_id,
            nonce,
            deadline,
        )
        .await?;

    tracing::info!("Facilitator submits transferFrom on behalf of Alice...");
    merces_contract
        .transfer_from(
            facilitator_provider,
            signed.sender,
            signed.receiver,
            signed.amount_commitment,
            signed.ciphertexts,
            signed.sender_pk,
            signed.beta,
            signed.proof,
            signed.nonce,
            signed.deadline,
            signed.signature,
        )
        .await?;

    tracing::info!("Bob withdraws on chain...");
    users[1].withdraw(amount, merces_contract).await?;

    e2e::check_action_queue_size(merces_contract, mpc.get_provider(), 2).await?;

    tracing::info!("Processing the transferFrom + withdraw batch in MPC...");
    mpc.process_mpc(merces_contract).await?;
    e2e::check_action_queue_size(merces_contract, mpc.get_provider(), 0).await?;

    tracing::info!("Comparing balances with expected changes...");
    let alice_balance_ = users[0].get_balance(token_contract).await?;
    let bob_balance_ = users[1].get_balance(token_contract).await?;

    // Alice deposited `amount`, so her balance decreased by at least `amount` (plus gas).
    // Bob withdrew `amount`, so his balance increased by `amount` (minus gas).
    let diff_alice = alice_balance - alice_balance_;
    let diff_bob = bob_balance_ - bob_balance;
    e2e::cmp_balance_with_gas(diff_alice, amount, decimals)?;
    e2e::cmp_balance_with_gas(diff_bob, amount, decimals)?;

    tracing::info!("TransferFrom testcase passed");
    Ok(())
}
