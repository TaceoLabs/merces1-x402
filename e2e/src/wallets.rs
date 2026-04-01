use alloy::{network::EthereumWallet, signers::local::PrivateKeySigner};
use secrecy::{ExposeSecret, SecretString};
use std::str::FromStr;

pub struct Wallets {
    pub wallets: Vec<EthereumWallet>,
}

impl Wallets {
    pub fn new_anvil(num: usize) -> eyre::Result<Self> {
        if num > crate::ANVIL_SKS.len() {
            eyre::bail!(
                "Requested number of wallets ({num}) exceeds available Anvil accounts ({})",
                crate::ANVIL_SKS.len()
            );
        }

        let mut wallets = Vec::with_capacity(num);
        for i in 0..num {
            let key = PrivateKeySigner::from_str(crate::ANVIL_SKS[i])?;
            let wallet = EthereumWallet::from(key);
            wallets.push(wallet);
        }
        Ok(Self { wallets })
    }

    pub fn from_strings(secret_keys: Vec<SecretString>) -> eyre::Result<Self> {
        let mut wallets = Vec::with_capacity(secret_keys.len());
        for key in secret_keys {
            let key = PrivateKeySigner::from_str(key.expose_secret())?;
            let wallet = EthereumWallet::from(key);
            wallets.push(wallet);
        }
        Ok(Self { wallets })
    }
}
