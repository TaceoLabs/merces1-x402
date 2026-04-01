pragma circom 2.2.2;

include "circuits.circom";

// circom -l .. --O2 --r1cs client.circom

component main {public [alpha]} = transfer_client_compressed(80, 16);
