#!/usr/bin/env bash
set -euo pipefail

# ── Merces x402 Demo — Local Anvil ───────────────────────────────────────────
#
# One-command launcher for the full confidential-payments demo on local anvil.
#
# Services spawned:
#   :8545  anvil               — local EVM
#   :4024  prove sidecar       — Rust client-proof generator (Merces1_updated client bin)
#   :4025  mpc_service         — Rust 3-party MPC poller (Merces1_updated e2e bin)
#   :4022  facilitator         — TS x402 facilitator
#   :4021  resource server     — TS /v1/sentiment endpoint
#
# Usage:
#   ./start-local.sh          Deploy + start all services + run 3 ZK payments
#   ./start-local.sh --stop   Kill everything on the demo ports
#
# Prerequisites:
#   - Foundry (anvil)               https://book.getfoundry.sh
#   - Rust bins built:              cargo build --release --bin prove --bin mpc_service
#                                   (from Merces1_updated repo root)
#   - pnpm install already run      (see README.md — needs x402 fork side-by-side)

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
PROVE_BIN="$REPO_ROOT/target/release/prove"
MPC_SERVICE_BIN="$REPO_ROOT/target/release/mpc_service"
PIDS_FILE="$SCRIPT_DIR/.demo-local-pids"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
CYAN='\033[0;36m'
NC='\033[0m'

log()  { echo -e "${GREEN}[demo]${NC} $*"; }
warn() { echo -e "${YELLOW}[demo]${NC} $*"; }
err()  { echo -e "${RED}[demo]${NC} $*"; }

# ── Stop ─────────────────────────────────────────────────────────────────────

stop_services() {
  log "Stopping services..."
  local stopped=0

  if [[ -f "$PIDS_FILE" ]]; then
    while read -r pid; do
      if kill -0 "$pid" 2>/dev/null; then
        kill "$pid" 2>/dev/null && stopped=$((stopped + 1))
      fi
    done < "$PIDS_FILE"
    rm -f "$PIDS_FILE"
  fi

  # Fallback: kill anything still bound to our ports
  for port in 4021 4022 4024 4025 8545; do
    local pid
    pid=$(lsof -ti:"$port" 2>/dev/null || true)
    if [[ -n "$pid" ]]; then
      kill "$pid" 2>/dev/null && stopped=$((stopped + 1))
    fi
  done

  if [[ $stopped -gt 0 ]]; then
    log "Stopped $stopped process(es)"
  else
    log "No running services found"
  fi
}

if [[ "${1:-}" == "--stop" ]]; then
  stop_services
  exit 0
fi

# ── Preflight ────────────────────────────────────────────────────────────────

if ! command -v anvil &>/dev/null; then
  err "anvil not found. Install Foundry: https://book.getfoundry.sh"
  exit 1
fi

if [[ ! -x "$PROVE_BIN" ]]; then
  err "prove binary not found at $PROVE_BIN"
  err "Build it first: cd $REPO_ROOT && cargo build --release --bin prove"
  exit 1
fi

if [[ ! -x "$MPC_SERVICE_BIN" ]]; then
  err "mpc_service binary not found at $MPC_SERVICE_BIN"
  err "Build it first: cd $REPO_ROOT && cargo build --release --bin mpc_service"
  exit 1
fi

if [[ ! -d "node_modules" ]]; then
  err "node_modules missing. Run \`pnpm install\` first (see README.md)."
  exit 1
fi

# Kill anything still on our ports before starting fresh
stop_services 2>/dev/null || true

> "$PIDS_FILE"

wait_for_port() {
  local url="$1"
  local name="$2"
  local attempts="${3:-30}"
  for _ in $(seq 1 "$attempts"); do
    if curl -s "$url" > /dev/null 2>&1; then
      return 0
    fi
    sleep 1
  done
  err "$name did not become ready in time"
  return 1
}

# ── Anvil ────────────────────────────────────────────────────────────────────

log "Starting anvil on :8545..."
anvil --silent 2>&1 &
echo $! >> "$PIDS_FILE"

for _ in $(seq 1 10); do
  if curl -s http://127.0.0.1:8545 -X POST -H "Content-Type: application/json" \
    -d '{"jsonrpc":"2.0","method":"eth_chainId","params":[],"id":1}' > /dev/null 2>&1; then
    break
  fi
  sleep 1
done
log "Anvil ${GREEN}ready${NC} (chain 31337)"

# ── Prove sidecar ────────────────────────────────────────────────────────────

log "Starting prove sidecar on :4024 (loads proving key ~10s)..."
"$PROVE_BIN" > "$SCRIPT_DIR/.prove.log" 2>&1 &
echo $! >> "$PIDS_FILE"
wait_for_port "http://127.0.0.1:4024/health" "prove sidecar" 30
log "Prove sidecar ${GREEN}ready${NC}"

# ── MPC service ──────────────────────────────────────────────────────────────

log "Starting mpc_service on :4025 (loads proving keys ~20s)..."
"$MPC_SERVICE_BIN" > "$SCRIPT_DIR/.mpc_service.log" 2>&1 &
echo $! >> "$PIDS_FILE"
wait_for_port "http://127.0.0.1:4025/health" "mpc_service" 45
log "MPC service ${GREEN}ready${NC}"

# ── Deploy ───────────────────────────────────────────────────────────────────

log "Deploying Merces contracts (fetches pubkeys from mpc_service, seeds 100 USDC)..."
pnpm run deploy 2>&1 | grep -E "^\[Deploy\]|\[Seed\]" | while read -r line; do
  echo -e "  ${CYAN}${line}${NC}"
done
log "Contracts deployed — mpc_service now polling"

# ── Facilitator ──────────────────────────────────────────────────────────────

log "Starting facilitator on :4022..."
npx tsx facilitator.ts > "$SCRIPT_DIR/.facilitator.log" 2>&1 &
echo $! >> "$PIDS_FILE"
wait_for_port "http://127.0.0.1:4022/supported" "facilitator" 15
log "Facilitator ${GREEN}ready${NC}"

# ── Server ───────────────────────────────────────────────────────────────────

log "Starting resource server on :4021..."
npx tsx server.ts > "$SCRIPT_DIR/.server.log" 2>&1 &
echo $! >> "$PIDS_FILE"
# Server returns 402 when healthy (paywall endpoint) — use that as readiness signal
for _ in $(seq 1 15); do
  if [[ "$(curl -s -o /dev/null -w "%{http_code}" http://127.0.0.1:4021/v1/sentiment 2>/dev/null)" == "402" ]]; then
    break
  fi
  sleep 1
done
log "Server ${GREEN}ready${NC}"

# ── Summary ──────────────────────────────────────────────────────────────────

echo ""
for pair in "8545:anvil" "4024:prove" "4025:mpc_service" "4022:facilitator" "4021:server"; do
  port="${pair%%:*}"
  name="${pair#*:}"
  printf "  ${GREEN}✓${NC} %-15s :${port}\n" "$name"
done
echo ""

log "All services running — running agent (3 paid requests with ZK proofs)..."
echo ""
pnpm run agent
echo ""

log "Demo done. Stop everything with: ${CYAN}./start-local.sh --stop${NC}"
