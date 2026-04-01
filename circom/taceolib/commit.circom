pragma circom 2.2.2;

include "precomputations.circom";

template commit1() {
    signal input value;
    signal input r;
    signal output out;

    var domain_sep = 0xDEADBEEF;
    var state[2] = TACEO_PRECOMPUTATION_Poseidon2(2)([value + domain_sep, r]);
    out <== state[0] + value;
}
