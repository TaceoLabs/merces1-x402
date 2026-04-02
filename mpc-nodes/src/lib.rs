pub mod circom;
pub mod map;
pub mod process_map;

use crate::{circom::config::CircomConfig, map::DepositValueShare};
use ark_ff::{BigInteger, PrimeField, Zero};
use circom_mpc_vm::{ComponentAcceleratorOutput, Rep3VmType};
use itertools::Itertools;
use mpc_core::{
    gadgets::poseidon2::{CircomTraceBatchedHasher, CircomTracePlainHasher, Poseidon2},
    protocols::{
        rep3::{self, Rep3PrimeFieldShare, Rep3State, network::Rep3NetworkExt},
        rep3_ring::{self, Rep3RingShare, ring::bit::Bit},
    },
    serde_compat::{ark_de, ark_se},
};
use mpc_net::Network;
use num_bigint::BigUint;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub(crate) type F = ark_bn254::Fr;

pub(crate) const CIRCOM_MAP_LABELS: [&str; 8] = [
    "sender_old_balance",
    "sender_old_r",
    "receiver_old_balance",
    "receiver_old_r",
    "amount",
    "amount_r",
    "sender_new_r",
    "receiver_new_r",
];

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub enum Action<K> {
    Deposit(
        K,
        #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")] F,
    ), // Receiver, amount
    Withdraw(
        K,
        #[serde(serialize_with = "ark_se", deserialize_with = "ark_de")] F,
    ), // Sender, amount
    Transfer(K, K, Rep3PrimeFieldShare<F>, Rep3PrimeFieldShare<F>), // Sender, Receiver, amount, amount_blinding
}

pub(crate) fn poseidon2_circom_commitment_helper<
    const I: usize,
    const I2: usize,
    F: PrimeField,
    N: Network,
>(
    input: &mut [Rep3PrimeFieldShare<F>; I2],
    net: &N,
    rep3_state: &mut Rep3State,
) -> eyre::Result<Vec<ComponentAcceleratorOutput<Rep3VmType<F>>>> {
    const T: usize = 2;
    assert_eq!(T * I, I2);
    let domain_separator = F::from(CircomConfig::DOMAIN_SEPARATOR);
    let hasher = Poseidon2::<F, T, 5>::default();
    let mut hasher_precomp = hasher.precompute_rep3(I, net, rep3_state)?;

    for input in input.iter_mut().step_by(T) {
        rep3::arithmetic::add_assign_public(input, domain_separator, rep3_state.id);
    }

    let mut result = Vec::with_capacity(I);
    let (states, traces) = hasher
        .rep3_permutation_in_place_with_precomputation_intermediate_packed::<N, I2, I>(
            *input,
            &mut hasher_precomp,
            net,
        )?;
    *input = states;
    for (state, trace) in states.chunks(T).zip(traces) {
        result.push(ComponentAcceleratorOutput::new(
            state
                .iter()
                .map(|x| (*x).into())
                .collect::<Vec<Rep3VmType<F>>>(),
            trace
                .iter()
                .map(|x| (*x).into())
                .collect::<Vec<Rep3VmType<F>>>(),
        ));
    }

    Ok(result)
}

#[expect(clippy::type_complexity)]
pub(crate) fn poseidon2_plain_circom_commitment_helper<
    const T: usize,
    const I: usize,
    const I2: usize,
    F: PrimeField,
>(
    mut input: [F; I2],
) -> eyre::Result<(Vec<[F; T]>, Vec<ComponentAcceleratorOutput<Rep3VmType<F>>>)> {
    assert_eq!(T * I, I2);
    let domain_separator = F::from(CircomConfig::DOMAIN_SEPARATOR);
    let hasher = Poseidon2::<F, T, 5>::default();
    for input in input.iter_mut().step_by(T) {
        *input += domain_separator;
    }

    let mut result = Vec::with_capacity(I);
    let mut traces = Vec::with_capacity(I);
    for input in input.chunks_exact(T) {
        let (state, trace) = hasher
            .plain_permutation_intermediate(input.try_into().expect("we take exact chunks"))?;
        let trace = ComponentAcceleratorOutput::new(
            state.iter().map(|x| (*x).into()).collect(),
            trace.into_iter().map(|x| x.into()).collect(),
        );
        result.push(state);
        traces.push(trace);
    }
    Ok((result, traces))
}

fn poseidon2_plain_sponge_circom_helper<const T: usize, const I2: usize, F: PrimeField>(
    input: [F; I2],
) -> (Vec<ComponentAcceleratorOutput<Rep3VmType<F>>>, F) {
    let domain_separator = F::from(CircomConfig::DOMAIN_SEPARATOR);
    let hasher = Poseidon2::<F, T, 5>::default();
    let permutations = I2.div_ceil(T - 1);
    let mut states = [F::zero(); T];

    // Initialize the state
    states[T - 1] = domain_separator;

    let mut traces = Vec::with_capacity(permutations);
    let mut absorbed = 0;
    for _ in 0..permutations {
        let mut remaining = I2 - absorbed;
        if remaining >= T - 1 {
            remaining = T - 1;
        }
        for i in 0..remaining {
            states[i] += input[absorbed + i];
        }
        absorbed += remaining;
        let res = hasher
            .plain_permutation_intermediate(states)
            .expect("This should work");
        states = res.0;
        traces.push(ComponentAcceleratorOutput::new(
            res.0.into_iter().map(|x| x.into()).collect(),
            res.1.into_iter().map(|x| x.into()).collect(),
        ));
    }

    (traces, states[0])
}

pub(crate) fn feed_forward_shared<
    const T: usize,
    const I: usize,
    const I2: usize,
    F: PrimeField,
>(
    commitments: [Rep3PrimeFieldShare<F>; I2],
    input: [Rep3PrimeFieldShare<F>; I2],
) -> [Rep3PrimeFieldShare<F>; I] {
    assert_eq!(T * I, I2);
    std::array::from_fn(|i| {
        let idx = i * T;
        input[idx] + commitments[idx]
    })
}

pub(crate) fn decompose_compose<F: PrimeField, N: Network>(
    sender_new_balance: Rep3PrimeFieldShare<F>,
    net: &N,
    rep3_state: &mut Rep3State,
) -> eyre::Result<(bool, ComponentAcceleratorOutput<Rep3VmType<F>>)> {
    let a2b = rep3::conversion::a2b(sender_new_balance, net, rep3_state)?;

    let mut to_compose = Vec::with_capacity(F::MODULUS_BIT_SIZE as usize);

    assert!(F::MODULUS_BIT_SIZE <= 256);
    assert!(F::MODULUS_BIT_SIZE > 192);
    let a2b_sender_a_ = a2b.a.to_u64_digits();
    let a2b_sender_b_ = a2b.b.to_u64_digits();

    // First the lowest 128 bit
    let mut a2b_sender_a = ((a2b_sender_a_[1] as u128) << 64) | a2b_sender_a_[0] as u128;
    let mut a2b_sender_b = ((a2b_sender_b_[1] as u128) << 64) | a2b_sender_b_[0] as u128;
    for _ in 0..128 {
        let bit = Rep3RingShare::new(
            Bit::new((a2b_sender_a & 1) == 1),
            Bit::new((a2b_sender_b & 1) == 1),
        );
        to_compose.push(bit);
        a2b_sender_a >>= 1;
        a2b_sender_b >>= 1;
    }
    // Then the rest
    let mut a2b_sender_a = ((a2b_sender_a_[3] as u128) << 64) | a2b_sender_a_[2] as u128;
    let mut a2b_sender_b = ((a2b_sender_b_[3] as u128) << 64) | a2b_sender_b_[2] as u128;
    for _ in 0..F::MODULUS_BIT_SIZE as usize - 128 {
        let bit = Rep3RingShare::new(
            Bit::new((a2b_sender_a & 1) == 1),
            Bit::new((a2b_sender_b & 1) == 1),
        );
        to_compose.push(bit);
        a2b_sender_a >>= 1;
        a2b_sender_b >>= 1;
    }

    let decomps = rep3_ring::conversion::bit_inject_from_bits_to_field_many::<F, _>(
        &to_compose,
        net,
        rep3_state,
    )?;

    debug_assert_eq!(decomps.len(), F::MODULUS_BIT_SIZE as usize);

    // Check if valid
    // let valid_should_zero = a2b >> CircomConfig::TRANSFER_BALANCE_BITSIZE;
    // let is_zero = rep3::binary::is_zero(&valid_should_zero, net, rep3_state)?;
    // let is_zero = Rep3RingShare::new(Bit::new(is_zero.a.bit(0)), Bit::new(is_zero.b.bit(0)));
    // let valid = rep3_ring::binary::open(&is_zero, net)?.0.convert();

    // Instead of doing the above, we can sum up the bits, multiply a random value and open
    let mut should_zero = Rep3PrimeFieldShare::zero();
    for bit in decomps.iter().skip(CircomConfig::TRANSFER_BALANCE_BITSIZE) {
        should_zero += bit;
    }
    let rand = rep3::arithmetic::rand(rep3_state);
    let should_zero_rand = should_zero * rand;
    let (b, c) = net.broadcast(should_zero_rand)?;
    let opened = should_zero_rand + b + c;
    let valid = opened.is_zero();

    let balance = ComponentAcceleratorOutput::new(
        decomps.into_iter().map(Rep3VmType::from).collect_vec(),
        Vec::new(),
    );
    Ok((valid, balance))
}

pub(crate) fn get_query_transaction_circom_input(
    sender_old: DepositValueShare<F>,
    receiver_old: Option<DepositValueShare<F>>,
    amount: Rep3PrimeFieldShare<F>,
    amount_blinding: Rep3PrimeFieldShare<F>,
    sender_new_blinding: Rep3PrimeFieldShare<F>,
    receiver_new_blinding: Rep3PrimeFieldShare<F>,
) -> (
    Vec<Rep3VmType<F>>,
    Rep3PrimeFieldShare<F>,
    Rep3PrimeFieldShare<F>,
) {
    let mut inputs = Vec::with_capacity(8);
    inputs.push(Rep3VmType::from(sender_old.amount));
    inputs.push(Rep3VmType::from(sender_old.blinding));
    let (receiver_old_amount, receiver_old_blinding) = if let Some(old) = receiver_old {
        inputs.push(Rep3VmType::from(old.amount));
        inputs.push(Rep3VmType::from(old.blinding));
        (old.amount, old.blinding)
    } else {
        inputs.push(Rep3VmType::from(F::zero()));
        inputs.push(Rep3VmType::from(F::zero()));
        (Rep3PrimeFieldShare::zero(), Rep3PrimeFieldShare::zero())
    };
    inputs.push(Rep3VmType::from(amount));
    inputs.push(Rep3VmType::from(amount_blinding));
    inputs.push(Rep3VmType::from(sender_new_blinding));
    inputs.push(Rep3VmType::from(receiver_new_blinding));
    (inputs, receiver_old_amount, receiver_old_blinding)
}

pub(crate) fn get_query_withdraw_circom_input_public_amount(
    sender_old: DepositValueShare<F>,
    amount: F,
    amount_blinding: F,
    sender_new_blinding: Rep3PrimeFieldShare<F>,
) -> Vec<Rep3VmType<F>> {
    vec![
        Rep3VmType::from(sender_old.amount),
        Rep3VmType::from(sender_old.blinding),
        Rep3VmType::default(),
        Rep3VmType::default(),
        Rep3VmType::from(amount),
        Rep3VmType::from(amount_blinding),
        Rep3VmType::from(sender_new_blinding),
        Rep3VmType::default(),
    ]
}

pub(crate) fn get_deposit_input_public_amount_circom(
    receiver_old: Option<DepositValueShare<F>>,
    amount: F,
    amount_blinding: F,
    receiver_new_blinding: Rep3PrimeFieldShare<F>,
) -> (
    Vec<Rep3VmType<F>>,
    Rep3PrimeFieldShare<F>,
    Rep3PrimeFieldShare<F>,
) {
    let mut inputs = Vec::with_capacity(8);
    inputs.push(Rep3VmType::from(amount));
    inputs.push(Rep3VmType::from(amount_blinding));
    let (receiver_old_amount, receiver_old_blinding) = if let Some(old) = receiver_old {
        inputs.push(Rep3VmType::from(old.amount));
        inputs.push(Rep3VmType::from(old.blinding));
        (old.amount, old.blinding)
    } else {
        inputs.push(Rep3VmType::default());
        inputs.push(Rep3VmType::default());
        (Rep3PrimeFieldShare::zero(), Rep3PrimeFieldShare::zero())
    };
    inputs.push(Rep3VmType::from(amount));
    inputs.push(Rep3VmType::from(amount_blinding));
    inputs.push(Rep3VmType::default());
    inputs.push(Rep3VmType::from(receiver_new_blinding));
    (inputs, receiver_old_amount, receiver_old_blinding)
}

pub(crate) fn add_inputs_to_circom_map(
    i: usize,
    inputs: Vec<Rep3VmType<F>>,
    circom_map: &mut BTreeMap<String, Rep3VmType<F>>,
) {
    debug_assert!(inputs.len() == 8);
    for (inp, label) in inputs.into_iter().zip(CIRCOM_MAP_LABELS.iter()) {
        circom_map.insert(format!("{label}[{i}]").to_string(), inp.clone());
    }
}

pub(crate) fn compute_alpha<const I: usize, F: PrimeField>(public_inputs: [F; I]) -> F {
    let public_inputs_as_bytes = public_inputs
        .iter()
        .flat_map(|x| {
            let mut bytes = x.into_bigint().to_bytes_be();
            if bytes.len() < 32 {
                let mut padded = vec![0u8; 32 - bytes.len()];
                padded.extend_from_slice(&bytes);
                bytes = padded;
            }
            bytes
        })
        .collect::<Vec<u8>>();

    let mut hasher = Sha256::new();
    hasher.update(public_inputs_as_bytes);
    let sha_hash = hasher.finalize();
    let mut alpha = BigUint::from_bytes_be(&sha_hash);
    let mask = (BigUint::from(1u8) << 253) - BigUint::from(1u8);
    alpha &= mask; // Drop three bits from the calculated hash
    F::from(alpha)
}

#[expect(unused)]
pub(crate) fn compression_commitment_helper<
    const T_SPONGE: usize,
    const I: usize,
    F: PrimeField,
>(
    public_inputs: [F; I],
) -> (Vec<ComponentAcceleratorOutput<Rep3VmType<F>>>, F) {
    let alpha = compute_alpha(public_inputs);

    let (beta_traces, _) = poseidon2_plain_sponge_circom_helper::<T_SPONGE, I, _>(public_inputs);

    (beta_traces, alpha)
}
