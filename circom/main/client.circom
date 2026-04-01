pragma circom 2.2.2;

include "circuits.circom";

// circom -l .. --O2 --r1cs client.circom

component main {public [mpc_pks]} = transfer_client(80);
