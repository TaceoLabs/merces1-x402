use crate::keys::{Keys, PublicKeys};
use alloy::{network::EthereumWallet, primitives::Address, providers::DynProvider};
use contract_rs::merces::MercesContract;
use itertools::{Itertools, izip};
use mpc_net::local::LocalNetwork;
use mpc_nodes::{
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
                    mpc_nodes::mpc_party(
                        my_key,
                        action,
                        ciphertexts,
                        &proving_key,
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
        eyre::ensure!(
            proofs
                .iter()
                .map(|res| (&res.0, &res.1, &res.2, &res.3, &res.4))
                .all_equal(),
            "MPC parties produced different results"
        );
        let (applied_transactions, commitments, valids, proof, public_inputs, _, _) =
            proofs.pop().unwrap();
        if !self.proving_key.verify(&proof, &public_inputs)? {
            eyre::bail!("Proof verification failed");
        }
        tracing::info!("Processed {applied_transactions} transactions in MPC");
        let invalid = valids.iter().filter(|valid| !**valid).count();
        if invalid != 0 {
            if invalid == 1 {
                tracing::warn!("1 invalid transaction was detected in MPC");
            } else {
                tracing::warn!("{} invalid transactions were detected in MPC", invalid);
            }
        }

        let commitments = commitments
            .into_iter()
            .map(contract_rs::bn254_fr_to_u256)
            .collect::<Vec<_>>();
        let beta = public_inputs[0];

        let res = contract
            .process_mpc(
                &self.provider,
                applied_transactions,
                commitments.try_into().unwrap(),
                valids.try_into().unwrap(),
                beta,
                proof,
            )
            .await?;

        Ok(res.0)
    }
}
