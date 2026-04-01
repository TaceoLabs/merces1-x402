use alloy::{primitives::Address, providers::DynProvider, sol};

sol!(
    #[sol(rpc)]
    Verifier,
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../contracts/json/VerifierServer.json"
    )
);

pub async fn deploy_contract(provider: &DynProvider) -> eyre::Result<Address> {
    let contract = Verifier::deploy(provider).await?;
    let address = contract.address().to_owned();
    tracing::info!("Deployed VerifierServer at {address:#x}");
    Ok(address)
}
