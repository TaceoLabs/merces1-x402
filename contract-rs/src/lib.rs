pub mod environment;
pub mod merces;
pub mod token;
pub mod verifiers;

use crate::{merces::MercesContract, token::USDCTokenContract};
use alloy::{
    hex,
    primitives::{Address, Bytes, TxKind, U256},
    providers::{DynProvider, Provider},
    rpc::types::TransactionRequest,
};
use ark_ff::PrimeField;
use eyre::{Context, ContextCompat};

pub struct DecodedCiphertext {
    pub amount: ark_bn254::Fr,
    pub amount_r: ark_bn254::Fr,
    pub sender_pk: ark_babyjubjub::EdwardsAffine,
}

pub fn u256_to_usize(value: U256) -> eyre::Result<usize> {
    let max = U256::from(usize::MAX);
    if value > max {
        eyre::bail!("U256 value is too large to fit into a usize");
    }

    let low = value.into_limbs()[0];
    let usize_value = low as usize;

    Ok(usize_value)
}

pub fn u256_to_field(value: U256) -> eyre::Result<ark_bn254::Fr> {
    let limbs = value.into_limbs();
    let bigint = <ark_bn254::Fr as PrimeField>::BigInt::new(limbs);
    ark_bn254::Fr::from_bigint(bigint)
        .ok_or_else(|| eyre::eyre!("U256 value is out of field range"))
}

pub fn bn254_fr_to_u256(field: ark_bn254::Fr) -> U256 {
    U256::from_limbs(field.into_bigint().0)
}

/// Links a library to bytecode hex string and returns the hex string (no decoding).
///
/// Use this when you need to link multiple libraries before decoding.
fn link_bytecode_hex(
    json: &str,
    bytecode_str: &str,
    library_path: &str,
    library_address: Address,
) -> eyre::Result<String> {
    let json: serde_json::Value = serde_json::from_str(json)?;
    let link_refs = &json["bytecode"]["linkReferences"];
    let (file_path, library_name) = library_path
        .split_once(':')
        .context("library_path must be in format 'file:Library'")?;

    let references = link_refs
        .get(file_path)
        .and_then(|v| v.get(library_name))
        .and_then(|v| v.as_array())
        .context("library reference not found")?;

    // Format library address as 40-character hex (20 bytes, no 0x prefix)
    let lib_addr_hex = format!("{library_address:040x}");

    let mut linked_bytecode = bytecode_str.to_string();

    // Process all references in reverse order to maintain correct positions
    let mut refs: Vec<_> = references
        .iter()
        .filter_map(|r| {
            let start = r["start"].as_u64()? as usize * 2; // byte offset -> hex offset
            Some(start)
        })
        .collect();
    refs.sort_by(|a, b| b.cmp(a)); // Sort descending

    for start_pos in refs {
        if start_pos + 40 <= linked_bytecode.len() {
            linked_bytecode.replace_range(start_pos..start_pos + 40, &lib_addr_hex);
        }
    }

    Ok(linked_bytecode)
}

pub async fn deploy_poseidon2(provider: &DynProvider) -> eyre::Result<Address> {
    // Deploy Poseidon2 library (no dependencies)
    let poseidon2_json = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../contracts/json/Poseidon2.json"
    ));
    let json_value: serde_json::Value = serde_json::from_str(poseidon2_json)?;
    let bytecode_str = json_value["bytecode"]["object"]
        .as_str()
        .context("bytecode not found in JSON")?
        .strip_prefix("0x")
        .unwrap_or_else(|| {
            json_value["bytecode"]["object"]
                .as_str()
                .expect("bytecode should be a string")
        })
        .to_string();
    let poseidon2_bytecode = Bytes::from(hex::decode(bytecode_str)?);

    let poseidon2_address = deploy_contract(provider, poseidon2_bytecode, Bytes::new())
        .await
        .context("failed to deploy Poseidon2 library")?;
    tracing::info!("Deployed Poseidon2 library at {poseidon2_address:#x}");
    Ok(poseidon2_address)
}

pub async fn deploy_babyjubjub(provider: &DynProvider) -> eyre::Result<Address> {
    // Deploy babyjubjub library (no dependencies)
    let action_vector_json = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../contracts/json/BabyJubJub.json"
    ));
    let json_value: serde_json::Value = serde_json::from_str(action_vector_json)?;
    let bytecode_str = json_value["bytecode"]["object"]
        .as_str()
        .context("bytecode not found in JSON")?
        .strip_prefix("0x")
        .unwrap_or_else(|| {
            json_value["bytecode"]["object"]
                .as_str()
                .expect("bytecode should be a string")
        })
        .to_string();
    let action_vector_bytecode = Bytes::from(hex::decode(bytecode_str)?);

    let action_vector_address = deploy_contract(provider, action_vector_bytecode, Bytes::new())
        .await
        .context("failed to deploy BabyJubJub library")?;
    tracing::info!("Deployed BabyJubJub library at {action_vector_address:#x}");
    Ok(action_vector_address)
}

async fn deploy_contract(
    provider: &DynProvider,
    bytecode: Bytes,
    constructor_args: Bytes,
) -> eyre::Result<Address> {
    let mut deployment_bytecode = bytecode.to_vec();
    deployment_bytecode.extend_from_slice(&constructor_args);

    let tx = TransactionRequest {
        to: Some(TxKind::Create),
        input: deployment_bytecode.into(),
        ..Default::default()
    };

    let pending_tx = provider.send_transaction(tx).await?;
    let receipt = pending_tx.get_receipt().await?;

    receipt
        .contract_address
        .context("contract deployment failed - no address in receipt")
}

#[derive(Clone, Debug, Copy)]
pub enum DeployToken {
    Native,
    ERC20,
}

pub async fn deploy(
    provider: &DynProvider,
    mpc_address: Address,
    mpc_pk1: ark_babyjubjub::EdwardsAffine,
    mpc_pk2: ark_babyjubjub::EdwardsAffine,
    mpc_pk3: ark_babyjubjub::EdwardsAffine,
    token: DeployToken,
    initial_supply: U256,
) -> eyre::Result<(MercesContract, Option<USDCTokenContract>)> {
    let client_verifier_address = verifiers::client::deploy_contract(provider).await?;
    let server_verifier_address = verifiers::server::deploy_contract(provider).await?;

    let poseidon2_address = deploy_poseidon2(provider).await?;
    let babyjubjub_address = deploy_babyjubjub(provider).await?;

    let token = match token {
        DeployToken::Native => {
            tracing::info!("Deploying with native token");
            None
        }
        DeployToken::ERC20 => {
            tracing::info!("Deploying with ERC20 token");
            let address = USDCTokenContract::deploy(provider, initial_supply).await?;
            Some(address)
        }
    };

    let merces = MercesContract::deploy(
        provider,
        client_verifier_address,
        server_verifier_address,
        poseidon2_address,
        babyjubjub_address,
        token
            .as_ref()
            .map(|x| x.contract_address)
            .unwrap_or_default(),
        mpc_address,
        mpc_pk1,
        mpc_pk2,
        mpc_pk3,
    )
    .await?;

    Ok((merces, token))
}
