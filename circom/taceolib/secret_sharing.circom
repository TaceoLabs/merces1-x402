pragma circom 2.2.2;


template additive_3rd_share() {
    signal input secret;
    signal input r[2];
    signal output share;

    share <== secret - r[0] - r[1];
}
