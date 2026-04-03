use crate::keys::{Keys, PublicKeys};
use alloy::{network::EthereumWallet, primitives::Address, providers::DynProvider};
use ark_bn254::Bn254;
use ark_groth16::Proof;
use contract_rs::merces::{
    Merces::{ActionItem, Ciphertext},
    MercesContract,
};
use itertools::izip;
use mpc_core::{
    MpcState,
    protocols::rep3::{
        Rep3PrimeFieldShare, Rep3State, conversion::A2BType, network::Rep3NetworkExt,
    },
};
use mpc_net::{Network, local::LocalNetwork};
use mpc_nodes::{
    Action,
    circom::{config::CircomConfig, groth16::Groth16Material},
    map::{DepositValueShare, PrivateDeposit},
};
use rand::{CryptoRng, Rng};
use secrecy::{ExposeSecret, SecretString};
use std::{
    sync::Arc,
    thread::{self, ScopedJoinHandle},
};

pub struct Mpc {
    provider: DynProvider,
    signer: Address,
    mpc_keys: Keys,
    proving_key: Arc<Groth16Material>,
    maps: [PrivateDeposit<Address, DepositValueShare<ark_bn254::Fr>>; 3],
}

impl Mpc {
    pub async fn from_wallet<R: Rng + CryptoRng>(
        rpc: &SecretString,
        wallet: EthereumWallet,
        proving_key: Arc<Groth16Material>,
        rng: &mut R,
    ) -> eyre::Result<Self> {
        let signer = wallet.default_signer().address();
        let provider = crate::connect_rpc(rpc.expose_secret(), wallet)
            .await
            .expect("Failed to connect to RPC");
        Ok(Self {
            provider,
            signer,
            proving_key,
            mpc_keys: Keys::random(rng),
            maps: Default::default(),
        })
    }

    pub fn get_provider(&self) -> &DynProvider {
        &self.provider
    }

    pub fn get_signer(&self) -> Address {
        self.signer
    }

    pub fn public_keys(&self) -> PublicKeys {
        self.mpc_keys.public_keys()
    }

    fn get_networks() -> [Vec<LocalNetwork>; 3] {
        let mut test_networks0 = Vec::with_capacity(CircomConfig::NUM_TRANSACTIONS);
        let mut test_networks1 = Vec::with_capacity(CircomConfig::NUM_TRANSACTIONS);
        let mut test_networks2 = Vec::with_capacity(CircomConfig::NUM_TRANSACTIONS);
        for _ in 0..CircomConfig::NUM_TRANSACTIONS {
            let [net0, net1, net2] = LocalNetwork::new(3).try_into().unwrap();
            test_networks0.push(net0);
            test_networks1.push(net1);
            test_networks2.push(net2);
        }
        [test_networks0, test_networks1, test_networks2]
    }

    fn read_and_build_queue<N: Network>(
        my_key: &ark_babyjubjub::Fr,
        action: &[ActionItem],
        ciphertexts: &[Ciphertext],
        networks: &[N],
    ) -> eyre::Result<Vec<Action<Address>>> {
        assert_eq!(action.len(), ciphertexts.len());
        assert!(action.len() <= networks.len());

        let my_id = networks[0].id();
        assert!(my_id < 3);
        let mut queue = Vec::with_capacity(action.len());
        for (action, ciphertext, net) in izip!(action, ciphertexts, networks) {
            match action.action {
                1 => {
                    // Deposit
                    queue.push(Action::Deposit(
                        action.receiver,
                        contract_rs::u256_to_field(action.amount)?,
                    ));
                }
                2 => {
                    // Withdraw
                    queue.push(Action::Withdraw(
                        action.sender,
                        contract_rs::u256_to_field(action.amount)?,
                    ));
                }
                3 => {
                    // Transfer
                    let ciphertext =
                        MercesContract::decode_ciphertext(ciphertext.clone())?[my_id].to_owned();
                    let sym_key =
                        client::cryptography::dh_key_derivation(my_key, ciphertext.sender_pk);
                    let plaintexts = client::cryptography::sym_decrypt2(
                        sym_key,
                        [ciphertext.amount, ciphertext.amount_r],
                        Default::default(),
                    );
                    let reshared = net.reshare_many(&plaintexts)?;
                    if reshared.len() != 2 {
                        eyre::bail!(
                            "Expected exactly 2 reshared plaintexts for transfer, got {}",
                            reshared.len()
                        );
                    }
                    queue.push(Action::Transfer(
                        action.sender,
                        action.receiver,
                        Rep3PrimeFieldShare::new(plaintexts[0], reshared[0]),
                        Rep3PrimeFieldShare::new(plaintexts[1], reshared[1]),
                    ));
                }
                x => {
                    return Err(eyre::eyre!("Unsupported action in action queue: {:?}", x));
                }
            }
        }

        Ok(queue)
    }

    #[expect(clippy::type_complexity)]
    fn mpc_party<N: Network>(
        my_key: &ark_babyjubjub::Fr,
        action: &[ActionItem],
        ciphertexts: &[Ciphertext],
        proving_key: Arc<Groth16Material>,
        map: &mut PrivateDeposit<Address, DepositValueShare<ark_bn254::Fr>>,
        nets: &[N; CircomConfig::NUM_TRANSACTIONS],
    ) -> eyre::Result<(
        usize,
        Vec<ark_bn254::Fr>,
        Vec<bool>,
        Proof<Bn254>,
        Vec<ark_bn254::Fr>,
    )> {
        let queue = Self::read_and_build_queue(my_key, action, ciphertexts, nets)?;

        let mut rep3_states = Vec::with_capacity(nets.len());
        let mut rep3_state = Rep3State::new(&nets[0], A2BType::default())?;
        for _ in 1..CircomConfig::NUM_TRANSACTIONS {
            rep3_states.push(rep3_state.fork(0).unwrap());
        }
        rep3_states.push(rep3_state);

        let (applied_transactions, commitments, valids, inputs, traces) = map
            .process_queue_with_cocircom_trace(
                queue,
                nets,
                rep3_states.as_mut_slice().try_into().unwrap(),
                CircomConfig::COMPRESSION,
            )?;
        let (proof, public_inputs) =
            proving_key.trace_to_proof(inputs, traces, &nets[0], &nets[1])?;

        Ok((
            applied_transactions,
            commitments,
            valids,
            proof,
            public_inputs,
        ))
    }

    pub async fn process_mpc(&mut self, contract: &MercesContract) -> eyre::Result<Vec<usize>> {
        let networks = Self::get_networks();

        let queue = contract
            .read_queue(CircomConfig::NUM_TRANSACTIONS, &self.provider)
            .await?;

        let res = thread::scope(|s| {
            let mut handles = Vec::with_capacity(3);
            for (networks, my_key, map) in
                izip!(networks, self.mpc_keys.iter(), self.maps.iter_mut())
            {
                let proving_key = self.proving_key.to_owned();
                let action = &queue.1;
                let ciphertexts = &queue.2;
                let handle: ScopedJoinHandle<eyre::Result<_>> = s.spawn(move || {
                    Self::mpc_party(
                        my_key,
                        action,
                        ciphertexts,
                        proving_key,
                        map,
                        networks.as_slice().try_into().unwrap(),
                    )
                });
                handles.push(handle);
            }

            let mut proofs = Vec::with_capacity(3);
            for (i, handle) in handles.into_iter().enumerate() {
                let join = handle.join();
                if join.is_err() {
                    eyre::bail!("MPC party {i} thread panicked");
                }
                let proof = join.unwrap()?;
                proofs.push(proof);
            }
            eyre::Result::Ok(proofs)
        });

        let mut proofs = res?;
        let proof = proofs.pop().unwrap();
        for proof_ in proofs {
            if proof != proof_ {
                eyre::bail!("MPC parties produced different proofs");
            }
        }
        if !self.proving_key.verify(&proof.3, &proof.4)? {
            eyre::bail!("Proof verification failed");
        }
        tracing::info!("Processed {} transactions in MPC", proof.0);
        let invalid = proof.2.iter().filter(|valid| !**valid).count();
        if invalid != 0 {
            if invalid == 1 {
                tracing::warn!("1 invalid transaction was detected in MPC");
            } else {
                tracing::warn!("{} invalid transactions were detected in MPC", invalid);
            }
        }

        let commitments = proof
            .1
            .into_iter()
            .map(contract_rs::bn254_fr_to_u256)
            .collect::<Vec<_>>();
        // let beta = proof.4[0];

        let res = contract
            .process_mpc(
                &self.provider,
                proof.0,
                commitments.try_into().unwrap(),
                proof.2.try_into().unwrap(),
                // beta,
                proof.3,
            )
            .await?;

        Ok(res.0)
    }
}
