use ark_ff::UniformRand;
use client::{
    circom::config::CircomConfig, transfer::Transfer, transfer_compressed::TransferCompressed,
};
use criterion::{Criterion, criterion_group, criterion_main};
use rand::{CryptoRng, Rng, thread_rng};
use std::array;

fn transfer<R: Rng + CryptoRng>(c: &mut Criterion, rng: &mut R) {
    let groth16 = CircomConfig::get_transfer_key_material(rng).unwrap();

    let amount = ark_bn254::Fr::from(rng.r#gen::<u64>());
    let mpc_pks = array::from_fn(|_| ark_babyjubjub::EdwardsAffine::rand(rng));

    let transfer = Transfer::new(amount, mpc_pks, rng);

    c.bench_function("transfer proof", |b| {
        b.iter(|| {
            std::hint::black_box(
                transfer.generate_proof(std::hint::black_box(&groth16), std::hint::black_box(rng)),
            )
        });
    });
}

fn transfer_compressed<R: Rng + CryptoRng>(c: &mut Criterion, rng: &mut R) {
    let groth16 = CircomConfig::get_transfer_compressed_key_material(rng).unwrap();

    let amount = ark_bn254::Fr::from(rng.r#gen::<u64>());
    let mpc_pks = array::from_fn(|_| ark_babyjubjub::EdwardsAffine::rand(rng));

    let mut transfer = TransferCompressed::new(amount, mpc_pks, rng);

    c.bench_function("transfer proof compressed", |b| {
        b.iter(|| {
            std::hint::black_box({
                transfer.compute_alpha();
                transfer.generate_proof(std::hint::black_box(&groth16), std::hint::black_box(rng))
            })
        });
    });
}

fn criterion_benchmark(c: &mut Criterion) {
    let mut rng = thread_rng();

    transfer(c, &mut rng);
    transfer_compressed(c, &mut rng);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
