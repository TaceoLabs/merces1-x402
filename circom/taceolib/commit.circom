pragma circom 2.2.2;

include "precomputations.circom";
include "babyjubjub.circom";

template commit1() {
    signal input value;
    signal input r;
    signal output out;

    var domain_sep = 0xDEADBEEF;
    var state[2] = NO_TACEO_PRECOMPUTATION_Poseidon2(2)([value + domain_sep, r]);
    out <== state[0] + value;
}

template register_pending_transaction(AMOUNT_BITSIZE) {
    signal input amount;
    signal input amount_he_r; // For the homomorphic commitment
    signal output he_commitment[2];
    signal output valid;

    assert(AMOUNT_BITSIZE <= 125); // Amount is positive, also in Babyjubjub ScalarField

    component range_check = range_check_with_output_flag(AMOUNT_BITSIZE);
    range_check.in <== amount;
    valid <== range_check.valid;

    component r_range_check = BabyJubJubIsInFr();
    r_range_check.in <== amount_he_r;
    // Commit
    component he_commit = TACEO_PRECOMPUTATION_PedersenCommitBits();
    for (var i = 0; i < AMOUNT_BITSIZE; i++) {
        he_commit.value_bits[i] <== range_check.in_bits[i];
    }
    for (var i = AMOUNT_BITSIZE; i < 251; i++) {
        he_commit.value_bits[i] <== 0;
    }
    he_commit.r_bits <== r_range_check.out_bits;
    he_commitment[0] <== he_commit.out.x;
    he_commitment[1] <== he_commit.out.y;
}

