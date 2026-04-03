use crate::{
    Action, F,
    circom::config::CircomConfig,
    map::{DepositValueShare, PrivateDeposit},
};
use ark_ff::{One, PrimeField, Zero};
use circom_mpc_vm::{ComponentAcceleratorOutput, Rep3VmType};
use itertools::izip;
use mpc_core::{
    gadgets,
    protocols::rep3::{self, Rep3PrimeFieldShare, Rep3State, id::PartyID},
};
use mpc_net::Network;
use std::{
    collections::{BTreeMap, HashSet},
    thread,
};

impl<K> PrivateDeposit<K, DepositValueShare<F>>
where
    K: std::hash::Hash + Eq + Clone + Send + Sync,
{
    pub fn zero_commitment() -> F {
        gadgets::field_from_hex_string(
            "0x87f763a403ee4109adc79d4a7638af3cb8cb6a33f5b027bd1476ffa97361acb",
        )
        .expect("Known string should work") // commit(0, 0)
    }

    #[expect(clippy::type_complexity)]
    // TODO we should probably only update the map after proof verification...
    pub fn process_queue_with_cocircom_trace<N: Network>(
        &mut self,
        queue: Vec<Action<K>>,
        nets: &[N; CircomConfig::NUM_TRANSACTIONS],
        rep3_states: &mut [Rep3State; CircomConfig::NUM_TRANSACTIONS],
        compression: bool,
    ) -> eyre::Result<(
        usize,
        Vec<F>,
        Vec<bool>,
        BTreeMap<String, Rep3VmType<F>>,
        Vec<ComponentAcceleratorOutput<Rep3VmType<F>>>,
    )> {
        let num_transactions = queue.len();
        assert!(
            num_transactions <= CircomConfig::NUM_TRANSACTIONS,
            "Queue length exceeds maximum"
        );

        let mut proof_inputs = BTreeMap::new();
        let mut traces = Vec::with_capacity(
            CircomConfig::NUM_TOTAL_COMMITMENTS + CircomConfig::NUM_TRANSACTIONS,
        ); // Commitments and range checks
        let mut new_balance_commitments = Vec::with_capacity(CircomConfig::NUM_TRANSACTIONS * 2);
        let mut valids = Vec::with_capacity(CircomConfig::NUM_TRANSACTIONS);
        let mut public_inputs = Vec::with_capacity(CircomConfig::NUM_PUBLIC_INPUTS);
        let mut applied_transactions = 0;

        let my_id = PartyID::try_from(nets[0].id())?;
        // We apply all changes to the copy map and in the end (if valid) we update the real map.
        // This is necessary to ensure that the i-th transaction has the result of th (i-1)-th transaction available, but we don't apply invalid transactions to the map.
        let mut copy_map = self.clone();

        let result = thread::scope(|scope| {
            let mut handles = Vec::with_capacity(num_transactions);
            for (action, net, rep3_state) in izip!(queue.iter(), nets, rep3_states.iter_mut()) {
                match action {
                    Action::Deposit(receiver, amount) => {
                        let amount_shared =
                            rep3::arithmetic::promote_to_trivial_share(my_id, *amount);
                        let (receiver_old, receiver_new) =
                            copy_map.deposit(receiver.clone(), amount_shared, rep3_state);
                        let handle = scope.spawn(move || {
                            Self::process_deposit_circom(
                                receiver_old,
                                receiver_new,
                                *amount,
                                net,
                                rep3_state,
                            )
                        });
                        handles.push(handle);
                    }
                    Action::Withdraw(sender, amount) => {
                        let amount_shared =
                            rep3::arithmetic::promote_to_trivial_share(my_id, *amount);
                        let (sender_old, sender_new) =
                            copy_map.withdraw(sender.clone(), amount_shared, rep3_state)?;
                        let handle = scope.spawn(move || {
                            Self::process_withdraw_circom(
                                sender_old, sender_new, *amount, net, rep3_state,
                            )
                        });
                        handles.push(handle);
                    }
                    Action::Transfer(sender, receiver, amount, amount_blinding) => {
                        let (sender_old, sender_new, receiver_old, receiver_new) = copy_map
                            .transaction(sender.clone(), receiver.clone(), *amount, rep3_state)?;
                        let handle = scope.spawn(move || {
                            Self::process_transaction_circom(
                                sender_old,
                                receiver_old,
                                sender_new,
                                receiver_new,
                                *amount,
                                *amount_blinding,
                                net,
                                rep3_state,
                            )
                        });
                        handles.push(handle);
                    }
                }
            }

            let mut errors = false; // Is true if a transaction was invalid.
            let mut faulty_parties = HashSet::new(); // The parties involved in invalid transactions
            let mut full_break = false; // Determines if we have to abort since the same account was accessed after an invalid transaction
            for (i, (action, handle)) in queue.iter().zip(handles).enumerate() {
                let (valid, sender_new_, receiver_new_, inputs_, traces_, commitments_opened) =
                    handle.join().map_err(|_| {
                        eyre::eyre!("A thread panicked while processing a transaction")
                    })??;

                // Check if we can apply the transaction
                if full_break {
                    continue;
                }
                if errors {
                    // There was a error before, check if we can still apply the transaction
                    match action {
                        Action::Deposit(receiver, _) => {
                            if faulty_parties.contains(receiver) {
                                full_break = true;
                                continue;
                            }
                        }
                        Action::Withdraw(sender, _) => {
                            if faulty_parties.contains(sender) {
                                full_break = true;
                                continue;
                            }
                        }
                        Action::Transfer(sender, receiver, _, _) => {
                            if faulty_parties.contains(sender) || faulty_parties.contains(receiver)
                            {
                                full_break = true;
                                continue;
                            }
                        }
                    }
                }
                // We can apply the transaction if it is valid, and can process it anyway
                if valid {
                    match action {
                        Action::Deposit(receiver, _) => {
                            self.insert(receiver.clone(), receiver_new_);
                        }
                        Action::Withdraw(sender, _) => {
                            self.insert(sender.clone(), sender_new_);
                        }
                        Action::Transfer(sender, receiver, _, _) => {
                            self.insert(sender.clone(), sender_new_);
                            self.insert(receiver.clone(), receiver_new_);
                        }
                    }
                } else {
                    // Transaction is invalid, but so far we can still continue
                    errors = true;
                    match action {
                        Action::Deposit(receiver, _) => {
                            faulty_parties.insert(receiver.clone());
                        }
                        Action::Withdraw(sender, _) => {
                            faulty_parties.insert(sender.clone());
                        }
                        Action::Transfer(sender, receiver, _, _) => {
                            faulty_parties.insert(sender.clone());
                            faulty_parties.insert(receiver.clone());
                        }
                    }
                }
                match action {
                    Action::Deposit(_, _) => {
                        new_balance_commitments.push(F::zero()); // Smart contract expects 0
                        new_balance_commitments.push(commitments_opened[3]); // Receiver new balance commitment
                    }
                    Action::Withdraw(_, _) => {
                        new_balance_commitments.push(commitments_opened[1]); // Sender new balance commitment
                        new_balance_commitments.push(F::zero()); // Smart contract expects 0
                    }
                    Action::Transfer(_, _, _, _) => {
                        new_balance_commitments.push(commitments_opened[1]); // Sender new balance commitment
                        new_balance_commitments.push(commitments_opened[3]); // Receiver new balance commitment
                    }
                }
                valids.push(valid);

                public_inputs.extend(commitments_opened);
                public_inputs.push(if valid { F::one() } else { F::zero() }); // Validity flag as public input
                super::add_inputs_to_circom_map(i, inputs_, &mut proof_inputs);
                traces.extend(traces_);
                applied_transactions += 1;
            }

            debug_assert_eq!(
                public_inputs.len(),
                (CircomConfig::NUM_COMMITMENTS + 1) * applied_transactions
            );
            // We need to pad the commitments and traces
            if applied_transactions < CircomConfig::NUM_TRANSACTIONS {
                let (dummy_input, dummy_trace) = Self::process_dummy_circom()?;

                let pad_public_inputs = [
                    Self::zero_commitment(),
                    Self::zero_commitment(),
                    Self::zero_commitment(),
                    Self::zero_commitment(),
                    Self::zero_commitment(),
                    F::one(),
                ];

                new_balance_commitments.resize(CircomConfig::NUM_TRANSACTIONS * 2, F::zero()); // Smart contract expects 0
                valids.resize(CircomConfig::NUM_TRANSACTIONS, true);

                for i in applied_transactions..CircomConfig::NUM_TRANSACTIONS {
                    super::add_inputs_to_circom_map(i, dummy_input.clone(), &mut proof_inputs);
                    traces.extend(dummy_trace.clone());
                    public_inputs.extend(pad_public_inputs);
                }
            }
            debug_assert_eq!(public_inputs.len(), CircomConfig::NUM_PUBLIC_INPUTS);
            debug_assert_eq!(valids.len(), CircomConfig::NUM_TRANSACTIONS);
            debug_assert_eq!(
                new_balance_commitments.len(),
                CircomConfig::NUM_TRANSACTIONS * 2
            );
            debug_assert_eq!(
                traces.len(),
                CircomConfig::NUM_TOTAL_COMMITMENTS + CircomConfig::NUM_TRANSACTIONS
            );

            if compression {
                // TODO once the Tracebuilder is fixed, we can use the compression helper again
                // let (final_traces, alpha) =
                //     super::compression_commitment_helper::<
                //         { CircomConfig::POSEIDON2_SPONGE_T },
                //         { CircomConfig::NUM_PUBLIC_INPUTS },
                //         _,
                //     >(public_inputs.try_into().expect("we checked lengths before"))?;
                // traces.extend(final_traces);
                let alpha = super::compute_alpha::<{ CircomConfig::NUM_PUBLIC_INPUTS }, _>(
                    public_inputs.try_into().expect("we checked lengths before"),
                );
                proof_inputs.insert("alpha".to_string(), alpha.into());
            }
            Result::<_, eyre::Report>::Ok(())
        });
        result?;

        Ok((
            applied_transactions,
            new_balance_commitments,
            valids,
            proof_inputs,
            traces,
        ))
    }

    #[expect(clippy::type_complexity, clippy::too_many_arguments)]
    pub fn process_transaction_circom<N: Network>(
        sender_old: DepositValueShare<F>,
        receiver_old: Option<DepositValueShare<F>>,
        sender_new: DepositValueShare<F>,
        receiver_new: DepositValueShare<F>,
        amount: Rep3PrimeFieldShare<F>,
        amount_blinding: Rep3PrimeFieldShare<F>,
        net0: &N,
        rep3_state: &mut Rep3State,
    ) -> eyre::Result<(
        bool,
        DepositValueShare<F>,
        DepositValueShare<F>,
        Vec<Rep3VmType<F>>,
        Vec<ComponentAcceleratorOutput<Rep3VmType<F>>>,
        [F; CircomConfig::NUM_COMMITMENTS],
    )> {
        let (inputs, receiver_old_amount, receiver_old_blinding) =
            super::get_query_transaction_circom_input(
                sender_old.to_owned(),
                receiver_old,
                amount,
                amount_blinding,
                sender_new.blinding,
                receiver_new.blinding,
            );
        let input_commitment = [
            amount,
            amount_blinding,
            sender_old.amount,
            sender_old.blinding,
            sender_new.amount,
            sender_new.blinding,
            receiver_old_amount,
            receiver_old_blinding,
            receiver_new.amount,
            receiver_new.blinding,
        ];
        let mut state = input_commitment;

        let mut traces =
            super::poseidon2_circom_commitment_helper::<5, _, _, _>(&mut state, net0, rep3_state)?;

        let ff_commitments = super::feed_forward_shared::<
            2,
            { CircomConfig::NUM_COMMITMENTS },
            { 2 * CircomConfig::NUM_COMMITMENTS },
            _,
        >(state, input_commitment);

        let mut opened_commitments: [F; CircomConfig::NUM_COMMITMENTS] =
            rep3::arithmetic::open_vec(&ff_commitments, net0)?
                .try_into()
                .expect("should fit");
        opened_commitments.rotate_left(1); // Amount commitment is at the end for the sponge input

        // The bit decomposition
        let (valid, decomp_sender) = super::decompose_compose(sender_new.amount, net0, rep3_state)?;
        traces.insert(1, decomp_sender);

        Ok((
            valid,
            sender_new,
            receiver_new,
            inputs,
            traces,
            opened_commitments,
        ))
    }

    #[expect(clippy::type_complexity)]
    pub fn process_withdraw_circom<N: Network>(
        sender_old: DepositValueShare<F>,
        sender_new: DepositValueShare<F>,
        amount: F,
        net0: &N,
        rep3_state: &mut Rep3State,
    ) -> eyre::Result<(
        bool,
        DepositValueShare<F>,
        DepositValueShare<F>,
        Vec<Rep3VmType<F>>,
        Vec<ComponentAcceleratorOutput<Rep3VmType<F>>>,
        [F; CircomConfig::NUM_COMMITMENTS],
    )> {
        let my_id = PartyID::try_from(net0.id())?;
        let inputs = super::get_query_withdraw_circom_input_public_amount(
            sender_old.to_owned(),
            amount,
            F::zero(),
            sender_new.blinding,
        );

        let receiver_new = DepositValueShare::new(
            rep3::arithmetic::promote_to_trivial_share(my_id, amount),
            Rep3PrimeFieldShare::zero_share(),
        );

        let input_commitment = [
            sender_old.amount,
            sender_old.blinding,
            sender_new.amount,
            sender_new.blinding,
        ];
        let mut state = input_commitment;

        let mut traces =
            super::poseidon2_circom_commitment_helper::<2, _, _, _>(&mut state, net0, rep3_state)?;

        let (state_public, traces_) =
            super::poseidon2_plain_circom_commitment_helper::<2, 2, _, _>([
                amount,
                F::zero(),
                F::zero(),
                F::zero(),
            ])?;

        let (valid, decomp_sender) = super::decompose_compose(sender_new.amount, net0, rep3_state)?;

        traces.insert(0, traces_[0].clone());
        for trace in traces_.into_iter().rev() {
            traces.push(trace);
        }

        traces.insert(1, decomp_sender);

        let ff_commmitments_shared =
            super::feed_forward_shared::<2, 2, 4, _>(state, input_commitment);

        let ff_commitment_public = state_public[0][0] + amount;

        let opened_commitments = rep3::arithmetic::open_vec(&ff_commmitments_shared, net0)?;

        let opened_commitments_final = [
            opened_commitments[0],
            opened_commitments[1],
            state_public[1][0],
            ff_commitment_public,
            ff_commitment_public,
        ];

        Ok((
            valid,
            sender_new,
            receiver_new,
            inputs,
            traces,
            opened_commitments_final,
        ))
    }

    #[expect(clippy::type_complexity)]
    pub fn process_deposit_circom<N: Network>(
        receiver_old: Option<DepositValueShare<F>>,
        receiver_new: DepositValueShare<F>,
        amount: F,
        net0: &N,
        rep3_state: &mut Rep3State,
    ) -> eyre::Result<(
        bool,
        DepositValueShare<F>,
        DepositValueShare<F>,
        Vec<Rep3VmType<F>>,
        Vec<ComponentAcceleratorOutput<Rep3VmType<F>>>,
        [F; CircomConfig::NUM_COMMITMENTS],
    )> {
        let (inputs, receiver_old_amount, receiver_old_blinding) =
            super::get_deposit_input_public_amount_circom(
                receiver_old,
                amount,
                F::zero(),
                receiver_new.blinding,
            );

        let sender_new = DepositValueShare::<F>::new(
            Rep3PrimeFieldShare::zero_share(),
            Rep3PrimeFieldShare::zero_share(),
        );
        let input_commitment = [
            receiver_old_amount,
            receiver_old_blinding,
            receiver_new.amount,
            receiver_new.blinding,
        ];
        let mut state = input_commitment;

        let mut traces =
            super::poseidon2_circom_commitment_helper::<2, _, _, _>(&mut state, net0, rep3_state)?;

        let (state_public, traces_) =
            super::poseidon2_plain_circom_commitment_helper::<2, 2, _, _>([
                amount,
                F::zero(),
                F::zero(),
                F::zero(),
            ])?;

        traces.insert(0, traces_[0].clone());
        traces.insert(0, traces_[0].clone());
        traces.insert(2, traces_[1].clone());
        traces.insert(
            1,
            ComponentAcceleratorOutput::new(
                vec![Rep3VmType::default(); F::MODULUS_BIT_SIZE as usize],
                Vec::new(),
            ),
        ); // Mimic the range check

        let ff_commmitments_shared =
            super::feed_forward_shared::<2, 2, 4, _>(state, input_commitment);

        let ff_commitment_public = state_public[0][0] + amount;

        let opened_commitments = rep3::arithmetic::open_vec(&ff_commmitments_shared, net0)?;

        let opened_commitments_final = [
            ff_commitment_public,
            state_public[1][0],
            opened_commitments[0],
            opened_commitments[1],
            ff_commitment_public,
        ];

        Ok((
            true,
            sender_new,
            receiver_new,
            inputs,
            traces,
            opened_commitments_final,
        ))
    }

    #[expect(clippy::type_complexity)]
    pub fn process_dummy_circom() -> eyre::Result<(
        Vec<Rep3VmType<F>>,
        Vec<ComponentAcceleratorOutput<Rep3VmType<F>>>,
    )> {
        // Commitment, range check, commitment, commitment, commitment, commitment
        let mut plain_traces =
            super::poseidon2_plain_circom_commitment_helper::<2, 1, _, _>([F::zero(), F::zero()])?
                .1;

        plain_traces.push(ComponentAcceleratorOutput::new(
            vec![Rep3VmType::default(); F::MODULUS_BIT_SIZE as usize],
            Vec::new(),
        ));
        plain_traces.push(plain_traces[0].clone());
        plain_traces.push(plain_traces[0].clone());
        plain_traces.push(plain_traces[0].clone());
        plain_traces.push(plain_traces[0].clone());

        Ok((vec![Rep3VmType::default(); 8], plain_traces))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::{DepositValue, DepositValuePlain};
    use ark_ff::UniformRand;
    use mpc_core::protocols::rep3::conversion::A2BType;
    use mpc_net::local::LocalNetwork;
    use rand::{CryptoRng, Rng, thread_rng};
    use std::sync::Arc;

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

    #[test]
    fn process_queue() {
        const NUM_ITEMS: usize = 100;
        const TEST_RUNS: usize = 10;
        const NUM_TRANSACTIONS: usize = 3; // Depost, Withdraw and Transfer

        let mut rng = thread_rng();

        let groth16 = Arc::new(CircomConfig::get_transfer_key_material(&mut rng).unwrap());

        // Init networks
        let mut test_networks0 = Vec::with_capacity(CircomConfig::NUM_TRANSACTIONS);
        let mut test_networks1 = Vec::with_capacity(CircomConfig::NUM_TRANSACTIONS);
        let mut test_networks2 = Vec::with_capacity(CircomConfig::NUM_TRANSACTIONS);
        for _ in 0..CircomConfig::NUM_TRANSACTIONS {
            let [net0, net1, net2] = LocalNetwork::new(3).try_into().unwrap();
            test_networks0.push(net0);
            test_networks1.push(net1);
            test_networks2.push(net2);
        }

        let mut plain_map = get_random_plain_map::<F, _>(NUM_ITEMS, &mut rng);
        let mut map_shares = plain_map.share(&mut rng);

        // The actual testcase
        // We test a batch with 3 transactions deposit to first new key, transfer between first and second new key, withdraw from second new key
        for _ in 0..TEST_RUNS {
            // Get two new random keys
            let key1 = F::rand(&mut rng);
            let mut key2 = F::rand(&mut rng);
            while key2 == key1 {
                key2 = F::rand(&mut rng);
            }
            let amount = F::from(rng.r#gen::<u64>());
            let amount_blinding = F::rand(&mut rng);

            // Share the amount and the blinding
            let amount_share = rep3::share_field_element(amount, &mut rng);
            let amount_blinding_share = rep3::share_field_element(amount_blinding, &mut rng);

            // Action queue per party
            let mut action_queue_0 = Vec::with_capacity(NUM_TRANSACTIONS);
            let mut action_queue_1 = Vec::with_capacity(NUM_TRANSACTIONS);
            let mut action_queue_2 = Vec::with_capacity(NUM_TRANSACTIONS);

            // Deposit to key1
            action_queue_0.push(Action::Deposit(key1, amount));
            action_queue_1.push(Action::Deposit(key1, amount));
            action_queue_2.push(Action::Deposit(key1, amount));

            // Transfer from key1 to key2
            action_queue_0.push(Action::Transfer(
                key1,
                key2,
                amount_share[0],
                amount_blinding_share[0],
            ));
            action_queue_1.push(Action::Transfer(
                key1,
                key2,
                amount_share[1],
                amount_blinding_share[1],
            ));
            action_queue_2.push(Action::Transfer(
                key1,
                key2,
                amount_share[2],
                amount_blinding_share[2],
            ));

            // Withdraw from key2
            action_queue_0.push(Action::Withdraw(key2, amount));
            action_queue_1.push(Action::Withdraw(key2, amount));
            action_queue_2.push(Action::Withdraw(key2, amount));

            // Update plain map (just amount, ignore blinding)
            plain_map.insert(key1, DepositValue::new(F::zero(), F::zero()));
            plain_map.insert(key2, DepositValue::new(F::zero(), F::zero()));

            // Do the MPC work
            let (proof, public_inputs, commitments) = thread::scope(|scope| {
                let mut handles = Vec::with_capacity(3);
                for (nets, map, transaction) in izip!(
                    [
                        &mut test_networks0,
                        &mut test_networks1,
                        &mut test_networks2
                    ],
                    &mut map_shares,
                    [action_queue_0, action_queue_1, action_queue_2]
                ) {
                    let groth16 = groth16.clone();
                    let handle = scope.spawn(move || {
                        let mut rep3_states = Vec::with_capacity(nets.len());
                        for net in nets.iter() {
                            rep3_states.push(Rep3State::new(net, A2BType::default()).unwrap());
                        }

                        let (applied_transactions, commitments, valids, inputs, traces) = map
                            .process_queue_with_cocircom_trace(
                                transaction,
                                nets.as_slice().try_into().unwrap(),
                                rep3_states.as_mut_slice().try_into().unwrap(),
                                CircomConfig::COMPRESSION,
                            )
                            .unwrap();
                        assert_eq!(applied_transactions, NUM_TRANSACTIONS);
                        assert_eq!(valids.len(), CircomConfig::NUM_TRANSACTIONS);
                        assert!(valids.iter().all(|&v| v)); // All transactions should be valid, including dummies
                        assert_eq!(commitments.len(), CircomConfig::NUM_TRANSACTIONS * 2); // We have two commitments per transaction
                        let (proof, public_inputs) = groth16
                            .trace_to_proof(inputs, traces, &nets[0], &nets[1])
                            .unwrap();

                        (proof, public_inputs, commitments)
                    });
                    handles.push(handle);
                }

                let (proof0, public_inputs0, commitments0) = handles.remove(0).join().unwrap();
                for handle in handles {
                    let (proof, public_inputs, commitments) = handle.join().unwrap();
                    assert_eq!(proof, proof0);
                    assert_eq!(public_inputs, public_inputs0);
                    assert_eq!(commitments, commitments0);
                }
                (proof0, public_inputs0, commitments0)
            });

            // Verifiy the results
            assert!(groth16.verify(&proof, &public_inputs).unwrap());
            for comm in commitments.into_iter().skip(NUM_TRANSACTIONS * 2) {
                assert!(comm.is_zero());
            }
        }

        // Finally, compare the maps
        for (key, plain_value) in plain_map.into_iter() {
            let amount = plain_value.amount;
            let share0 = map_shares[0].remove(&key).unwrap().amount;
            let share1 = map_shares[1].remove(&key).unwrap().amount;
            let share2 = map_shares[2].remove(&key).unwrap().amount;
            let combined = rep3::combine_field_element(share0, share1, share2);
            assert_eq!(amount, combined);
        }
        assert!(map_shares[0].is_empty());
        assert!(map_shares[1].is_empty());
        assert!(map_shares[2].is_empty());
    }
}
