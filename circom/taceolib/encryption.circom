pragma circom 2.2.2;

include "precomputations.circom";
include "babyjubjub.circom";

// Derive the symmetric keys for encryption
template derive_sym_key() {
    // My secret key
    input BabyJubJubScalarField() my_sk;
    // The other party's public key
    signal input pk[2];
    signal output key;

    // SAFETY: pk_p is on the curve and in the correct subgroup, guaranteed outside of the ZK proof as this is a public input.
    BabyJubJubPoint() { twisted_edwards_in_subgroup } pk_p;
    pk_p.x <== pk[0];
    pk_p.y <== pk[1];
    component sym_key = BabyJubJubScalarMul();
    sym_key.p <== pk_p;
    sym_key.e <== my_sk;

    key <== sym_key.out.x;
}


// Derive the symmetric keys for encryption
template derive_sym_key_bits() {
    // My secret key
    signal input my_sk_bits[251];
    // The other party's public key
    signal input pk[2];
    signal output key;

    // SAFETY: pk_p is on the curve and in the correct subgroup, guaranteed outside of the ZK proof as this is a public input.
    BabyJubJubPoint() { twisted_edwards_in_subgroup } pk_p;
    pk_p.x <== pk[0];
    pk_p.y <== pk[1];
    component sym_key = BabyJubJubScalarMulBits();
    sym_key.p <== pk_p;
    sym_key.e <== my_sk_bits;

    key <== sym_key.out.x;
}

template encrypt1() {
    signal input key;
    signal input nonce;
    signal input message;
    signal output cipher;

    // From SAFE-API paper (https://eprint.iacr.org/2023/522.pdf)
    // Absorb 2, squeeze 1,  domainsep = 0x4142
    // [0x80000002, 0x00000001, 0x4142]
    var DS = 0x80000002000000014142;
    var poseidon2_cipher_state[3] = TACEO_PRECOMPUTATION_Poseidon2(3)([key, nonce, DS]);
    cipher <== poseidon2_cipher_state[0] + message;
}

template decrypt1() {
    signal input key;
    signal input nonce;
    signal input cipher;
    signal output message;

    // From SAFE-API paper (https://eprint.iacr.org/2023/522.pdf)
    // Absorb 2, squeeze 1,  domainsep = 0x4142
    // [0x80000002, 0x00000001, 0x4142]
    var DS = 0x80000002000000014142;
    var poseidon2_cipher_state[3] = TACEO_PRECOMPUTATION_Poseidon2(3)([key, nonce, DS]);
    message <== cipher - poseidon2_cipher_state[0];
}

template encrypt2() {
    signal input key;
    signal input nonce;
    signal input message[2];
    signal output cipher[2];

    // From SAFE-API paper (https://eprint.iacr.org/2023/522.pdf)
    // Absorb 2, squeeze 2,  domainsep = 0x4142
    // [0x80000002, 0x00000002, 0x4142]
    var DS = 0x80000002000000024142;
    var poseidon2_cipher_state[3] = TACEO_PRECOMPUTATION_Poseidon2(3)([key, nonce, DS]);
    for (var i = 0; i < 2; i++) {
        cipher[i] <== poseidon2_cipher_state[i] + message[i];
    }
}

template decrypt2() {
    signal input key;
    signal input nonce;
    signal input cipher[2];
    signal output message[2];

    // From SAFE-API paper (https://eprint.iacr.org/2023/522.pdf)
    // Absorb 2, squeeze 2,  domainsep = 0x4142
    // [0x80000002, 0x00000002, 0x4142]
    var DS = 0x80000002000000024142;
    var poseidon2_cipher_state[3] = TACEO_PRECOMPUTATION_Poseidon2(3)([key, nonce, DS]);
    for (var i = 0; i < 2; i++) {
        message[i] <== cipher[i] - poseidon2_cipher_state[i];
    }
}

template encrypt3() {
    signal input key;
    signal input nonce;
    signal input message[3];
    signal output cipher[3];

    // From SAFE-API paper (https://eprint.iacr.org/2023/522.pdf)
    // Absorb 2, squeeze 3,  domainsep = 0x4142
    // [0x80000002, 0x00000003, 0x4142]
    var DS = 0x80000002000000034142;
    var poseidon2_cipher_state[4] = TACEO_PRECOMPUTATION_Poseidon2(4)([key, nonce, 0, DS]);
    for (var i = 0; i < 3; i++) {
        cipher[i] <== poseidon2_cipher_state[i] + message[i];
    }
}

template decrypt3() {
    signal input key;
    signal input nonce;
    signal input cipher[3];
    signal output message[3];

    // From SAFE-API paper (https://eprint.iacr.org/2023/522.pdf)
    // Absorb 2, squeeze 3,  domainsep = 0x4142
    // [0x80000002, 0x00000003, 0x4142]
    var DS = 0x80000002000000034142;
    var poseidon2_cipher_state[4] = TACEO_PRECOMPUTATION_Poseidon2(4)([key, nonce, 0, DS]);
    for (var i = 0; i < 3; i++) {
        message[i] <== cipher[i] - poseidon2_cipher_state[i];
    }
}
