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
    component new_balance_range_check = range_check_with_output_flag(125);
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
    signal input sender_old_r;
    signal input receiver_old_balance;
    signal input receiver_old_r;
    signal input amount;
    signal input amount_r;
    signal input sender_new_r;
    signal input receiver_new_r;
    signal output sender_old_c;
    signal output sender_new_c;
    signal output receiver_old_c;
    signal output receiver_new_c;
    signal output amount_c;
    signal output valid;

    // No range check, since the registration of the transfer already includes this check. So at this point the amount is already proven to be in range.
    amount_c <== commit1()(amount, amount_r);

    component withdraw = withdraw(BALANCE_BITSIZE);
    withdraw.old_balance <== sender_old_balance;
    withdraw.old_r <== sender_old_r;
    withdraw.amount <== amount;
    withdraw.new_r <== sender_new_r;
    sender_old_c <== withdraw.old_c;
    sender_new_c <== withdraw.new_c;
    valid <== withdraw.valid;

    component deposit = deposit_inner();
    deposit.old_balance <== receiver_old_balance;
    deposit.old_r <== receiver_old_r;
    deposit.amount <== amount;
    deposit.new_r <== receiver_new_r;
    receiver_old_c <== deposit.old_c;
    receiver_new_c <== deposit.new_c;
}
