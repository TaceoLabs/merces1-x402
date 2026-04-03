pragma circom 2.2.2;

include "precomputations.circom";

template Compression(N, T) {
    signal input q[N]; // Original public inputs, now private!
    signal input alpha; // Hash of q using SHA256, public!
    signal output beta;
    signal output gamma;

    // We call this function to be able to set inputs to be public in the Circom-MPC-VM
    signal public_q[N] <== ToPublic(q);
    for (var i = 0; i < N; i++) {
        public_q[i] === q[i];
    }

    // Compute beta using Poseidon2 sponge
    beta <== Poseidon2Sponge(N, T)(public_q);

    // Compute gamma using UHF
    gamma <== UHF(N)(alpha, beta, public_q);
}

function ToPublic(in) {
    return in;
}

template Poseidon2Sponge(N, T) {
    signal input in[N];
    signal output out;

    assert(T >= 2); // Minimum state size for Poseidon2
    assert(N >= 1); // Must absorb at least one element

    var ds = 0xDEADBEEF;
    var permutations = (N + T - 2) \ (T-1); // ceil( N / (T - 1))
    var states[permutations + 1][T];

    // Initialize the state
    for (var i = 0; i < T - 1; i++) {
        states[0][i] = 0;
    }
    states[0][T - 1] = ds;

    // Absorb and permute
    var absorbed = 0;
    for (var p = 0; p < permutations; p++) {
        var remaining = N - absorbed;
        if (remaining > T - 1) {
            remaining = T - 1;
        }
        for (var i = 0; i < remaining; i++) {
            states[p][i] = states[p][i] + in[absorbed + i];
        }
        absorbed += remaining;
        states[p + 1] = Poseidon2NoAccelerator(T)(states[p]);
    }
    out <== states[permutations][0];
}

template UHF(N) {
    signal input alpha;
    signal input beta;
    signal input x[N];
    signal output gamma;

    assert(N >= 1); // The degree of the polynomial is at least zero

    signal seed <== alpha + beta;
    signal muls[N];
    muls[N - 1] <== 0;
    for (var i = N - 1; i > 0; i--) {
        muls[i - 1] <== seed * (muls[i] + x[i]);
    }
    gamma <== muls[0] + x[0];
}
