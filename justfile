PRIVATE_KEY := "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
MPC_ADDRESS := "0x70997970C51812dc3A010C7d01b50e0d17dc79C8"
MPC_PRIVATE_KEY := "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d"
FACILITATOR_PRIVATE_KEY := "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a"
FAUCET_PRIVATE_KEY := "0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6"
X402_SERVER_ADDRESS := "0x23618e81E3f5cdF7f54C3d65f7FBc0aBf5B21E8f"
X402_CLIENT_ADDRESS := "0xa0Ee7A142d267C1f36714E4a8F75612F20a79720"
PTAU_FILE := "phase2_17.ptau"
PTAU_URL := "https://storage.googleapis.com/zkevm/ptau/powersOfTau28_hez_final_17.ptau"

[private]
default:
    @just --justfile {{ justfile() }} --list --list-heading $'Project commands:\n'

lint:
    cargo fmt --all -- --check
    cargo clippy --workspace --tests --examples --benches --bins -q -- -D warnings
    RUSTDOCFLAGS='-D warnings' cargo doc --workspace -q --no-deps --document-private-items
    cd contracts && forge fmt --check

[working-directory('contracts')]
show-contract-errors:
    forge inspect src/Merces.sol:Merces errors
    forge inspect src/verifiers/VerifierClient.sol:Verifier errors
    forge inspect src/verifiers/VerifierServer.sol:Verifier errors
    forge inspect src/Token.sol:USDCToken errors
    forge inspect src/Token.sol:USDT0Token errors

[working-directory('circom')]
build-zk-artifacts:
    #!/usr/bin/env bash

    CIRCOM_CIRCUITS=("client" "server")
    SOLIDITY_VERIFIERS=("VerifierClient" "VerifierServer")

    PTAU_PATH="{{ PTAU_FILE }}"
    if [[ ! -f "$PTAU_PATH" ]]; then
        echo "ptau file not found, downloading from {{ PTAU_URL }}"
        curl -fL --retry 3 --retry-delay 2 "{{ PTAU_URL }}" -o "${PTAU_PATH}.tmp"
        mv "${PTAU_PATH}.tmp" "$PTAU_PATH"
    fi

    for CIRCUIT in "${CIRCOM_CIRCUITS[@]}"; do
        echo "Compiling ${CIRCUIT}.circom..."
        circom -l . --O2 --r1cs "main/${CIRCUIT}.circom"
        mv "${CIRCUIT}.r1cs" "r1cs/${CIRCUIT}.r1cs"

        echo "Generating zkey for ${CIRCUIT} circuit..."
        snarkjs groth16 setup "r1cs/${CIRCUIT}.r1cs" "$PTAU_PATH" "${CIRCUIT}_circuit.zkey"
        snarkjs zkey contribute "${CIRCUIT}_circuit.zkey" "${CIRCUIT}_circuit_final.zkey" --name="1st Contributor Name" -v -e="some random text"
        rm "${CIRCUIT}_circuit.zkey"
        snarkjs zkey export verificationkey "${CIRCUIT}_circuit_final.zkey" "${CIRCUIT}_verification_key.json"
        convert-zkey-to-ark --zkey-path "${CIRCUIT}_circuit_final.zkey" --uncompressed
        mv "${CIRCUIT}_circuit_final.zkey" "artifacts/${CIRCUIT}.zkey"
        mv arks.zkey "artifacts/${CIRCUIT}.arks.zkey"
        mv "${CIRCUIT}_verification_key.json" "artifacts/${CIRCUIT}_verification_key.json"

        echo "Generating Solidity verifier for ${CIRCUIT} circuit..."
        groth16-sol-utils extract-verifier --vk "artifacts/${CIRCUIT}_verification_key.json" > "${CIRCUIT}.sol"
        mv "${CIRCUIT}.sol" "../contracts/src/verifiers/Verifier${CIRCUIT^}.sol"

        # if client generate wasm as well ad copy wasm + zkey to client js package
        if [[ "$CIRCUIT" == "client" ]]; then
            echo "Generating wasm for ${CIRCUIT} circuit..."
            circom -l . --O2 --wasm "main/${CIRCUIT}.circom"
            mv "${CIRCUIT}_js/${CIRCUIT}.wasm" ../taceo-merces1-client-js/
            rm -rf "${CIRCUIT}_js"
            cp "artifacts/${CIRCUIT}.zkey" ../taceo-merces1-client-js/
        fi
    done

    cd ../contracts && forge fmt 

[working-directory('contracts')]
deploy-merces-anvil $TOKEN_ADDRESS="0x0000000000000000000000000000000000000000":
    MPC_ADDRESS={{ MPC_ADDRESS }} \
    MPC_PK1_X=20753332016692298037070725519498706856018536650957009186217190802393636394798 \
    MPC_PK1_Y=7870889370474934069210756140130118230952037969542869026332032190368575018928 \
    MPC_PK2_X=20753332016692298037070725519498706856018536650957009186217190802393636394798 \
    MPC_PK2_Y=7870889370474934069210756140130118230952037969542869026332032190368575018928 \
    MPC_PK3_X=20753332016692298037070725519498706856018536650957009186217190802393636394798 \
    MPC_PK3_Y=7870889370474934069210756140130118230952037969542869026332032190368575018928 \
    MERCES_DEPLOYMENT=test \
    forge script script/DeployMerces.s.sol:DeployMerces \
        --rpc-url http://localhost:8545 \
        --private-key {{ PRIVATE_KEY }} \
        --broadcast \
        -vvv

[working-directory('contracts')]
deploy-token-anvil:
    forge script script/DeployToken.s.sol:DeployToken \
        --rpc-url http://localhost:8545 \
        --private-key {{ FAUCET_PRIVATE_KEY }} \
        --broadcast \
        -vvv

run-setup $TOKEN="erc20":
    #!/usr/bin/env bash

    set -eu

    wait_for_health() {
        local name=$1
        local port=$2
        local timeout=120
        local start_time=$(date +%s)
        echo "waiting for $name on port $port to be healthy..."
        while true; do
            http_code=$(curl -s -o /dev/null -w "%{http_code}" "http://127.0.0.1:$port/health" || echo "000")
            if [[ "$http_code" == "200" ]]; then
                echo "$name is healthy!"
                break
            fi
            now=$(date +%s)
            if (( now - start_time >= timeout )); then
                echo "error: $name did not become healthy after $timeout seconds" >&2
                exit 1
            fi
            sleep 1
        done
    }

    teardown() {
        docker compose -f docker-compose.test.yml down || true
        killall -9 taceo-merces1-node 2>/dev/null || true
        killall -9 taceo-merces1-x402-facilitator 2>/dev/null || true
        killall -9 taceo-merces1-x402-server 2>/dev/null || true
        killall -9 taceo-merces1-faucet 2>/dev/null || true
        killall -9 anvil 2>/dev/null || true
    }


    rm -rf logs
    mkdir -p logs
    cargo build --workspace --release
    teardown

    trap teardown EXIT SIGINT SIGTERM

    anvil > logs/anvil.log 2>&1 &

    DB_URL="postgres://postgres:postgres@localhost:5432/postgres"
    docker compose -f docker-compose.test.yml up -d db

    sleep 1

    echo "deploying contracts.."
    if [[ $TOKEN == "native" ]]; then
        token_contract="0x0000000000000000000000000000000000000000"
    elif [[ $TOKEN == "erc20" ]]; then
        just deploy-token-anvil > logs/deploy-token.log 2>&1
        token_contract=$(jq -r '.transactions[] | select(.contractName == "USDCToken") | .contractAddress' ./contracts/broadcast/DeployToken.s.sol/31337/run-latest.json)
    else
        echo "unsupported token type: $TOKEN" >&2
        exit 1
    fi
    just deploy-merces-anvil $token_contract > logs/deploy-merces.log 2>&1
    merces_contract=$(jq -r '.transactions[] | select(.contractName == "Merces") | .contractAddress' ./contracts/broadcast/DeployMerces.s.sol/31337/run-latest.json)
    # Inject placeholder bytecode at the addresses that x402-chain-eip155 requires to exist on startup.
    # These contracts (ERC-6492 Validator, Uniswap Permit2, x402 ExactPermit2Proxy) are not used by
    # V2Eip155Confidential, but the library checks for them unconditionally. Safe for local Anvil only.
    cast rpc anvil_setCode "0xdAcD51A54883eb67D95FAEb2BBfdC4a9a6BD2a3B" "0x00" --rpc-url http://localhost:8545 > /dev/null
    cast rpc anvil_setCode "0x000000000022D473030F116dDEE9F6B43aC78BA3" "0x00" --rpc-url http://localhost:8545 > /dev/null
    cast rpc anvil_setCode "0x402085c248EeA27D92E8b30b2C58ed07f9E20001" "0x00" --rpc-url http://localhost:8545 > /dev/null

    echo "starting nodes.."
    for i in 0 1 2; do
        RUST_LOG=taceo=debug,info \
        MERCES1_NODE__BIND_ADDR=0.0.0.0:1001$i \
        MERCES1_NODE__SERVICE__MPC_BIND_ADDR=0.0.0.0:1000$i \
        MERCES1_NODE__SERVICE__PARTY_ID=$i \
        MERCES1_NODE__SERVICE__NODE_ADDRS=localhost:10000,localhost:10001,localhost:10002 \
        MERCES1_NODE__SERVICE__MERCES_CONTRACT=$merces_contract \
        MERCES1_NODE__SERVICE__WALLET_PRIVATE_KEY={{ MPC_PRIVATE_KEY }} \
        MERCES1_NODE__SERVICE__ENVIRONMENT=dev \
        MERCES1_NODE__SERVICE__MPC_SK=43 \
        MERCES1_NODE__SERVICE__RPC__HTTP_URLS=http://localhost:8545 \
        MERCES1_NODE__SERVICE__POSTGRES__CONNECTION_STRING=$DB_URL \
        MERCES1_NODE__SERVICE__POSTGRES__SCHEMA=node$i \
            ./target/release/taceo-merces1-node > logs/node$i.log 2>&1 &
    done
    for i in 0 1 2; do
        wait_for_health "node$i" 1001$i
    done

    echo "starting faucet.."
    RUST_LOG=taceo=debug,info \
    MERCES1_FAUCET__BIND_ADDR=0.0.0.0:8082 \
    MERCES1_FAUCET__SERVICE__ENVIRONMENT=dev \
    MERCES1_FAUCET__SERVICE__WALLET_PRIVATE_KEY={{ FAUCET_PRIVATE_KEY }} \
    MERCES1_FAUCET__SERVICE__MERCES_CONTRACT=$merces_contract \
    MERCES1_FAUCET__SERVICE__TOKEN=$token_contract \
    MERCES1_FAUCET__SERVICE__RPC__HTTP_URLS=http://localhost:8545 \
        ./target/release/taceo-merces1-faucet > logs/faucet.log 2>&1 &
    wait_for_health "faucet" 8082

    echo "starting x402 facilitator.."
    RUST_LOG=taceo=debug,x402=debug,info \
    MERCES1_X402_FACILITATOR__BIND_ADDR=0.0.0.0:8080 \
    MERCES1_X402_FACILITATOR__SERVICE__NODE_URLS=http://localhost:10010,http://localhost:10011,http://localhost:10012 \
    MERCES1_X402_FACILITATOR__SERVICE__MERCES_CONTRACT=$merces_contract \
    MERCES1_X402_FACILITATOR__SERVICE__WALLET_PRIVATE_KEY={{ FACILITATOR_PRIVATE_KEY }} \
    MERCES1_X402_FACILITATOR__SERVICE__ENVIRONMENT=dev \
    MERCES1_X402_FACILITATOR__SERVICE__MPC_PKS="20753332016692298037070725519498706856018536650957009186217190802393636394798,7870889370474934069210756140130118230952037969542869026332032190368575018928,20753332016692298037070725519498706856018536650957009186217190802393636394798,7870889370474934069210756140130118230952037969542869026332032190368575018928,20753332016692298037070725519498706856018536650957009186217190802393636394798,7870889370474934069210756140130118230952037969542869026332032190368575018928" \
    MERCES1_X402_FACILITATOR__SERVICE__RPC__HTTP_URLS=http://localhost:8545 \
        ./target/release/taceo-merces1-x402-facilitator --config taceo-merces1-x402-facilitator/config.json > logs/x402-facilitator.log 2>&1 &
    wait_for_health "x402-facilitator" 8080

    echo "starting x402 server.."
    RUST_LOG=taceo=debug,x402=debug,info \
    MERCES1_X402_SERVER__BIND_ADDR=0.0.0.0:8081 \
    MERCES1_X402_SERVER__SERVICE__ENVIRONMENT=dev \
    MERCES1_X402_SERVER__SERVICE__FACILITATOR_URL=http://localhost:8080 \
    MERCES1_X402_SERVER__SERVICE__PAY_TO={{ X402_SERVER_ADDRESS }} \
        ./target/release/taceo-merces1-x402-server > logs/x402-server.log 2>&1 &
    wait_for_health "x402-server" 8081

    echo "setup started!"

    wait

run-x402-client:
    #!/usr/bin/env bash
    set -eu
    echo "claim funds from faucet.."
    curl -X POST "http://localhost:8082/claim/{{ X402_CLIENT_ADDRESS }}"
    echo ""
    echo "balance before: $(just get-balance {{ X402_CLIENT_ADDRESS }})"
    echo "running x402 client example.."
    ./target/release/examples/taceo-merces1-x402-client
    echo "balance before: $(just get-balance {{ X402_CLIENT_ADDRESS }})"

run-x402-client-js:
    #!/usr/bin/env bash
    set -eu
    echo "claim funds from faucet.."
    curl -X POST "http://localhost:8082/claim/{{ X402_CLIENT_ADDRESS }}"
    echo ""
    echo "balance before: $(just get-balance {{ X402_CLIENT_ADDRESS }})"
    echo "running x402 js client example.."
    cd taceo-merces1-x402-js && npm run example:client
    echo "balance after: $(just get-balance {{ X402_CLIENT_ADDRESS }})"

get-balance address:
    #!/usr/bin/env python3
    import urllib.request
    BN254_PRIME = 0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001
    node_urls = ["http://localhost:10010", "http://localhost:10011", "http://localhost:10012"]
    shares = [int(urllib.request.urlopen(f"{url}/balance/{{ address }}").read()) for url in node_urls]
    balance = sum(shares) % BN254_PRIME
    print(f"{balance}")
