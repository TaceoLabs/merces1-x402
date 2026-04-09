use alloy::{
    network::EthereumWallet,
    primitives::{Address, Bytes, U256},
    providers::DynProvider,
    signers::{Signer, local::PrivateKeySigner},
};
use client::transfer_compressed::TransferCompressed;
use contract_rs::{merces::MercesContract, token::USDCTokenContract};
use groth16_material::circom::CircomGroth16Material;
use rand::thread_rng;
use secrecy::{ExposeSecret, SecretString};
use std::sync::Arc;

use crate::keys::PublicKeys;

pub struct User {
    provider: DynProvider,
    signer: Address,
    /// Raw local signer, kept so the user can produce EIP-712 signatures for facilitator-submitted
    /// transferFrom flows.
    key_signer: PrivateKeySigner,
    proving_key: Arc<CircomGroth16Material>,
}

impl User {
    pub async fn from_wallet(
        rpc: &SecretString,
        wallet: EthereumWallet,
        key_signer: PrivateKeySigner,
        proving_key: Arc<CircomGroth16Material>,
    ) -> eyre::Result<Self> {
        let signer = wallet.default_signer().address();
        let provider = crate::connect_rpc(rpc.expose_secret(), wallet)
            .await
            .expect("Failed to connect to RPC");
        Ok(Self {
            provider,
            signer,
            key_signer,
            proving_key,
        })
    }

    pub fn get_provider(&self) -> &DynProvider {
        &self.provider
    }

    pub fn get_signer(&self) -> Address {
        self.signer
    }

    pub async fn get_balance(&self, token: &Option<USDCTokenContract>) -> eyre::Result<U256> {
        match token {
            Some(token_contract) => {
                crate::get_erc20_balance(&self.provider, token_contract, self.signer).await
            }
            None => crate::get_native_balance(&self.provider, self.signer).await,
        }
    }

    pub async fn deposit(
        &self,
        amount: U256,
        merces_contract: &MercesContract,
        token_contract: &Option<USDCTokenContract>,
    ) -> eyre::Result<usize> {
        let native_amount = match token_contract {
            Some(contract) => {
                contract
                    .approve(&self.provider, merces_contract.contract_address, amount)
                    .await?;
                Default::default()
            }
            None => amount,
        };
        let res = merces_contract
            .deposit(&self.provider, amount, native_amount)
            .await?;
        Ok(res.0)
    }

    pub async fn withdraw(
        &self,
        amount: U256,
        merces_contract: &MercesContract,
    ) -> eyre::Result<usize> {
        let res = merces_contract.withdraw(&self.provider, amount).await?;
        Ok(res.0)
    }

    pub async fn transfer(
        &self,
        amount: U256,
        recipient: Address,
        mpc_pks: PublicKeys,
        merces_contract: &MercesContract,
    ) -> eyre::Result<usize> {
        let mut rng = thread_rng();
        let mut transfer = TransferCompressed::new(
            contract_rs::u256_to_field(amount)?,
            mpc_pks.into(),
            &mut rng,
        );
        let onchain_transfer = transfer.compute_alpha();

        let (proof, public_inputs) = transfer.generate_proof(&self.proving_key, &mut rng)?;
        let beta = public_inputs[0];
        self.proving_key.verify_proof(&proof, &public_inputs)?;

        let res = merces_contract
            .transfer(
                &self.provider,
                recipient,
                onchain_transfer.amount_commitment,
                onchain_transfer.ciphertexts,
                onchain_transfer.sender_pk,
                beta,
                proof,
            )
            .await?;
        Ok(res.0)
    }

    /// Builds a signed `transferFrom` authorization ready for a facilitator to submit on-chain.
    ///
    /// The user generates the ZK proof, computes the ciphertext + commitment, and then signs the
    /// EIP-712 `TransferFromAuthorization` with its private key. The resulting `SignedTransferFrom`
    /// bundles everything the facilitator needs to call `MercesContract::transfer_from`.
    pub async fn authorize_transfer_from(
        &self,
        amount: U256,
        recipient: Address,
        mpc_pks: PublicKeys,
        merces_contract: &MercesContract,
        chain_id: u64,
        nonce: U256,
        deadline: U256,
    ) -> eyre::Result<SignedTransferFrom> {
        let mut rng = thread_rng();
        let mut transfer = TransferCompressed::new(
            contract_rs::u256_to_field(amount)?,
            mpc_pks.into(),
            &mut rng,
        );
        let onchain_transfer = transfer.compute_alpha();
        let (proof, public_inputs) = transfer.generate_proof(&self.proving_key, &mut rng)?;
        let beta = public_inputs[0];
        self.proving_key.verify_proof(&proof, &public_inputs)?;

        // Compute the EIP-712 digest the same way the contract does.
        let digest = MercesContract::transfer_from_signing_hash(
            chain_id,
            merces_contract.contract_address,
            self.signer,
            recipient,
            onchain_transfer.amount_commitment,
            onchain_transfer.ciphertexts,
            onchain_transfer.sender_pk,
            beta,
            nonce,
            deadline,
        );

        // Sign the digest with the user's private key.
        let signature = self.key_signer.sign_hash(&digest).await?;
        let signature_bytes = Bytes::from(signature.as_bytes().to_vec());

        Ok(SignedTransferFrom {
            sender: self.signer,
            receiver: recipient,
            amount_commitment: onchain_transfer.amount_commitment,
            ciphertexts: onchain_transfer.ciphertexts,
            sender_pk: onchain_transfer.sender_pk,
            beta,
            proof,
            nonce,
            deadline,
            signature: signature_bytes,
        })
    }
}

/// Everything a facilitator needs to submit a transferFrom on behalf of a user.
/// Produced by `User::authorize_transfer_from` and consumed by `MercesContract::transfer_from`.
pub struct SignedTransferFrom {
    pub sender: Address,
    pub receiver: Address,
    pub amount_commitment: ark_bn254::Fr,
    pub ciphertexts: [[ark_bn254::Fr; 2]; 3],
    pub sender_pk: ark_babyjubjub::EdwardsAffine,
    pub beta: ark_bn254::Fr,
    pub proof: ark_groth16::Proof<ark_bn254::Bn254>,
    pub nonce: U256,
    pub deadline: U256,
    pub signature: Bytes,
}
