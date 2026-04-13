pragma circom 2.2.2;

include "commit.circom";
include "range.circom";

template deposit_inner() {
    signal input old_balance;
    signal input old_r;
    signal input amount;
    signal input new_r;
    signal output old_c;
    signal output new_c;

    signal new_balance <== old_balance + amount;
    var old_commitment = commit1()(old_balance, old_r);
    var new_commitment = commit1()(new_balance, new_r);
    old_c <== old_commitment;
    new_c <== new_commitment;
}

// Valid output:
// There is no sender, so we just return true
template deposit() {
    signal input old_balance;
    signal input old_r;
    signal input amount; // Public
    signal input new_r;
    signal output old_c;
    signal output new_c;
    signal output valid;

    component deposit = deposit_inner();
    deposit.old_balance <== old_balance;
    deposit.old_r <== old_r;
    deposit.amount <== amount;
    deposit.new_r <== new_r;
    old_c <== deposit.old_c;
    new_c <== deposit.new_c;

    valid <== 1;
}

// Valid output:
// We check whether the sender had enough balance.
template withdraw(BALANCE_BITSIZE) {
    signal input old_balance;
    signal input old_r;
    signal input amount; // Public
    signal input new_r;
    signal output old_c;
    signal output new_c;
    signal output valid;

    signal new_balance <== old_balance - amount;
    component new_balance_range_check = range_check_with_output_flag(BALANCE_BITSIZE);
    new_balance_range_check.in <== new_balance;
    valid <== new_balance_range_check.valid;
    var old_commitment = commit1()(old_balance, old_r);
    var new_commitment = commit1()(new_balance, new_r);
    old_c <== old_commitment;
    new_c <== new_commitment;
}

// Valid output:
// We check whether the sender had enough balance.
// We don't include amount range checks in valid since the registration of the transfer already includes this check. So at this point the amount is already proven to be in range.
template transfer(BALANCE_BITSIZE) {
    signal input sender_old_balance;
    signal input amount;
    signal input sender_new_r;
    signal output commitment[2]; 
    signal output valid;

    signal sender_new_balance <== sender_old_balance - amount;
    
    component pedersen = register_pending_transaction(BALANCE_BITSIZE);
    pedersen.amount <== sender_new_balance;
    pedersen.amount_he_r <== sender_new_r;
    commitment[0] <== pedersen.he_commitment[0];
    commitment[1] <== pedersen.he_commitment[1];
    valid <== pedersen.valid;
}
