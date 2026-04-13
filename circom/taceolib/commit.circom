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
    component he_commit = pedersen_commit_bits();
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

// Does not includes range checks!
template pedersen_commit_bits() {
    signal input value_bits[251];
    signal input r_bits[251];
    output BabyJubJubPoint() { twisted_edwards } out;

    component g_value = BabyJubJubScalarGeneratorBits();
    g_value.e <== value_bits;

    // The default h-generator. We generated it from the hashing g.x to the curve using the implementation in this repo.
    var h_x = 18070489056226311699126950111606780081892760427770517382371397914121919205062;
    var h_y = 15271815330304366999180694217454548993927804584117026509847005260140807626286;
    component g_r = BabyJubJubScalarMulFixBits([h_x, h_y]);
    g_r.e <== r_bits;

    component add = BabyJubJubAdd();
    add.lhs <== g_value.out;
    add.rhs <== g_r.out;
    out <== add.out;
}