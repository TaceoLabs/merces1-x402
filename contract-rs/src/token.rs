use alloy::{
    primitives::{Address, U256},
    providers::DynProvider,
    rpc::types::TransactionReceipt,
    sol,
};
use eyre::Context;

// Codegen from ABI file to interact with the contract.
sol!(
    #[sol(rpc)]
    USDCToken,
    concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../contracts/json/USDCToken.json"
    )
);

#[derive(Debug, Clone)]
pub struct USDCTokenContract {
    pub contract_address: Address,
}

impl USDCTokenContract {
    pub fn new(contract_address: Address) -> Self {
        Self { contract_address }
    }

    pub async fn deploy(provider: &DynProvider, initial_supply: U256) -> eyre::Result<Self> {
        let contract = USDCToken::deploy(provider, initial_supply).await?;
        let address = contract.address().to_owned();
        tracing::info!("Deployed USDCToken at {address:#x}");
        Ok(Self {
            contract_address: address,
        })
    }

    // Use U256 max value for unlimited approval
    pub async fn approve(
        &self,
        provider: &DynProvider,
        receiver: Address,
        amount: U256,
    ) -> eyre::Result<TransactionReceipt> {
        let contract = USDCToken::new(self.contract_address, provider);

        let receipt = contract
            .approve(receiver, amount)
            .send()
            .await
            .context("while broadcasting to network")?
            .get_receipt()
            .await
            .context("while registering watcher for transaction")?;

        if receipt.status() {
            tracing::info!(
                "approve done with transaction hash: {}",
                receipt.transaction_hash
            );
        } else {
            eyre::bail!("cannot finish approve: {receipt:?}");
        }

        Ok(receipt)
    }

    // Use U256 max value for unlimited approval
    pub async fn approve_with_sender(
        &self,
        provider: &DynProvider,
        from: Address,
        receiver: Address,
        amount: U256,
    ) -> eyre::Result<TransactionReceipt> {
        let contract = USDCToken::new(self.contract_address, provider);

        let receipt = contract
            .approve(receiver, amount)
            .from(from)
            .send()
            .await
            .context("while broadcasting to network")?
            .get_receipt()
            .await
            .context("while registering watcher for transaction")?;

        if receipt.status() {
            tracing::info!(
                "approve done with transaction hash: {}",
                receipt.transaction_hash
            );
        } else {
            eyre::bail!("cannot finish approve: {receipt:?}");
        }

        Ok(receipt)
    }
    pub async fn mint(
        &self,
        provider: &DynProvider,
        receiver: Address,
        amount: U256,
    ) -> eyre::Result<TransactionReceipt> {
        let contract = USDCToken::new(self.contract_address, provider);

        let receipt = contract
            .mint(receiver, amount)
            .send()
            .await
            .context("while broadcasting to network")?
            .get_receipt()
            .await
            .context("while registering watcher for transaction")?;

        if receipt.status() {
            tracing::info!(
                "mint done with transaction hash: {}",
                receipt.transaction_hash
            );
        } else {
            eyre::bail!("cannot finish mint: {receipt:?}");
        }

        Ok(receipt)
    }

    pub async fn transfer(
        &self,
        provider: &DynProvider,
        receiver: Address,
        amount: U256,
    ) -> eyre::Result<TransactionReceipt> {
        let contract = USDCToken::new(self.contract_address, provider);

        let receipt = contract
            .transfer(receiver, amount)
            .send()
            .await
            .context("while broadcasting to network")?
            .get_receipt()
            .await
            .context("while registering watcher for transaction")?;

        if receipt.status() {
            tracing::info!(
                "transfer done with transaction hash: {}",
                receipt.transaction_hash
            );
        } else {
            eyre::bail!("cannot finish transfer: {receipt:?}");
        }

        Ok(receipt)
    }

    pub async fn balance_of(&self, provider: &DynProvider, account: Address) -> eyre::Result<U256> {
        let contract = USDCToken::new(self.contract_address, provider);
        let balance = contract.balanceOf(account).call().await?;
        Ok(balance)
    }

    pub async fn decimals(&self, provider: &DynProvider) -> eyre::Result<u8> {
        let contract = USDCToken::new(self.contract_address, provider);
        let decimals = contract.decimals().call().await?;
        Ok(decimals)
    }

    pub async fn name(&self, provider: &DynProvider) -> eyre::Result<String> {
        let contract = USDCToken::new(self.contract_address, provider);
        let name = contract.name().call().await?;
        Ok(name)
    }
}
