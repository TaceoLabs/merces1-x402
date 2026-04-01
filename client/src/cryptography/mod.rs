use ark_ec::{AffineRepr, CurveGroup};
use ark_ff::{BigInt, PrimeField};

pub fn get_pk(sk: ark_babyjubjub::Fr) -> ark_babyjubjub::EdwardsAffine {
    (ark_babyjubjub::EdwardsAffine::generator() * sk).into_affine()
}

pub fn dh_key_derivation(
    my_sk: &ark_babyjubjub::Fr,
    their_pk: ark_babyjubjub::EdwardsAffine,
) -> ark_babyjubjub::Fq {
    (their_pk * my_sk).into_affine().x
}

pub fn commit1(value: ark_bn254::Fr, r: ark_bn254::Fr) -> ark_bn254::Fr {
    const DS: u64 = 0xDEADBEEF;
    let ds = ark_bn254::Fr::from(DS);

    let mut state = [value + ds, r];
    poseidon2::bn254::t2::permutation_in_place(&mut state);
    state[0] + value
}

pub fn get_ds1() -> ark_bn254::Fr {
    // From SAFE-API paper (https://eprint.iacr.org/2023/522.pdf)
    // Absorb 2, squeeze 1,  domainsep = 0x4142
    // [0x80000002, 0x00000001, 0x4142]
    const DS_LO: u64 = 0x00020000_00014142;
    const DS_HI: u64 = 0x8000;
    ark_bn254::Fr::from_bigint(BigInt([DS_LO, DS_HI, 0, 0])).unwrap()
}

pub fn get_ds2() -> ark_bn254::Fr {
    // From SAFE-API paper (https://eprint.iacr.org/2023/522.pdf)
    // Absorb 2, squeeze 2,  domainsep = 0x4142
    // [0x80000002, 0x00000002, 0x4142]
    const DS_LO: u64 = 0x00020000_00024142;
    const DS_HI: u64 = 0x8000;
    ark_bn254::Fr::from_bigint(BigInt([DS_LO, DS_HI, 0, 0])).unwrap()
}

pub fn get_ds3() -> ark_bn254::Fr {
    // From SAFE-API paper (https://eprint.iacr.org/2023/522.pdf)
    // Absorb 2, squeeze 3,  domainsep = 0x4142
    // [0x80000002, 0x00000003, 0x4142]
    const DS_LO: u64 = 0x00020000_00034142;
    const DS_HI: u64 = 0x8000;
    ark_bn254::Fr::from_bigint(BigInt([DS_LO, DS_HI, 0, 0])).unwrap()
}

pub fn sym_encrypt1(
    key: ark_bn254::Fr,
    mut msg: ark_bn254::Fr,
    nonce: ark_bn254::Fr,
) -> ark_bn254::Fr {
    let ds = get_ds1();
    let mut state = [key, nonce, ds];
    poseidon2::bn254::t3::permutation_in_place(&mut state);
    msg += state[0];
    msg
}

pub fn sym_encrypt2(
    key: ark_bn254::Fr,
    mut msg: [ark_bn254::Fr; 2],
    nonce: ark_bn254::Fr,
) -> [ark_bn254::Fr; 2] {
    let ds = get_ds2();
    let mut state = [key, nonce, ds];
    poseidon2::bn254::t3::permutation_in_place(&mut state);
    for i in 0..2 {
        msg[i] += state[i];
    }
    msg
}

// pub fn sym_encrypt3(
//     key: ark_bn254::Fr,
//     mut msg: [ark_bn254::Fr; 3],
//     nonce: ark_bn254::Fr,
// ) -> [ark_bn254::Fr; 3] {
//     let ds = get_ds3();
//     let mut state = [key, nonce, ark_bn254::Fr::ZERO, ds];
//     poseidon2::bn254::t4::permutation_in_place(&mut state);
//     for i in 0..3 {
//         msg[i] += state[i];
//     }
//     msg
// }

pub fn sym_decrypt1(
    key: ark_bn254::Fr,
    mut ciphertext: ark_bn254::Fr,
    nonce: ark_bn254::Fr,
) -> ark_bn254::Fr {
    let ds = get_ds1();
    let mut state = [key, nonce, ds];
    poseidon2::bn254::t3::permutation_in_place(&mut state);
    ciphertext -= state[0];
    ciphertext
}

pub fn sym_decrypt2(
    key: ark_bn254::Fr,
    mut ciphertext: [ark_bn254::Fr; 2],
    nonce: ark_bn254::Fr,
) -> [ark_bn254::Fr; 2] {
    let ds = get_ds2();
    let mut state = [key, nonce, ds];
    poseidon2::bn254::t3::permutation_in_place(&mut state);
    for i in 0..2 {
        ciphertext[i] -= state[i];
    }
    ciphertext
}

// pub fn sym_decrypt3(
//     key: ark_bn254::Fr,
//     mut ciphertext: [ark_bn254::Fr; 3],
//     nonce: ark_bn254::Fr,
// ) -> [ark_bn254::Fr; 3] {
//     let ds = get_ds3();
//     let mut state = [key, nonce, ark_bn254::Fr::ZERO, ds];
//     poseidon2::bn254::t4::permutation_in_place(&mut state);
//     for i in 0..3 {
//         ciphertext[i] -= state[i];
//     }
//     ciphertext
// }
