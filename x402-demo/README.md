# x402 Merces Demo (local)

End-to-end local demo of the [x402 payment protocol](https://github.com/coinbase/x402)
running against a real Merces 3-party MPC for confidential USDC payments. No
testnet, no faucets — one script spins up anvil, the MPC, the facilitator, and
a resource server, then an agent makes three paid requests with compressed
Groth16 ZK proofs verified on-chain.

## Architecture

```
 Agent                   Server                  Facilitator              Merces (anvil)
   │                        │                         │                         │
   │── GET /v1/sentiment ───┤                         │                         │
   │                        │                         │                         │
   │◀────── 402 + paymentRequirements ────────────────┤                         │
   │                                                                            │
   │── POST /prove ──▶ prove sidecar :4024  (Rust, 78ms compressed Groth16)     │
   │   (agent signs EIP-712 TransferFromAuthorization on returned ciphertext)   │
   │                                                                            │
   │── GET /v1/sentiment (with X-Payment header) ─▶ Server ──▶ Facilitator      │
   │                                                                │           │
   │                                                                │── verify + transferFrom() ──▶
   │                                                                            │
   │                                                         mpc_service :4025 polls
   │                                                         ┌─ readQueue                          │
   │                                                         ├─ 3-party MPC in-process (threads)   │
   │                                                         └─ processMPC() ──▶ balance updated   │
   │                                                                            │
   │◀──────── 200 + response + X-Payment-Response ─────────────────────────────┤
```

## Services

| Port | Process        | What it does                                                      |
| ---- | -------------- | ----------------------------------------------------------------- |
| 8545 | `anvil`        | Local EVM                                                         |
| 4024 | `prove`        | Rust HTTP sidecar — client compressed-proof gen (~78 ms)          |
| 4025 | `mpc_service`  | Rust HTTP sidecar — wraps `e2e::mpc::Mpc`, polls Merces queue     |
| 4022 | `facilitator`  | TS x402 facilitator, submits `transferFrom()` on Merces           |
| 4021 | `server`       | TS resource server exposing `/v1/sentiment` (paywalled $0.05 USDC) |

## Prerequisites

1. **Foundry** — `anvil` must be on your path.
   <https://book.getfoundry.sh>

2. **Rust toolchain** — to build `prove` and `mpc_service`.

3. **pnpm** v10+.

4. **`@taceo/x402-evm` must be published to npm.** This package is the Taceo
   fork of `@x402/evm` with the confidential-payments scheme (`confidential/`
   subpath) added as a sibling of `exact/`. Upstream `@x402/core`,
   `@x402/express`, `@x402/axios` are consumed unmodified from npm at v2.10.0.

   To publish `@taceo/x402-evm` for the first time:

   ```bash
   # Prereqs (one-time):
   #  - create the `@taceo` scope on npm: https://www.npmjs.com/org/create
   #  - npm login (or set NPM_TOKEN) with publish rights to @taceo

   git clone --branch feat/real-mpc-integration https://github.com/TaceoLabs/x402
   cd x402/typescript
   pnpm install
   pnpm --filter @x402/evm build

   cd packages/mechanisms/evm
   ./publish-taceo.sh --dry-run     # sanity check
   ./publish-taceo.sh               # real publish
   ```

   The script renames the package to `@taceo/x402-evm` at publish time and
   rewrites `workspace:~` deps to concrete versions, so the published package
   resolves `@x402/core` + `@x402/extensions` from upstream npm.

   > After Coinbase merges the confidential scheme into upstream `@x402/evm`,
   > this prerequisite disappears — just drop the `@taceo/x402-evm` dep and
   > retarget the imports at `@x402/evm/confidential/*`.

## Build the Rust bins

From the repo root (not this directory):

```bash
cd ..
cargo build --release --bin prove --bin mpc_service
# Also make sure contract JSONs exist (these are checked in, but if you touch Merces.sol
# run forge build + cp out/Merces.sol/Merces.json contracts/json/Merces.json)
```

## Install TS deps

```bash
pnpm install      # run from this directory
```

This resolves `@taceo/x402-evm` from npm alongside upstream `@x402/core`,
`@x402/axios`, `@x402/express`.

## Run the demo

```bash
./start-local.sh
```

You should see:

```
[demo] Anvil ready (chain 31337)
[demo] Prove sidecar ready
[demo] MPC service ready
[demo] Contracts deployed — mpc_service now polling
[demo] Facilitator ready
[demo] Server ready

  ✓ anvil           :8545
  ✓ prove           :4024
  ✓ mpc_service     :4025
  ✓ facilitator     :4022
  ✓ server          :4021

[Agent] Request 1/3: Fetching sentiment for ETH...
[Agent] Response: ETH — sentiment=0.73, signal=bullish
[Agent] Payment settled — tx: 0x...
[Agent] Compressed ZK proof verified on-chain
... (×3)
```

### Stop everything

```bash
./start-local.sh --stop
```

## Manual run (without start-local.sh)

```bash
# Terminal 1
anvil --silent

# Terminal 2
../target/release/prove

# Terminal 3
../target/release/mpc_service

# Terminal 4 — deploy (fetches pubkeys from :4025, seeds 100 USDC, calls /start)
pnpm run deploy

# Terminal 5
pnpm run facilitator

# Terminal 6
pnpm run server

# Terminal 7
pnpm run agent
```

## What's where

| File                  | Role                                                                       |
| --------------------- | -------------------------------------------------------------------------- |
| `deploy.ts`           | Fetches MPC pubkeys from `:4025/pubkeys`, deploys Merces + verifiers + USDC, mints + deposits 100 USDC for the agent, POSTs `:4025/start`. |
| `facilitator.ts`      | x402 facilitator. Registers the confidential scheme (imported from `@taceo/x402-evm/confidential/facilitator`) for chain 31337. Submits `transferFrom()` on-chain. |
| `server.ts`           | Resource server exposing `/v1/sentiment`. Returns 402 with confidential paymentRequirements. |
| `agent.ts`            | Makes 3 paid requests. Calls `prove` sidecar for each proof, signs EIP-712 transferFrom authorization. |
| `start-local.sh`      | One-command launcher. Checks prereqs, boots services in dependency order. |

## Known issues / TODOs

- **Upstream the confidential scheme to Coinbase x402.** Once merged, the
  `@taceo/x402-evm` prerequisite disappears — swap imports from
  `@taceo/x402-evm/confidential/*` → `@x402/evm/confidential/*` and drop the
  `@taceo/x402-evm` dep. See `TaceoLabs/x402` `feat/real-mpc-integration`
  for the staged PR.
- **Publish `@taceo/x402-evm` via CI.** Right now it's a manual
  `./publish-taceo.sh` in the fork. Wire a GitHub Actions workflow that
  publishes on tags.
- **Base Sepolia deployment** — skipped for the handoff. The old
  `deploy-sepolia.ts` / `start.sh` in the upstream x402 example are for the
  pre-Merces stack and would need porting against the new flow.
- **Single-process MPC** — `mpc_service` runs all three MPC parties in-process
  via `mpc_net::LocalNetwork`. Spawning three separate nodes is a follow-up.
- **No persistent state** — `mpc_service` regenerates MPC secret keys on every
  boot. After a restart the agent's confidential balance becomes irrecoverable
  since the on-chain balance commitment no longer decrypts. Acceptable for a
  demo; a production deployment needs key persistence.

## Related commits

- Merces1_updated `feat/x402-demo`
  - `ae2a1b3` — mpc_service HTTP sidecar
  - `4cd82c3` — initial TS demo port
  - _(this commit)_ — switch to `@taceo/x402-evm` npm dep + publish script docs
- TaceoLabs/x402 `feat/real-mpc-integration` (pushed)
  - `df3db3ae` — `@x402/evm` updated for Merces compressed proofs
  - `f6fc6bef` — agent.ts (prove sidecar) + facilitator.ts (Merces domain, no snarkjs)
  - `b06bb4f3` — deploy.ts for Merces contracts
  - _(this commit in fork)_ — `publish-taceo.sh` rename-and-publish script
