pragma circom 2.2.2;

include "poseidon2.circom";
include "circomlib/aliascheck.circom";
include "circomlib/bitify.circom";
include "babyjubjub.circom";

template NO_TACEO_PRECOMPUTATION_Poseidon2(T) {
    signal input in[T];
    signal output out[T];

    out <== Poseidon2(T)(in);
}

template TACEO_PRECOMPUTATION_Num2Bits(n) {
    signal input in;
    signal output out[n];

    out <== Num2Bits(n)(in);
}

template TACEO_PRECOMPUTATION_PedersenCommitBits() {
    signal input value_bits[251];
    signal input r_bits[251];
    output BabyJubJubPoint() { twisted_edwards } out;

    component pedersen = PedersenCommitBits();
    pedersen.value_bits <== value_bits;
    pedersen.r_bits <== r_bits;


    out <== pedersen.out;
}

// Does not includes range checks!
template PedersenCommitBits() {
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

template TACEO_PRECOMPUTATION_AliasCheck() {
    signal input in[254];

    AliasCheck()(in);
}