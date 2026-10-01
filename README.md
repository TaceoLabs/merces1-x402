# Confidential x402 with Merces

## Overview

| Service | Description |
| --- | --- |
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

The confidential x402 demo frontend is deployed at: <https://x402.merces-demo.taceo.io>

### Wallets

- MPC Wallet: [0x26D5f6487DEf34B80a6F4B25f2d8c2566D6df86D](https://sepolia.basescan.org/address/0x26D5f6487DEf34B80a6F4B25f2d8c2566D6df86D)
- Faucet Wallet: [0x4DcdC198481d082912ddD3dE01459cb13926fdfC](https://sepolia.basescan.org/address/0x4DcdC198481d082912ddD3dE01459cb13926fdfC)
- x402 Facilitator Wallet: [0xAb7C0c4F2AaDA18cF385A6635caCC7D395C1f3E4](https://sepolia.basescan.org/address/0xAb7C0c4F2AaDA18cF385A6635caCC7D395C1f3E4)
- x402 Resource Server Wallet: [0x2AA787Ad0E04Ab8D02c4f3Fd3165e3FE6b1b3b05](https://sepolia.basescan.org/address/0x2AA787Ad0E04Ab8D02c4f3Fd3165e3FE6b1b3b05)

### Smart Contracts (Base Sepolia)

- USDC Contract: [0x4Ee80fFA1332525A8Cd100E1edf72Fe066f01c10](https://sepolia.basescan.org/address/0x4Ee80fFA1332525A8Cd100E1edf72Fe066f01c10)
- Merces Contract: [0x2A07183Ec9cFFCED639C9Cb33BE106FD81d59E16](https://sepolia.basescan.org/address/0x2A07183Ec9cFFCED639C9Cb33BE106FD81d59E16)

### Disclaimer

This is a demo app only and is intentionally deployed on a test network. The code is not audited and deployments that handle real funds are discouraged and at your own risk.

The demo app also takes a few shortcuts for the demo's sake, e.g., the MPC nodes will can reveal the balances and transaction history of all users to any users.
This is an intentional choice for this demo to enable us to disclose the values behind commitments on chain to get a better view of them, and a real deployment would not have these routes available without authentication.

This version is also based on the first version of the Merces protocol with confidentiality only, and this codebase is not related to other Merces protocol instances that offer full on-chain privacy based on obvlious map constructions.
