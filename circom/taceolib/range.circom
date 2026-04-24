pragma circom 2.2.2;

include "precomputations.circom";
include "circomlib/comparators.circom";

template range_check_with_output_flag(BITSIZE) {
    assert(BITSIZE <= 254);
    assert(BITSIZE > 0);
    signal input in;
    signal output valid;
    signal output in_bits[BITSIZE];

    // Num2Bits_strict with taceo_precomputation
    component n2b = TACEO_PRECOMPUTATION_Num2Bits(254);
    in ==> n2b.in;

    TACEO_PRECOMPUTATION_AliasCheck()(n2b.out); 
    
    for (var i=0; i<BITSIZE; i++) {
        in_bits[i] <== n2b.out[i];
    }

    // Sum up all bits above BITSIZE
    // Works since bits are enforced to be 0 or 1 already.
    // Thus this sum cannot overflow and if at least one bit is 1, sum > 0
    var sum = 0;
    for (var i=BITSIZE; i<254; i++) {
        sum += n2b.out[i];
    }

    valid <== TACEO_PRECOMPUTATION_IsZero()(sum);
}

// Checks the size of amount and computes a commitment
template check_amount(AMOUNT_BITSIZE) {
    signal input amount;
    signal input amount_r;
    signal output out;
    signal output out_bits[AMOUNT_BITSIZE];

    out_bits <== TACEO_PRECOMPUTATION_Num2Bits(AMOUNT_BITSIZE)(amount);
    out <== commit1()(amount, amount_r);
}
