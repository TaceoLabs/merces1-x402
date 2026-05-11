# Confidential x402 with Merces

## Overview

| Service | Description |
|---|---|
| `taceo-merces1-node` | MPC cluster; stores secret-shared balances; produces server proofs |
| `taceo-merces1-client` | Rust CLI client for performing transactions and querying balances |
| `taceo-merces1-faucet` | Faucet service for token distribution |
| `taceo-merces1-client-js` | Merces1 client in TypeScript |
| `taceo-merces1-x402` | Confidential x402 scheme in Rust |
| `taceo-merces1-x402-js` | Confidential x402 scheme in TypeScript |
| `taceo-merces1-x402-facilitator` | X402 facilitator service |
| `taceo-merces1-x402-server` | X402 server |
| `taceo-merces1-x402-react-app` | Confidential x402 demo frontend |

Smart contracts live in `contracts/`: `Merces.sol`, Groth16 verifiers, and test tokens.

## Confidential x402 Scheme

Checkout the Rust implementation [here](./taceo-merces1-x402/README.md) and the TypeScript implementation [here](./taceo-merces1-x402-js/README.md).

## Deployment

### Frontend

The confidential x402 demo frontend is deployed at: https://confidential-x402.taceo.io

### Wallets

- MPC Wallet: [0x26D5f6487DEf34B80a6F4B25f2d8c2566D6df86D](https://sepolia.basescan.org/address/0x26D5f6487DEf34B80a6F4B25f2d8c2566D6df86D)
- Faucet Wallet: [0x4DcdC198481d082912ddD3dE01459cb13926fdfC](https://sepolia.basescan.org/address/0x4DcdC198481d082912ddD3dE01459cb13926fdfC)
- x402 Facilitator Wallet: [0xAb7C0c4F2AaDA18cF385A6635caCC7D395C1f3E4](https://sepolia.basescan.org/address/0xAb7C0c4F2AaDA18cF385A6635caCC7D395C1f3E4)
- x402 Resource Server Wallet: [0x2AA787Ad0E04Ab8D02c4f3Fd3165e3FE6b1b3b05](https://sepolia.basescan.org/address/0x2AA787Ad0E04Ab8D02c4f3Fd3165e3FE6b1b3b05)

### Smart Contracts (Base Sepolia)

- USDC Contract: [0x4Ee80fFA1332525A8Cd100E1edf72Fe066f01c10](https://sepolia.basescan.org/address/0x4Ee80fFA1332525A8Cd100E1edf72Fe066f01c10)
- Merces Contract: [0x6AA4dd47444154A1E4424D08622EF6e96bf66de6](https://sepolia.basescan.org/address/0x6AA4dd47444154A1E4424D08622EF6e96bf66de6)

