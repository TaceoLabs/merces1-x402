//! CLI entry point for the Merces2 client.
//!
//! This binary wires configuration, key material, and a signer into
//! [`taceo_merces2_client::Client`], then executes one command per invocation.
//! Amounts are accepted in ether-style decimal strings and converted to wei.

use std::str::FromStr;

use alloy::{
    primitives::{Address, ruint::aliases::U256},
    signers::local::PrivateKeySigner,
};
use clap::Parser;
use contract_rs::amount_to_wei;
use eyre::Context;
use groth16_material::circom::CircomGroth16MaterialBuilder;
use secrecy::ExposeSecret as _;
use taceo_merces1_client::{Client, config::Merces1ClientConfig};

/// Supported CLI operations.
///
/// Transaction commands submit either private or confidential operations.
/// Query commands read history, balances, or index-to-address mappings.
#[derive(Debug, Parser)]
pub enum Command {
    /// Performs a deposit for the configured signer.
    Deposit {
        /// The amount to deposit
        #[clap(long)]
        amount: U256,
    },
    /// Performs a withdrawal for the configured signer.
    Withdraw {
        /// The amount to withdraw
        #[clap(long)]
        amount: U256,
    },
    /// Performs a transfer from the configured signer to `receiver`.
    Transfer {
        /// The receiver address
        #[clap(long)]
        receiver: Address,

        /// The amount to transfer
        #[clap(long)]
        amount: U256,
    },
    /// Retrieves the reconstructed current balance for an address.
    GetBalance {
        /// The user address
        #[clap(long)]
        address: Address,
    },
}

/// Top-level CLI options.
#[derive(Debug, Parser)]
pub struct Merces2ClientCli {
    /// Network and authentication configuration.
    #[clap(flatten)]
    pub config: Merces1ClientConfig,

    /// The command to execute
    #[clap(subcommand)]
    pub command: Command,
}

/// Executes the requested client command.
///
/// The function initializes tracing, loads Groth16 proving material, builds a
/// [`Client`] from CLI configuration, and dispatches to the selected command.
#[tokio::main]
async fn main() -> eyre::Result<()> {
    // we panic if we cannot setup tracing + TLS - if that fails we won't see anything anyways on tracing endpoint
    rustls::crypto::aws_lc_rs::default_provider()
        .install_default()
        .expect("Can install");
    taceo_nodes_observability::install_tracing("info");

    let cli = Merces2ClientCli::parse();
    tracing::info!("config: {cli:#?}");
    let dir = env!("CARGO_MANIFEST_DIR");
    let zkey_path = format!("{dir}/../circom/artifacts/client.arks.zkey");
    let graph_path = format!("{dir}/../circom/graph/client_graph.bin");
    let groth16_material = CircomGroth16MaterialBuilder::new()
        .bbf_num_2_bits_helper()
        .bbf_inv()
        .build_from_paths(zkey_path, graph_path)?;
    let signer = PrivateKeySigner::from_str(cli.config.private_key.expose_secret())
        .context("while parsing private key")?;
    let client = Client::new(cli.config, groth16_material, signer)
        .await
        .context("while creating client")?;
    let mut rng = rand::thread_rng();
    let decimals = client
        .decimals()
        .await
        .context("while fetching multiplier")?;

    match cli.command {
        Command::Deposit { amount } => {
            tracing::info!("Deposit with {amount}");
            client
                .deposit(amount_to_wei(amount, decimals))
                .await
                .context("while private depositing")?;
            tracing::info!("Deposit successful!");
        }
        Command::Withdraw { amount } => {
            tracing::info!("Withdraw with {amount}");
            client
                .withdraw(amount_to_wei(amount, decimals))
                .await
                .context("while private withdrawing")?;
            tracing::info!("Withdraw successful!");
        }
        Command::Transfer { receiver, amount } => {
            tracing::info!("Transfer with amount {amount} to receiver {receiver}");
            client
                .transfer(receiver, amount_to_wei(amount, decimals), &mut rng)
                .await
                .context("while private transferring")?;
            tracing::info!("Transfer successful!");
        }
        Command::GetBalance { address } => {
            tracing::info!("Get balance of {address}...");
            let balance = client
                .get_balance(address)
                .await
                .context("while getting balance")?;
            tracing::info!(
                "Balance is {}",
                alloy::primitives::utils::format_units(balance, decimals)
                    .expect("can format balance")
            );
        }
    }

    Ok(())
}
