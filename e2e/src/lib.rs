pub mod deployer;
pub mod keys;
pub mod mpc;
pub mod proving_keys;
pub mod user;
pub mod wallets;

use crate::proving_keys::ProvingKeys;
use alloy::{
    consensus::constants::ETH_TO_WEI,
    network::EthereumWallet,
    primitives::{Address, U256},
    providers::{DynProvider, Provider, ProviderBuilder},
};
use ark_ff::PrimeField;
use contract_rs::{merces::MercesContract, token::USDCTokenContract};
use eyre::Context;
use groth16_sol::SolidityVerifierConfig;
use mpc_nodes::map::{DepositValue, DepositValuePlain, PrivateDeposit};
use rand::{CryptoRng, Rng};

pub const SEED: &str = "Solidity_MERCES1";
pub const ROOT: &str = std::env!("CARGO_MANIFEST_DIR");
pub const SOLIDITY_PATH: &str = "/../contracts/src/verifiers/";

const TOKEN_SUPPLY: u128 = 10_000_000 * ETH_TO_WEI;
pub const ANVIL_RPC: &str = "http://127.0.0.1:8545";
pub const ANVIL_SKS: [&str; 10] = [
    "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
    "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d",
    "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a",
    "0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6",
    "0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a",
    "0x8b3a350cf5c34c9194ca85829a2df0ec3153be0318b5e2d3348e872092edffba",
    "0x92db14e403b83dfe3df233f83dfa3a0d7096f21ca9b0d6d6b8d88b2b4ec1564e",
    "0x4bbbf85ce3377467afe5d46f804f221813b2bb87f24d81f60f1fcdbf7cbf4356",
    "0xdbda1821b80551c9d65939329250298aa3472ba22feea921c0cf5d620ea67b97",
    "0x2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6",
];

pub async fn connect_rpc(rpc: &str, wallet: EthereumWallet) -> eyre::Result<DynProvider> {
    let provider = ProviderBuilder::new()
        .wallet(wallet)
        .connect(rpc)
        .await
        .context("while connecting to RPC")?
        .erased();
    Ok(provider)
}

pub fn write_solidity_verifiers(keys: &ProvingKeys) -> eyre::Result<()> {
    let config = SolidityVerifierConfig::default();
    keys.write_solidity_verifiers(config)?;
    Ok(())
}

async fn get_native_balance(provider: &DynProvider, address: Address) -> eyre::Result<U256> {
    Ok(provider.get_balance(address).await?)
}

async fn get_erc20_balance(
    provider: &DynProvider,
    contract: &USDCTokenContract,
    address: Address,
) -> eyre::Result<U256> {
    contract.balance_of(provider, address).await
}

pub async fn get_decimals(
    providers: &DynProvider,
    token: &Option<USDCTokenContract>,
) -> eyre::Result<u8> {
    match token {
        None => Ok(18),
        Some(token_contract) => token_contract.decimals(providers).await,
    }
}

pub async fn check_action_queue_size(
    contract: &MercesContract,
    provider: &DynProvider,
    expected: usize,
) -> eyre::Result<()> {
    let num_items = contract.get_queue_size(provider).await?;
    if num_items != expected {
        eyre::bail!("Expected action queue to have {expected} items, found {num_items}");
    }
    Ok(())
}

pub fn cmp_balance_with_gas(diff: U256, expected_change: U256, decimals: u8) -> eyre::Result<()> {
    let multiplier = 10u128.pow(decimals as u32);
    let cmp_diff = U256::from(multiplier / 1000); // Represents GAS cost

    if diff < expected_change - cmp_diff || diff > expected_change + cmp_diff {
        eyre::bail!(
            "Balance change {diff} is not within expected range [{}, {}]",
            expected_change - cmp_diff,
            expected_change + cmp_diff
        );
    }

    Ok(())
}

pub fn amount_to_wei(amount: U256, decimals: u8) -> U256 {
    amount * U256::from(10u128.pow(decimals as u32))
}

pub struct TestConfig {}

impl TestConfig {
    pub fn install_tracing() {
        use tracing_subscriber::fmt::format::FmtSpan;
        use tracing_subscriber::prelude::*;
        use tracing_subscriber::{EnvFilter, fmt};

        let fmt_layer = fmt::layer()
            .with_target(false)
            .with_line_number(false)
            .with_span_events(FmtSpan::CLOSE | FmtSpan::ENTER);
        let filter_layer = EnvFilter::try_from_default_env()
            .or_else(|_| EnvFilter::try_new("warn,bench=info"))
            .unwrap();

        tracing_subscriber::registry()
            .with(filter_layer)
            .with(fmt_layer)
            .init();
    }

    pub fn get_random_plain_map<F: PrimeField, R: Rng + CryptoRng>(
        num_items: usize,
        rng: &mut R,
    ) -> PrivateDeposit<F, DepositValuePlain<F>> {
        let mut map: PrivateDeposit<F, DepositValue<F>> = PrivateDeposit::with_capacity(num_items);
        for _ in 0..num_items {
            let key = F::rand(rng);
            let amount = F::from(rng.gen_range(0..u32::MAX)); // We don't use the full u64 range to avoid overflows in the testcases
            let blinding = F::rand(rng);
            // We don't check whether the key is already in the map since the probability is negligible
            map.insert(key, DepositValuePlain::new(amount, blinding));
        }
        assert_eq!(map.len(), num_items);
        map
    }
}
