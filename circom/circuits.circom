pragma circom 2.2.2;

include "taceolib/commit.circom";
include "taceolib/secret_sharing.circom";
include "taceolib/encryption.circom";
include "taceolib/merces.circom";
include "taceolib/compression.circom";

template transfer_batched(N, BALANCE_BITSIZE) {
    signal input sender_old_balance[N];
    signal input sender_old_r[N];
    signal input receiver_old_balance[N];
    signal input receiver_old_r[N];
    signal input amount[N];
    signal input amount_r[N];
    signal input sender_new_r[N];
    signal input receiver_new_r[N];
    signal output sender_old_commitment[N];
    signal output sender_new_commitment[N];
    signal output receiver_old_commitment[N];
    signal output receiver_new_commitment[N];
    signal output amount_commitment[N];
    signal output valid[N];

    component transactions[N];
    for (var i=0; i<N; i++) {
        transactions[i] = transfer(BALANCE_BITSIZE);
        transactions[i].sender_old_balance <== sender_old_balance[i];
        transactions[i].sender_old_r <== sender_old_r[i];
        transactions[i].receiver_old_balance <== receiver_old_balance[i];
        transactions[i].receiver_old_r <== receiver_old_r[i];
        transactions[i].amount <== amount[i];
        transactions[i].amount_r <== amount_r[i];
        transactions[i].sender_new_r <== sender_new_r[i];
        transactions[i].receiver_new_r <== receiver_new_r[i];

        sender_old_commitment[i] <== transactions[i].sender_old_c;
        sender_new_commitment[i] <== transactions[i].sender_new_c;
        receiver_old_commitment[i] <== transactions[i].receiver_old_c;
        receiver_new_commitment[i] <== transactions[i].receiver_new_c;
        amount_commitment[i] <== transactions[i].amount_c;
        valid[i] <== transactions[i].valid;
    }
}

template transfer_batched_compressed(N, BALANCE_BITSIZE, T) {
    signal input sender_old_balance[N];
    signal input sender_old_r[N];
    signal input receiver_old_balance[N];
    signal input receiver_old_r[N];
    signal input amount[N];
    signal input amount_r[N];
    signal input sender_new_r[N];
    signal input receiver_new_r[N];
    signal input alpha; // Public input for compression
    signal output beta;
    signal output gamma;

    // Calling the old component
    component transaction_batched = transfer_batched(N, BALANCE_BITSIZE);
    transaction_batched.sender_old_balance <== sender_old_balance;
    transaction_batched.sender_old_r <== sender_old_r;
    transaction_batched.receiver_old_balance <== receiver_old_balance;
    transaction_batched.receiver_old_r <== receiver_old_r;
    transaction_batched.amount <== amount;
    transaction_batched.amount_r <== amount_r;
    transaction_batched.sender_new_r <== sender_new_r;
    transaction_batched.receiver_new_r <== receiver_new_r;

    // Compressing the outputs
    var q[6 * N];
    for (var i = 0; i < N; i++) {
        q[6 * i] = transaction_batched.sender_old_commitment[i];
        q[6 * i + 1] = transaction_batched.sender_new_commitment[i];
        q[6 * i + 2] = transaction_batched.receiver_old_commitment[i];
        q[6 * i + 3] = transaction_batched.receiver_new_commitment[i];
        q[6 * i + 4] = transaction_batched.amount_commitment[i];
        q[6 * i + 5] = transaction_batched.valid[i];
    }

    component compression = Compression(6 * N, T);
    compression.q <== q;
    compression.alpha <== alpha;
    beta <== compression.beta;
    gamma <== compression.gamma;
}

template transfer_client(AMOUNT_BITSIZE) {
    // Transaction amount and randomness used for commitment
    signal input amount;
    signal input amount_r;
    // Encryptions
    signal input encrypt_sk;
    signal input mpc_pks[3][2]; // Public
    // Secret shares
    signal input share_amount[2];
    signal input share_amount_r[2];
    // Outputs
    signal output encrypt_pk[2];
    signal output amount_c;
    signal output ciphertexts[3][2];

    // 1. Commitment to the amount using the provided randomness including range check
    component amount_comm = check_amount(AMOUNT_BITSIZE);
    amount_comm.amount <== amount;
    amount_comm.amount_r <== amount_r;
    amount_c <== amount_comm.out;

     // 2. Additive secret sharing of amount and amount_r using
    signal share_amount_[3];
    signal share_amount_r_[3];
    for (var i = 0; i < 2; i++) {
        share_amount_[i] <== share_amount[i];
        share_amount_r_[i] <== share_amount_r[i];
    }
    share_amount_[2] <== additive_3rd_share()(amount, [share_amount_[0], share_amount_[1]]);
    share_amount_r_[2] <== additive_3rd_share()(amount_r, [share_amount_r_[0], share_amount_r_[1]]);

    // 3. Encryptions of secret shares
    // Ensure sk is in field Fr
    component sk_range_check = BabyJubJubIsInFr();
    sk_range_check.in <== encrypt_sk;
    // Encrypt secret shares
    for (var i = 0; i < 3; i++) {
        var symkey = derive_sym_key_bits()(sk_range_check.out_bits, mpc_pks[i]);
        ciphertexts[i] <== encrypt2()(symkey, 0, [share_amount_[i], share_amount_r_[i]]);
    }

    // 4. proof the correct public key was used for encryption
    component pk_calc = BabyJubJubScalarGeneratorBits();
    pk_calc.e <== sk_range_check.out_bits;
    encrypt_pk[0] <== pk_calc.out.x;
    encrypt_pk[1] <== pk_calc.out.y;
}
