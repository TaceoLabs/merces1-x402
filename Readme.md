# Source code for "Merces: Confidential Token Transfers via MPC and CoSNARKs"

This repository contains the source code for the paper "Merces: Confidential Token Transfers via MPC and CoSNARKs".
It is organized as follows:

- `circom`: Contains the ZK circuits written in the commonly used Circom language
- `client`: Contains the client code (e.g,. client transfer ZK proof generation) written in Rust
- `contract-rs`: Contains the interface to interact with the smart contract programmatically. The interface is written in Rust.
- `contracts`: Contains the source code of the smart contracts written in solidity. Note that the Groth16 verifiers are generated programmatically from the Circom source files.
- `e2e`: Contains an end-to-end test which interacts with anvil, acommonly used blockchain test environment.
- `mpc-nodes` Contains the MPC code, including the data structure, modifying the data structure in a batch, and creating the Groth16 proof in MPC.

## Instructions to run

We give instructions to run the end-to-end test on a linux system here.

### Setup of dependencies

To run the end-to-end test, you need to have Rust installed (see [https://rust-lang.org/tools/install/](https://rust-lang.org/tools/install/)):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```

To run the local blockchain test environment, anvil is required. Install it with the following commands (see [https://github.com/foundry-rs/foundry](https://github.com/foundry-rs/foundry)):

```bash
curl -L https://foundry.paradigm.xyz | bash
foundryup
```

### Get Repository

The anonymized repository is located at [https://anonymous.4open.science/r/Merces-46CE/](https://anonymous.4open.science/r/Merces-46CE/).

In order to run the code, you need to get 3 dependency git repository (for solidity code). The easiest way to setup everything correctly is just getting the `clone.py` script and execute it.
This will clone the anonymized repository into the newly created `Merces` folder and put all git-dependencies where they belong.

#### Manual setup

If you prefer a manual setup instead of running `clone.py`, download the anonymized repository as a zip folder and extract it. Go to the `contracts` folder inside the repository and execute the following commands

```bash
mkdir lib
git clone https://github.com/OpenZeppelin/openzeppelin-contracts
git clone https://github.com/foundry-rs/forge-std
git clone https://github.com/TaceoLabs/babyjubjub-solidity
cd openzeppelin-contracts && git checkout 8ff78ffb6e78463f070eab59487b4ba30481b53c
cd ../forge-std && git checkout 0e44f85a13976ba7491c6a9ee994b1a7efc3c281
cd ../babyjubjub-solidity && git checkout 4202e8794c44cf3f894c910354c0567b659e6669
cd ../..
```

### Run end-to-end test

To run the end-to-end test, you first need to start the local blockchain test environment `anvil`.
Run the following command in a terminal:

```bash
anvil
```

In a second terminal, go to the root folder of this repository and run:

```bash
cargo run --release --bin e2e
```

This runs the end-to-end test using the native token (i.e., ETH).
To run on a ERC20 token instead, use the following command:

```bash
cargo run --release --bin e2e -- -e
```

### What you will see

The end-to-end test first deploys all dependencies of the Merces smart contract, before deploying Merces itself.
If you use the `-e` flag to run Merces on ERC20 tokens, the end-to-end test will also deploy a standard ERC20 token contract and give two users, Alice and Bob, enough funds to run the test.

After deployment is done, the two users post the following intents on-chain:

- Alice first posts the intent of a deposit of some tokens, effectively shielding the tokens
- Alice then posts the intent of transferring the tokens privately to Bob
- Bob posts an intent for withdrawing the tokens it received
- Alice then posts another intent of transferring the same tokens to Bob. This intent will produce a valid ZK proof, but it will indicate that Alice has not enough balances to fulfill the request, so no token transfer will happen

After the intents are posted, we let the MPC network process the intents in a batch. Note that in a real deployment the MPC network will just query the smart contract action queue and process the read elements. In this end-to-end test, the MPC parties are instantiated as separate threads and connected with a localhost network.

The network will process the 4 intents it read from chain, and pad the batch with 46 dummy transactions. Once done, it will create a ZK proof of correctness, learn which intents produced invalid transactions and post the result on-chain. You will see a warning that one transaction was invalid (which is intended behavior).

After the MPC network posted the proof on-chain, the end-to-end test checks whether the on-chain balances of Alice and Bob changed according to our expectation. If no error is written, everything was done correctly.

Whenever the end-to-end test interacts with the smart contract, you will see it posting a transaction hash. Observe (in the terminal running anvil) that each transaction produced a state-update of the blockchain.
