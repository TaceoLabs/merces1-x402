#!/usr/bin/env bash

# CIRCOM Files
CIRCOM_CIRCUITS=("client" "server")

cd circom/main
for CIRCUIT in "${CIRCOM_CIRCUITS[@]}"; do
    circom -l .. --O2 --r1cs ${CIRCUIT}.circom
    mv ${CIRCUIT}.r1cs ../r1cs/${CIRCUIT}.r1cs
done
cd ../..

# CIRCOM graph files
CIRCOM_GRAPH_CIRCUITS=("client")

cd circom-witness-rs
for CIRCUIT in "${CIRCOM_GRAPH_CIRCUITS[@]}"; do
    WITNESS_CPP=../circom/main/${CIRCUIT}.circom CIRCOM_LIBRARY_PATH=../circom/ cargo run --bin generate-graph --features build-witness
    mv graph.bin ../circom/graph/${CIRCUIT}_graph.bin
done
cd ..

# Solidity JSON
cd contracts
forge build --silent
rm -rf json
mkdir json
cp out/BabyJubJub.sol/BabyJubJub.json json/BabyJubJub.json
cp out/Merces.sol/Merces.json json/Merces.json
cp out/Token.sol/USDCToken.json json/USDCToken.json
cp out/VerifierClient.sol/Verifier.json json/VerifierClient.json
cp out/VerifierServer.sol/Verifier.json json/VerifierServer.json
cd ..
