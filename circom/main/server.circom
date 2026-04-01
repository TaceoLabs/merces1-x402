pragma circom 2.2.2;

include "circuits.circom";

// circom -l .. --O2 --r1cs server.circom

component main = transfer_batched(50, 100);
