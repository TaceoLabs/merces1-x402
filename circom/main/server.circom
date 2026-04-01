pragma circom 2.2.2;

include "circuits.circom";

// circom -l .. --O2 --r1cs server.circom

component main {public [alpha]} = transfer_batched_compressed(50, 100, 16);
