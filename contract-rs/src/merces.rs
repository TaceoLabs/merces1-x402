use std::array;

use crate::{
    DecodedCiphertext,
    merces::Merces::{ActionItem, Ciphertext, MercesInstance},
};
use alloy::{
    hex,
    network::Ethereum,
    primitives::{Address, B256, Bytes, FixedBytes, Log, U256, keccak256},
    providers::{DynProvider, PendingTransactionBuilder, Provider},
    rpc::types::{Filter, TransactionReceipt},
    sol,
    sol_types::{
        Eip712Domain, SolCall, SolConstructor, SolEvent, SolStruct, SolValue, eip712_domain,
    },
};
use ark_bn254::Bn254;
use ark_ff::PrimeField;
use ark_groth16::Proof;
use eyre::{Context, ContextCompat};
use taceo_nodes_common::Environment;
use tracing::instrument;

// Codegen from ABI file to interact with the contract.
sol!(
    #[sol(rpc, ignore_unlinked)]
    #[allow(clippy::too_many_arguments)]
    Merces,
    concat!(env!("CARGO_MANIFEST_DIR"), "/../contracts/json/Merces.json")
);

// EIP-712 typed data struct for transferFrom — must match TRANSFER_FROM_TYPEHASH in Merces.sol.
// Used by the client to sign an authorization that a facilitator can later submit on their behalf.
sol! {
    struct TransferFromAuthorization {
        address sender;
        address receiver;
        uint256 amountCommitment;
        bytes32 ciphertextHash;
        uint256 beta;
        uint256 nonce;
        uint256 deadline;
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct MercesContract {
    pub contract_address: Address,
}

fn merces_interface_id() -> [u8; 4] {
    let mut id = [0u8; 4];

    for sel in [
        Merces::getNextActionIndexCall::SELECTOR,
        Merces::getQueueSizeCall::SELECTOR,
        Merces::processMPCCall::SELECTOR,
        Merces::readQueueCall::SELECTOR,
        Merces::retrieveFundsCall::SELECTOR,
    ] {
        for i in 0..4 {
            id[i] ^= sel[i];
        }
    }

    id
}

impl MercesContract {
    pub const BATCH_SIZE: usize = 50;

    /// EIP-712 domain matching the contract's constructor: EIP712("Merces", "1").
    pub fn eip712_domain(chain_id: u64, verifying_contract: Address) -> Eip712Domain {
        eip712_domain! {
            name: "Merces",
            version: "1",
            chain_id: chain_id,
            verifying_contract: verifying_contract,
        }
    }

    pub fn get_address(&self) -> Address {
        self.contract_address
    }

    #[instrument(level = "info", skip_all)]
    pub async fn contract_comp_check(
        &self,
        provider: &DynProvider,
        environment: Environment,
    ) -> eyre::Result<()> {
        let instance = MercesInstance::new(self.contract_address, provider);
        tracing::info!("checking contract address {}", self.contract_address);
        let domain_tag_preimage = format!("{environment}-merces2");
        tracing::info!("domain tag: {domain_tag_preimage}");
        instance
            .contractCompCheck(
                FixedBytes::from(merces_interface_id()),
                alloy::primitives::keccak256(domain_tag_preimage.as_bytes()),
            )
            .call()
            .await
            .context("while doing comp check")?;
        Ok(())
    }

    fn encode_babyjubjub(input: ark_babyjubjub::EdwardsAffine) -> BabyJubJub::Affine {
        BabyJubJub::Affine {
            x: U256::from_limbs(input.x.into_bigint().0),
            y: U256::from_limbs(input.y.into_bigint().0),
        }
    }

    fn decode_babyjubjub(input: BabyJubJub::Affine) -> eyre::Result<ark_babyjubjub::EdwardsAffine> {
        if input.x.is_zero() && input.y.is_zero() {
            // Handle the identity point case
            return Ok(ark_babyjubjub::EdwardsAffine::zero());
        }

        Ok(ark_babyjubjub::EdwardsAffine::new(
            crate::u256_to_field(input.x)?,
            crate::u256_to_field(input.y)?,
        ))
    }

    fn compress_proof(proof: &Proof<Bn254>) -> [U256; 4] {
        groth16_sol::prepare_compressed_proof(proof)
    }

    fn encode_ciphertext(
        ciphertexts: [[ark_bn254::Fr; 2]; 3],
        sender_pk: ark_babyjubjub::EdwardsAffine,
    ) -> Merces::Ciphertext {
        let amount = array::from_fn(|i| super::bn254_fr_to_u256(ciphertexts[i][0]));
        let r = array::from_fn(|i| super::bn254_fr_to_u256(ciphertexts[i][1]));
        Merces::Ciphertext {
            amount,
            r,
            senderPk: Self::encode_babyjubjub(sender_pk),
        }
    }

    pub fn decode_ciphertext(
        ciphertext: Merces::Ciphertext,
    ) -> eyre::Result<[DecodedCiphertext; 3]> {
        Ok([
            DecodedCiphertext {
                amount: crate::u256_to_field(ciphertext.amount[0])?,
                amount_r: crate::u256_to_field(ciphertext.r[0])?,
                sender_pk: Self::decode_babyjubjub(ciphertext.senderPk.clone())?,
            },
            DecodedCiphertext {
                amount: crate::u256_to_field(ciphertext.amount[1])?,
                amount_r: crate::u256_to_field(ciphertext.r[1])?,
                sender_pk: Self::decode_babyjubjub(ciphertext.senderPk.clone())?,
            },
            DecodedCiphertext {
                amount: crate::u256_to_field(ciphertext.amount[2])?,
                amount_r: crate::u256_to_field(ciphertext.r[2])?,
                sender_pk: Self::decode_babyjubjub(ciphertext.senderPk)?,
            },
        ])
    }

    #[expect(clippy::too_many_arguments)]
    pub async fn deploy(
        provider: &DynProvider,
        client_verifier_address: Address,
        server_verifier_address: Address,
        poseidon2_address: Address,
        babyjubjub_address: Address,
        action_queue_address: Address,
        token_address: Address,
        mpc_address: Address,
        mpc_pk1: ark_babyjubjub::EdwardsAffine,
        mpc_pk2: ark_babyjubjub::EdwardsAffine,
        mpc_pk3: ark_babyjubjub::EdwardsAffine,
    ) -> eyre::Result<Self> {
        // Link action_vector, babyjubjub and poseidon2 to Merces
        let json = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../contracts/json/Merces.json"
        ));
        let json_value: serde_json::Value = serde_json::from_str(json)?;
        let mut bytecode_str = json_value["bytecode"]["object"]
            .as_str()
            .context("bytecode not found in JSON")?
            .strip_prefix("0x")
            .unwrap_or_else(|| {
                json_value["bytecode"]["object"]
                    .as_str()
                    .expect("bytecode should be a string")
            })
            .to_string();

        bytecode_str = super::link_bytecode_hex(
            json,
            &bytecode_str,
            "src/Poseidon2.sol:Poseidon2T2_BN254",
            poseidon2_address,
        )?;

        bytecode_str = super::link_bytecode_hex(
            json,
            &bytecode_str,
            "lib/babyjubjub-solidity/src/BabyJubJub.sol:BabyJubJub",
            babyjubjub_address,
        )?;

        bytecode_str = super::link_bytecode_hex(
            json,
            &bytecode_str,
            "src/ActionQueue.sol:ActionQueueLib",
            action_queue_address,
        )?;

        // Decode the fully-linked bytecode
        let bytecode = Bytes::from(hex::decode(bytecode_str)?);

        let init_data = Bytes::from(
            Merces::constructorCall {
                _clientVerifier: client_verifier_address,
                _serverVerifier: server_verifier_address,
                _mpcAddress: mpc_address,
                _tokenAddress: token_address,
                _mpcPk1: Self::encode_babyjubjub(mpc_pk1),
                _mpcPk2: Self::encode_babyjubjub(mpc_pk2),
                _mpcPk3: Self::encode_babyjubjub(mpc_pk3),
                _environmentTag: format!("{}", Environment::Test),
            }
            .abi_encode(),
        );

        let address = super::deploy_contract(provider, bytecode, init_data)
            .await
            .context("failed to deploy Merces implementation")?;
        tracing::info!("Deployed Merces contract at {address:#x}");
        Ok(Self {
            contract_address: address,
        })
    }

    pub async fn retrieve_funds(
        &self,
        provider: &DynProvider,
        receiver: Address,
    ) -> eyre::Result<TransactionReceipt> {
        let contract = Merces::new(self.contract_address, provider);

        let receipt = contract
            .retrieveFunds(receiver)
            .send()
            .await
            .context("while broadcasting to network")?
            .get_receipt()
            .await
            .context("while receiving receipt for transaction")?;

        if receipt.status() {
            tracing::info!(
                "retrieve funds done with transaction hash: {}",
                receipt.transaction_hash
            );
        } else {
            eyre::bail!("cannot finish transaction: {receipt:?}");
        }

        Ok(receipt)
    }

    pub async fn get_queue_size(&self, provider: &DynProvider) -> eyre::Result<usize> {
        let contract = Merces::new(self.contract_address, provider);
        let size = contract
            .getQueueSize()
            .call()
            .await
            .context("while calling get_action_queue_size")?;

        crate::u256_to_usize(size)
    }

    pub async fn get_next_action_index(&self, provider: &DynProvider) -> eyre::Result<usize> {
        let contract = Merces::new(self.contract_address, provider);
        let index = contract
            .getNextActionIndex()
            .call()
            .await
            .context("while calling get_next_action_index")?;

        crate::u256_to_usize(index)
    }

    pub async fn read_queue(
        &self,
        num_items: usize,
        provider: &DynProvider,
    ) -> eyre::Result<(Vec<usize>, Vec<ActionItem>, Vec<Ciphertext>)> {
        let contract = Merces::new(self.contract_address, provider);
        let res = contract
            .readQueue(U256::from(num_items))
            .call()
            .await
            .context("while calling read_queue")?;

        if res._0.len() != res._1.len() {
            eyre::bail!("mismatched lengths in read_queue");
        }
        if res._0.len() != res._2.len() {
            eyre::bail!("mismatched lengths in read_queue");
        }
        let indices = res
            ._0
            .into_iter()
            .map(crate::u256_to_usize)
            .collect::<eyre::Result<Vec<usize>>>()?;

        Ok((indices, res._1, res._2))
    }

    pub async fn get_mpc_public_keys(
        &self,
        provider: &DynProvider,
    ) -> eyre::Result<(
        ark_babyjubjub::EdwardsAffine,
        ark_babyjubjub::EdwardsAffine,
        ark_babyjubjub::EdwardsAffine,
    )> {
        let contract = Merces::new(self.contract_address, provider);
        let result = contract
            .getMpcPublicKeys()
            .call()
            .await
            .context("while calling getMPCPublicKeys")?;

        Ok((
            Self::decode_babyjubjub(result._0)?,
            Self::decode_babyjubjub(result._1)?,
            Self::decode_babyjubjub(result._2)?,
        ))
    }

    pub async fn deposit(
        &self,
        provider: &DynProvider,
        amount: U256,
        native_amount: U256,
    ) -> eyre::Result<(usize, TransactionReceipt)> {
        let contract = Merces::new(self.contract_address, provider);

        let receipt = contract
            .deposit(amount)
            .value(native_amount)
            .send()
            .await
            .context("while broadcasting to network")?
            .get_receipt()
            .await
            .context("while receiving receipt for transaction")?;

        if receipt.status() {
            tracing::info!(
                "deposit done with transaction hash: {}",
                receipt.transaction_hash
            );
        } else {
            eyre::bail!("cannot finish transaction: {receipt:?}");
        }

        let result = receipt
            .decoded_log::<Merces::Deposit>()
            .ok_or_else(|| eyre::eyre!("no Deposit event found in transaction receipt logs"))?;
        let action_index = crate::u256_to_usize(result.actionIndex)?;

        Ok((action_index, receipt))
    }

    pub async fn withdraw(
        &self,
        provider: &DynProvider,
        amount: U256,
    ) -> eyre::Result<(usize, TransactionReceipt)> {
        let contract = Merces::new(self.contract_address, provider);

        let receipt = contract
            .withdraw(amount)
            .send()
            .await
            .context("while broadcasting to network")?
            .get_receipt()
            .await
            .context("while receiving receipt for transaction")?;

        if receipt.status() {
            tracing::info!(
                "withdraw done with transaction hash: {}",
                receipt.transaction_hash
            );
        } else {
            eyre::bail!("cannot finish transaction: {receipt:?}");
        }

        let result = receipt
            .decoded_log::<Merces::Withdraw>()
            .ok_or_else(|| eyre::eyre!("no Withdraw event found in transaction receipt logs"))?;
        let action_index = crate::u256_to_usize(result.actionIndex)?;

        Ok((action_index, receipt))
    }

    #[expect(clippy::too_many_arguments)]
    pub async fn transfer(
        &self,
        provider: &DynProvider,
        receiver: Address,
        amount_commitment: ark_bn254::Fr,
        ciphertexts: [[ark_bn254::Fr; 2]; 3],
        sender_pk: ark_babyjubjub::EdwardsAffine,
        beta: ark_bn254::Fr,
        proof: Proof<Bn254>,
    ) -> eyre::Result<(usize, TransactionReceipt)> {
        let contract = Merces::new(self.contract_address, provider);

        let ciphertext = Self::encode_ciphertext(ciphertexts, sender_pk);
        let amount_commitment = super::bn254_fr_to_u256(amount_commitment);
        let beta = super::bn254_fr_to_u256(beta);
        let proof = Self::compress_proof(&proof);

        let receipt = contract
            .transfer(receiver, amount_commitment, beta, ciphertext, proof)
            .send()
            .await
            .context("while broadcasting to network")?
            .get_receipt()
            .await
            .context("while receiving receipt for transaction")?;

        if receipt.status() {
            tracing::info!(
                "transfer done with transaction hash: {}",
                receipt.transaction_hash
            );
        } else {
            eyre::bail!("cannot finish transaction: {receipt:?}");
        }

        let result = receipt
            .decoded_log::<Merces::Transfer>()
            .ok_or_else(|| eyre::eyre!("no Transfer event found in transaction receipt logs"))?;
        let action_index = crate::u256_to_usize(result.actionIndex)?;

        Ok((action_index, receipt))
    }

    /// Computes the EIP-712 digest that the sender must sign to authorize a facilitator-submitted
    /// transferFrom. Returns the 32-byte digest ready to pass into `Signer::sign_hash`.
    ///
    /// Callers should construct the same `Ciphertext` they'll submit on-chain, hash it here with
    /// `abi.encode + keccak256` (matching `Merces.sol`), and then sign the returned digest.
    #[expect(clippy::too_many_arguments)]
    pub fn transfer_from_signing_hash(
        chain_id: u64,
        verifying_contract: Address,
        sender: Address,
        receiver: Address,
        amount_commitment: ark_bn254::Fr,
        ciphertexts: [[ark_bn254::Fr; 2]; 3],
        sender_pk: ark_babyjubjub::EdwardsAffine,
        beta: ark_bn254::Fr,
        nonce: U256,
        deadline: U256,
    ) -> B256 {
        let ciphertext = Self::encode_ciphertext(ciphertexts, sender_pk);
        // Match the contract: bytes32 ciphertextHash = keccak256(abi.encode(ciphertext));
        let ciphertext_hash = keccak256(ciphertext.abi_encode());

        let authorization = TransferFromAuthorization {
            sender,
            receiver,
            amountCommitment: super::bn254_fr_to_u256(amount_commitment),
            ciphertextHash: ciphertext_hash,
            beta: super::bn254_fr_to_u256(beta),
            nonce,
            deadline,
        };

        let domain = Self::eip712_domain(chain_id, verifying_contract);
        authorization.eip712_signing_hash(&domain)
    }

    /// Facilitator-submitted transfer. The `provider` is the facilitator's wallet-connected
    /// provider (i.e. it pays gas and is the `msg.sender`). The `signature` must be an EIP-712
    /// signature over the TransferFromAuthorization struct produced by `transfer_from_signing_hash`,
    /// signed by `sender`.
    #[expect(clippy::too_many_arguments)]
    pub async fn transfer_from<P: Provider>(
        &self,
        provider: &P,
        sender: Address,
        receiver: Address,
        amount_commitment: ark_bn254::Fr,
        ciphertexts: [[ark_bn254::Fr; 2]; 3],
        sender_pk: ark_babyjubjub::EdwardsAffine,
        beta: ark_bn254::Fr,
        proof: Proof<Bn254>,
        nonce: U256,
        deadline: U256,
        signature: Bytes,
    ) -> eyre::Result<(usize, TransactionReceipt)> {
        let contract = Merces::new(self.contract_address, provider);

        let ciphertext = Self::encode_ciphertext(ciphertexts, sender_pk);
        let amount_commitment = super::bn254_fr_to_u256(amount_commitment);
        let beta = super::bn254_fr_to_u256(beta);
        let proof = Self::compress_proof(&proof);

        let receipt = contract
            .transferFrom(
                sender,
                receiver,
                amount_commitment,
                beta,
                ciphertext,
                proof,
                nonce,
                deadline,
                signature,
            )
            .send()
            .await
            .context("while broadcasting transferFrom to network")?
            .get_receipt()
            .await
            .context("while receiving receipt for transferFrom")?;

        if receipt.status() {
            tracing::info!(
                "transferFrom done with transaction hash: {}",
                receipt.transaction_hash
            );
        } else {
            eyre::bail!("cannot finish transferFrom: {receipt:?}");
        }

        let result = receipt
            .decoded_log::<Merces::TransferFrom>()
            .ok_or_else(|| {
                eyre::eyre!("no TransferFrom event found in transferFrom receipt logs")
            })?;
        let action_index = crate::u256_to_usize(result.actionIndex)?;

        Ok((action_index, receipt))
    }

    pub async fn process_mpc(
        &self,
        provider: &DynProvider,
        num_transactions: usize,
        commitments: [U256; Self::BATCH_SIZE * 2],
        valid: [bool; Self::BATCH_SIZE],
        beta: ark_bn254::Fr,
        proof: Proof<Bn254>,
    ) -> eyre::Result<(Vec<usize>, TransactionReceipt)> {
        let contract = Merces::new(self.contract_address, provider);

        let beta = super::bn254_fr_to_u256(beta);
        let proof = Self::compress_proof(&proof);

        let receipt = contract
            .processMPC(
                U256::from(num_transactions),
                commitments,
                valid,
                beta,
                proof,
            )
            .send()
            .await
            .context("while broadcasting to network")?
            .get_receipt()
            .await
            .context("while receiving receipt for transaction")?;

        if receipt.status() {
            tracing::info!(
                "processMPC done with transaction hash: {}",
                receipt.transaction_hash
            );
        } else {
            eyre::bail!("cannot finish transaction: {receipt:?}");
        }

        let result = receipt
            .decoded_log::<Merces::ProcessedMPC>()
            .ok_or_else(|| {
                eyre::eyre!("no ProcessedMPC event found in transaction receipt logs")
            })?;
        let action_indices = result
            .actionIndices
            .into_iter()
            .take(num_transactions)
            .map(crate::u256_to_usize)
            .collect::<eyre::Result<Vec<usize>>>()?;

        Ok((action_indices, receipt))
    }

    pub async fn read_processed_mpc_events_since(
        &self,
        provider: &DynProvider,
        block: u64,
    ) -> eyre::Result<Vec<Log<Merces::ProcessedMPC>>> {
        let filter = Filter::new()
            .address(self.contract_address)
            .event_signature(Merces::ProcessedMPC::SIGNATURE_HASH)
            .from_block(block);
        let logs = provider.get_logs(&filter).await?;

        let mut logs_ = Vec::with_capacity(logs.len());

        for log in logs {
            let decoded_log = log.log_decode::<Merces::ProcessedMPC>()?;
            logs_.push(decoded_log.into_inner());
        }

        Ok(logs_)
    }

    pub async fn read_processed_mpc_events(
        &self,
        provider: &DynProvider,
        n_blocks: u64, // amount of latest blocks to read from
    ) -> eyre::Result<Vec<Log<Merces::ProcessedMPC>>> {
        let last_block = provider.get_block_number().await?;
        let from_block = last_block.saturating_sub(n_blocks);

        self.read_processed_mpc_events_since(provider, from_block)
            .await
    }

    pub async fn get_processed_mpc_event<P: Provider>(
        &self,
        provider: &P,
        from_block: u64,
        action_index: usize,
    ) -> eyre::Result<(usize, alloy::rpc::types::Log<Merces::ProcessedMPC>)> {
        let filter = Filter::new()
            .address(self.contract_address)
            .from_block(from_block)
            .event_signature(Merces::ProcessedMPC::SIGNATURE_HASH);
        loop {
            let logs = provider.get_logs(&filter).await?;

            for log in logs {
                let decoded = log.log_decode::<Merces::ProcessedMPC>()?;

                if let Some(pos) = decoded
                    .inner
                    .actionIndices
                    .iter()
                    .position(|i| *i == action_index)
                {
                    return Ok((pos, decoded));
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }
    }

    pub async fn get_action_index_and_receipt_from_pending(
        &self,
        pending: PendingTransactionBuilder<Ethereum>,
    ) -> eyre::Result<(usize, TransactionReceipt)> {
        let receipt = pending
            .get_receipt()
            .await
            .context("while receiving receipt for transaction")?;

        if receipt.status() {
            tracing::info!(
                "transaction done with transaction hash: {}",
                receipt.transaction_hash
            );
        } else {
            eyre::bail!("cannot finish transaction: {receipt:?}");
        }

        let log = receipt
            .logs()
            .iter()
            .find_map(|log| {
                log.topics().first().and_then(|topic0| {
                    if [
                        Merces::Deposit::SIGNATURE_HASH,
                        Merces::Withdraw::SIGNATURE_HASH,
                        Merces::Transfer::SIGNATURE_HASH,
                        Merces::TransferFrom::SIGNATURE_HASH,
                    ]
                    .contains(topic0)
                    {
                        Some(log)
                    } else {
                        None
                    }
                })
            })
            .context("no logs found in transaction receipt")?;
        let action_index = crate::u256_to_usize(
            (*log
                .topics()
                .get(1)
                .context("topic1 (action_index) not found in log")?)
            .into(),
        )?;

        Ok((action_index, receipt))
    }
}
