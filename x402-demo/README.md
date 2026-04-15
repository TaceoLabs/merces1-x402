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

## Repositories

This demo spans **two** repos — they must be checked out side-by-side.

1. **`TaceoLabs/Merces1_updated`** — Merces contract + 3-party MPC (Rust) +
   this demo's glue (TypeScript + scripts). Branch **`feat/x402-demo`**.
2. **`TaceoLabs/x402`** — Taceo's fork of Coinbase's x402 TS stack, adding the
   confidential-payments scheme as a sibling of `exact/` inside
   `@x402/evm`. Branch **`feat/real-mpc-integration`**.

The demo's `pnpm-workspace.yaml` links to the x402 fork via relative paths, so
the filesystem layout matters:

```
<some-parent>/
├── Merces1_updated/        ← this repo
│   └── x402-demo/          ← you are here
└── x402/
    └── repo/               ← the x402 fork
        └── typescript/packages/*
```

If your layout differs, adjust the relative paths in `pnpm-workspace.yaml`.

## Prerequisites on the host

- **Foundry** (for `anvil`) — <https://book.getfoundry.sh>
- **Rust toolchain** — stable
- **Node 20+** and **pnpm 10+**

## One-time setup

```bash
# 1. Get the two repos side-by-side
mkdir -p ~/taceo && cd ~/taceo
git clone -b feat/x402-demo            git@github.com:TaceoLabs/Merces1_updated.git
mkdir -p x402 && cd x402
git clone -b feat/real-mpc-integration git@github.com:TaceoLabs/x402.git repo
cd ..

# 2. Build the Rust bins (prove + mpc_service)
#    First-time build pulls all crates and compiles ~600 deps — expect 5–10 min.
#    Subsequent builds are seconds. Output goes to ../target/release/.
cd Merces1_updated
cargo build --release --bin prove --bin mpc_service

# 3. Install TS deps for the demo (resolves @x402/* from the sibling x402 fork)
cd x402-demo
pnpm install
```

## Running the demo

From `Merces1_updated/x402-demo/`:

```bash
./start-local.sh
```

Output you should see:

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

Note: `start-local.sh` exits after the agent finishes its 3 requests, but
the background services (anvil, prove, mpc_service, facilitator, server)
keep running so you can poke at the contract state, replay the agent, etc.
When you're done:

```bash
./start-local.sh --stop
```

While the demo is running, per-service logs are tailable from this directory:

```
.prove.log            prove sidecar (proof generation timings)
.mpc_service.log      mpc_service (queue polling + processMPC calls + tx hashes)
.facilitator.log      facilitator (verify + settle results per request)
.server.log           resource server (incoming requests)
```

## Manual run (skipping start-local.sh)

If you want each service in its own terminal so you can tail its output
directly, run them in this order (each waits for the previous to be ready):

```bash
# Terminal 1
anvil --silent

# Terminal 2 — loads proving key ~10 s, then :4024
../target/release/prove

# Terminal 3 — loads proving keys ~10 s, then :4025
../target/release/mpc_service

# Terminal 4 — deploys 7 Merces contracts, mints + deposits 100 USDC,
#             fetches MPC pubkeys from :4025/pubkeys, POSTs :4025/start
pnpm run deploy

# Terminal 5 — :4022
pnpm run facilitator

# Terminal 6 — :4021
pnpm run server

# Terminal 7 — 3 paid requests, each triggers a prove→facilitator→MPC cycle
pnpm run agent
```

## What's where

| File                  | Role                                                                       |
| --------------------- | -------------------------------------------------------------------------- |
| `deploy.ts`           | Fetches MPC pubkeys from `:4025/pubkeys`, deploys Merces + verifiers + USDC, mints + deposits 100 USDC for the agent, POSTs `:4025/start` so the MPC begins polling. |
| `facilitator.ts`      | x402 facilitator. Registers the confidential scheme for chain 31337, verifies payloads, submits `transferFrom()` on-chain. |
| `server.ts`           | Resource server exposing `/v1/sentiment`. Returns 402 with confidential paymentRequirements. |
| `agent.ts`            | Client. Makes 3 paid requests. Calls the `prove` sidecar for each proof, signs the EIP-712 `TransferFromAuthorization`. |
| `start-local.sh`      | One-command launcher. Checks prereqs, boots services in dependency order, runs the agent. |
| `pnpm-workspace.yaml` | Bridges this package to the `@x402/*` packages in the sibling x402 fork via relative paths. |

## Where the TypeScript dependencies come from

```
@x402/core       → x402 fork · typescript/packages/core
@x402/evm        → x402 fork · typescript/packages/mechanisms/evm
                   (this is the fork's version — adds confidential/ scheme)
@x402/axios      → x402 fork · typescript/packages/http/axios
@x402/express    → x402 fork · typescript/packages/http/express
```

All four are consumed via pnpm's `workspace:*` protocol; `pnpm install` in
this directory reads `pnpm-workspace.yaml`, discovers the packages at those
relative paths, and symlinks them into `node_modules/`.

## Known issues / future work

- **Upstream the confidential scheme to Coinbase.** The fork only *adds* a new
  scheme (`confidential/`) alongside `exact/` — no upstream files are modified.
  The eventual PR is additive. Once merged, this demo can drop the fork and
  consume `@x402/evm` from npm directly.
- **Publish `@taceo/x402-evm` to npm.** Once published, consumers wouldn't
  need the x402 fork checkout at all. Deferred — colleague's call.
- **Base Sepolia deployment.** Local-anvil only for now. The pre-Merces
  Sepolia path (`deploy-sepolia.ts` etc., still in the x402 fork's
  `examples/typescript/agent-confidential/` for history) would need porting
  against this new flow.
- **Single-process MPC.** `mpc_service` runs all three MPC parties in-process
  via `mpc_net::LocalNetwork`. Splitting into three separate processes is a
  follow-up.
- **No persistent state.** `mpc_service` regenerates MPC secret keys on every
  boot. After a restart the agent's confidential balance becomes
  irrecoverable, since the on-chain balance commitment no longer decrypts.
  Acceptable for a demo; a production deployment needs key persistence.

## Branch tips for the handoff

- `TaceoLabs/Merces1_updated` — branch **`feat/x402-demo`** (this directory)
- `TaceoLabs/x402` — branch **`feat/real-mpc-integration`** (the fork; see its
  `FORK-README.md` for an overview of what the branch adds on top of upstream)
