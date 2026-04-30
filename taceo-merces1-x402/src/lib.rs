use alloy::primitives::Address;

// #[cfg(feature = "server")]
pub mod server;
// #[cfg(feature = "server")]
pub use server::*;

// #[cfg(feature = "facilitator")]
pub mod facilitator;
// #[cfg(feature = "facilitator")]
pub use facilitator::*;

// #[cfg(feature = "client")]
pub mod client;
// #[cfg(feature = "client")]
pub use client::*;

pub mod types;
pub use types::*;

use x402_chain_eip155::chain::Eip155ChainReference;
use x402_types::{chain::ChainId, scheme::X402SchemeId};

pub struct V2Eip155Confidential;

impl X402SchemeId for V2Eip155Confidential {
    fn namespace(&self) -> &str {
        "eip155"
    }

    fn scheme(&self) -> &str {
        ConfidentialScheme.as_ref()
    }
}

pub struct ConfidentialUSDC;

impl ConfidentialUSDC {
    pub fn anvil() -> Eip155ConfidentialTokenDeployment {
        let chain_id = ChainId::new("eip155", "31337");
        let chain_reference =
            Eip155ChainReference::try_from(chain_id).expect("valid eip155 chain id");

        Eip155ConfidentialTokenDeployment {
            chain_reference,
            address: Address::default(), // TODO?
            decimals: 6,
        }
    }

    pub fn base_sepolia() -> Eip155ConfidentialTokenDeployment {
        let chain_id = ChainId::new("eip155", "84532");
        let chain_reference =
            Eip155ChainReference::try_from(chain_id).expect("valid eip155 chain id");

        Eip155ConfidentialTokenDeployment {
            chain_reference,
            address: Address::default(), // TODO?
            decimals: 6,
        }
    }
}
