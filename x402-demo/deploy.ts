/**
 * deploy.ts — Deploy all Merces contracts to local anvil for the confidential x402 demo.
 *
 * Deploys:
 *   1. VerifierClient (compressed Groth16 verifier for client transfer proofs)
 *   2. VerifierServer (compressed Groth16 verifier for MPC batch proofs)
 *   3. Poseidon2 hasher (library)
 *   4. BabyJubJub (library)
 *   5. ActionQueueLib (library)
 *   6. USDCToken (test ERC-20 — needed because real USDC can't be minted on local chain)
 *   7. Merces (confidential token contract, linked with libraries)
 *
 * Then deposits tokens for the agent.
 *
 * Usage: tsx deploy.ts
 * Requires: anvil running on http://127.0.0.1:8545
 */

import { readFileSync, writeFileSync } from "fs";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";
import {
  createPublicClient,
  createWalletClient,
  http,
  defineChain,
} from "viem";
import { privateKeyToAccount } from "viem/accounts";

const __dirname = dirname(fileURLToPath(import.meta.url));

// ── Configuration ──────────────────────────────────────────────────────────────

const RPC_URL = process.env.RPC_URL || "http://127.0.0.1:8545";
const CHAIN_ID = parseInt(process.env.CHAIN_ID || "31337");

// Anvil default accounts
const DEPLOYER_KEY = (process.env.DEPLOYER_PRIVATE_KEY ||
  "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80") as `0x${string}`;
const MPC_KEY = (process.env.MPC_PRIVATE_KEY ||
  "0x59c6995e998f97a5a0044966f0945389dc9e86dae88c7a8412f4603b6b78690d") as `0x${string}`;
const FACILITATOR_KEY = (process.env.FACILITATOR_PRIVATE_KEY ||
  "0x5de4111afa1a4b94908f83103eb1f1706367c2e68ca870fc3fb9a804cdab365a") as `0x${string}`;
const AGENT_KEY = (process.env.AGENT_PRIVATE_KEY ||
  "0x7c852118294e51e653712a81e05800f419141751be58f605c371e15141b007a6") as `0x${string}`;
const SERVER_KEY = (process.env.SERVER_PRIVATE_KEY ||
  "0x47e179ec197488593b187f80a00eb0da91f1b9d0b13f8733639f19c30a34926a") as `0x${string}`;

// Path to Merces contract artifacts (colocated in this repo)
const MERCES_ARTIFACTS = resolve(
  __dirname,
  process.env.MERCES_ARTIFACTS_DIR || "../contracts/json",
);

// URL of the mpc-service HTTP sidecar. deploy.ts fetches the MPC's 3 BabyJubJub
// pubkeys from `/pubkeys` before deploying Merces (so the contract is constructed
// with the real keys), then POSTs `/start` with the Merces address so the service
// begins polling the on-chain queue.
const MPC_SERVICE_URL = process.env.MPC_SERVICE_URL || "http://127.0.0.1:4025";

type MpcPubkeys = {
  x1: bigint; y1: bigint;
  x2: bigint; y2: bigint;
  x3: bigint; y3: bigint;
};

async function fetchMpcPubkeys(): Promise<MpcPubkeys> {
  const res = await fetch(`${MPC_SERVICE_URL}/pubkeys`);
  if (!res.ok) {
    throw new Error(`mpc-service /pubkeys returned ${res.status}`);
  }
  const j = await res.json() as { x1: string; y1: string; x2: string; y2: string; x3: string; y3: string };
  return {
    x1: BigInt(j.x1), y1: BigInt(j.y1),
    x2: BigInt(j.x2), y2: BigInt(j.y2),
    x3: BigInt(j.x3), y3: BigInt(j.y3),
  };
}

async function notifyMpcStart(mercesAddress: string): Promise<void> {
  const res = await fetch(`${MPC_SERVICE_URL}/start`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ merces: mercesAddress }),
  });
  if (!res.ok) {
    const text = await res.text();
    throw new Error(`mpc-service /start returned ${res.status}: ${text}`);
  }
}

// ── Helpers ────────────────────────────────────────────────────────────────────

const anvil = defineChain({
  id: CHAIN_ID,
  name: "Anvil",
  nativeCurrency: { name: "Ether", symbol: "ETH", decimals: 18 },
  rpcUrls: { default: { http: [RPC_URL] } },
});

function loadArtifact(name: string) {
  const path = resolve(MERCES_ARTIFACTS, `${name}.json`);
  const raw = JSON.parse(readFileSync(path, "utf8"));
  return {
    abi: raw.abi,
    bytecode: raw.bytecode.object as `0x${string}`,
    linkReferences: (raw.bytecode.linkReferences || {}) as Record<string, Record<string, Array<{ start: number; length: number }>>>,
  };
}

function linkBytecode(
  bytecode: string,
  linkReferences: Record<string, Record<string, Array<{ start: number; length: number }>>>,
  libraries: Record<string, `0x${string}`>,
): `0x${string}` {
  let linked = bytecode;
  for (const [file, libs] of Object.entries(linkReferences)) {
    for (const [libName, offsets] of Object.entries(libs)) {
      const addr = libraries[libName];
      if (!addr) throw new Error(`Missing library address for ${file}:${libName}`);
      const addrHex = addr.slice(2).toLowerCase();
      for (const { start, length } of offsets) {
        const charStart = 2 + start * 2;
        const charLen = length * 2;
        linked = linked.slice(0, charStart) + addrHex + linked.slice(charStart + charLen);
      }
    }
  }
  return linked as `0x${string}`;
}

// ── Main ───────────────────────────────────────────────────────────────────────

async function main() {
  console.log("[Deploy] Starting Merces deployment to anvil...\n");

  console.log(`[Deploy] Fetching MPC pubkeys from ${MPC_SERVICE_URL}/pubkeys...`);
  const mpcPubkeys = await fetchMpcPubkeys();
  console.log("[Deploy] Got MPC pubkeys");
  const pk1 = { x: mpcPubkeys.x1, y: mpcPubkeys.y1 };
  const pk2 = { x: mpcPubkeys.x2, y: mpcPubkeys.y2 };
  const pk3 = { x: mpcPubkeys.x3, y: mpcPubkeys.y3 };

  const deployerAccount = privateKeyToAccount(DEPLOYER_KEY);
  const mpcAccount = privateKeyToAccount(MPC_KEY);
  const facilitatorAccount = privateKeyToAccount(FACILITATOR_KEY);
  const agentAccount = privateKeyToAccount(AGENT_KEY);
  const serverAccount = privateKeyToAccount(SERVER_KEY);

  const publicClient = createPublicClient({ chain: anvil, transport: http(RPC_URL) });
  const walletClient = createWalletClient({
    account: deployerAccount,
    chain: anvil,
    transport: http(RPC_URL),
  });

  // 1. Deploy VerifierClient
  console.log("[Deploy] Deploying VerifierClient...");
  const vc = loadArtifact("VerifierClient");
  const vcHash = await walletClient.deployContract({ abi: vc.abi, bytecode: vc.bytecode });
  const vcReceipt = await publicClient.waitForTransactionReceipt({ hash: vcHash });
  const vcAddress = vcReceipt.contractAddress!;
  console.log(`[Deploy] VerifierClient deployed at ${vcAddress}\n`);

  // 2. Deploy VerifierServer
  console.log("[Deploy] Deploying VerifierServer...");
  const vs = loadArtifact("VerifierServer");
  const vsHash = await walletClient.deployContract({ abi: vs.abi, bytecode: vs.bytecode });
  const vsReceipt = await publicClient.waitForTransactionReceipt({ hash: vsHash });
  const vsAddress = vsReceipt.contractAddress!;
  console.log(`[Deploy] VerifierServer deployed at ${vsAddress}\n`);

  // 3. Deploy Poseidon2 (library)
  console.log("[Deploy] Deploying Poseidon2...");
  const p2 = loadArtifact("Poseidon2");
  const p2Hash = await walletClient.deployContract({ abi: p2.abi, bytecode: p2.bytecode });
  const p2Receipt = await publicClient.waitForTransactionReceipt({ hash: p2Hash });
  const p2Address = p2Receipt.contractAddress!;
  console.log(`[Deploy] Poseidon2 deployed at ${p2Address}\n`);

  // 4. Deploy BabyJubJub (library)
  console.log("[Deploy] Deploying BabyJubJub...");
  const bjj = loadArtifact("BabyJubJub");
  const bjjHash = await walletClient.deployContract({ abi: bjj.abi, bytecode: bjj.bytecode });
  const bjjReceipt = await publicClient.waitForTransactionReceipt({ hash: bjjHash });
  const bjjAddress = bjjReceipt.contractAddress!;
  console.log(`[Deploy] BabyJubJub deployed at ${bjjAddress}\n`);

  // 5. Deploy ActionQueueLib (library)
  console.log("[Deploy] Deploying ActionQueueLib...");
  const aq = loadArtifact("ActionQueue");
  const aqHash = await walletClient.deployContract({ abi: aq.abi, bytecode: aq.bytecode });
  const aqReceipt = await publicClient.waitForTransactionReceipt({ hash: aqHash });
  const aqAddress = aqReceipt.contractAddress!;
  console.log(`[Deploy] ActionQueueLib deployed at ${aqAddress}\n`);

  // 6. Deploy USDCToken (test ERC-20)
  console.log("[Deploy] Deploying USDCToken...");
  const usdc = loadArtifact("USDCToken");
  const usdcHash = await walletClient.deployContract({
    abi: usdc.abi,
    bytecode: usdc.bytecode,
    args: [deployerAccount.address],
  });
  const usdcReceipt = await publicClient.waitForTransactionReceipt({ hash: usdcHash });
  const usdcAddress = usdcReceipt.contractAddress!;
  console.log(`[Deploy] USDCToken deployed at ${usdcAddress}\n`);

  // 7. Deploy Merces (linked with 3 libraries)
  console.log("[Deploy] Deploying Merces...");
  const merces = loadArtifact("Merces");
  const linkedBytecode = linkBytecode(
    merces.bytecode,
    merces.linkReferences,
    {
      BabyJubJub: bjjAddress as `0x${string}`,
      ActionQueueLib: aqAddress as `0x${string}`,
      Poseidon2T2_BN254: p2Address as `0x${string}`,
    },
  );
  const mercesHash = await walletClient.deployContract({
    abi: merces.abi,
    bytecode: linkedBytecode,
    args: [
      vcAddress,              // _clientVerifier
      vsAddress,              // _serverVerifier
      mpcAccount.address,     // _mpcAddress
      usdcAddress,            // _tokenAddress (address(0) for native ETH)
      pk1,                    // _mpcPk1
      pk2,                    // _mpcPk2
      pk3,                    // _mpcPk3
      "demo",                 // _environmentTag
    ],
  });
  const mercesReceipt = await publicClient.waitForTransactionReceipt({ hash: mercesHash });
  const mercesAddress = mercesReceipt.contractAddress!;
  console.log(`[Deploy] Merces deployed at ${mercesAddress}\n`);

  // ── Seed: mint USDC and deposit for agent ──────────────────────────────────

  console.log("[Seed] Minting USDC for agent...");
  const mintAmount = BigInt(100_000_000); // 100 USDC (6 decimals)
  const mintTx = await walletClient.writeContract({
    address: usdcAddress,
    abi: usdc.abi,
    functionName: "mint",
    args: [agentAccount.address, mintAmount],
  });
  await publicClient.waitForTransactionReceipt({ hash: mintTx });
  console.log(`[Seed] Minted ${Number(mintAmount) / 1e6} USDC to agent ${agentAccount.address}`);

  // Agent approves + deposits
  const agentWallet = createWalletClient({
    account: privateKeyToAccount(AGENT_KEY),
    chain: anvil,
    transport: http(RPC_URL),
  });

  const approveTx = await agentWallet.writeContract({
    address: usdcAddress,
    abi: usdc.abi,
    functionName: "approve",
    args: [mercesAddress, mintAmount],
  });
  await publicClient.waitForTransactionReceipt({ hash: approveTx });

  const depositTx = await agentWallet.writeContract({
    address: mercesAddress,
    abi: merces.abi,
    functionName: "deposit",
    args: [mintAmount],
  });
  await publicClient.waitForTransactionReceipt({ hash: depositTx });
  console.log(`[Seed] Agent deposited ${Number(mintAmount) / 1e6} USDC into Merces\n`);

  // ── Notify MPC service to start polling ──────────────────────────────────────

  console.log(`[Deploy] Notifying mpc-service at ${MPC_SERVICE_URL}/start...`);
  await notifyMpcStart(mercesAddress);
  console.log("[Deploy] mpc-service started — it will process the deposit on next poll\n");

  // ── Write .env ───────────────────────────────────────────────────────────────

  const envContent = `# Generated by deploy.ts — DO NOT EDIT
# Anvil default accounts
DEPLOYER_PRIVATE_KEY=${DEPLOYER_KEY}
MPC_PRIVATE_KEY=${MPC_KEY}
FACILITATOR_PRIVATE_KEY=${FACILITATOR_KEY}
AGENT_PRIVATE_KEY=${AGENT_KEY}
SERVER_PRIVATE_KEY=${SERVER_KEY}

# Chain
CHAIN_ID=${CHAIN_ID}
RPC_URL=${RPC_URL}

# Contract addresses
USDC_ADDRESS=${usdcAddress}
MERCES_ADDRESS=${mercesAddress}
VERIFIER_CLIENT_ADDRESS=${vcAddress}
VERIFIER_SERVER_ADDRESS=${vsAddress}

# Derived addresses
DEPLOYER_ADDRESS=${deployerAccount.address}
MPC_ADDRESS=${mpcAccount.address}
FACILITATOR_ADDRESS=${facilitatorAccount.address}
SERVER_ADDRESS=${serverAccount.address}
AGENT_ADDRESS=${agentAccount.address}

# MPC Public Keys (fetched from mpc-service at deploy time)
MPC_PK1_X=${mpcPubkeys.x1.toString()}
MPC_PK1_Y=${mpcPubkeys.y1.toString()}
MPC_PK2_X=${mpcPubkeys.x2.toString()}
MPC_PK2_Y=${mpcPubkeys.y2.toString()}
MPC_PK3_X=${mpcPubkeys.x3.toString()}
MPC_PK3_Y=${mpcPubkeys.y3.toString()}

# MPC service URL (Rust HTTP sidecar that polls on-chain queue and calls processMPC)
MPC_SERVICE_URL=${MPC_SERVICE_URL}

# Merces artifacts path
MERCES_ARTIFACTS_DIR=${process.env.MERCES_ARTIFACTS_DIR || "../contracts/json"}
`;

  const envPath = resolve(__dirname, ".env");
  writeFileSync(envPath, envContent);
  console.log(`[Deploy] Env written to ${envPath}\n`);

  // ── Summary ──────────────────────────────────────────────────────────────────

  console.log("=".repeat(60));
  console.log("  Merces Deployment Complete!");
  console.log("=".repeat(60));
  console.log(`  USDCToken:       ${usdcAddress}`);
  console.log(`  VerifierClient:  ${vcAddress}`);
  console.log(`  VerifierServer:  ${vsAddress}`);
  console.log(`  Merces:          ${mercesAddress}`);
  console.log("");
  console.log(`  Deployer:     ${deployerAccount.address}`);
  console.log(`  MPC Operator: ${mpcAccount.address}`);
  console.log(`  Facilitator:  ${facilitatorAccount.address}`);
  console.log(`  Agent:        ${agentAccount.address} (100 USDC deposited)`);
  console.log(`  Server:       ${serverAccount.address}`);
  console.log("=".repeat(60));
}

main().catch((err) => {
  console.error("[Deploy] Fatal error:", err);
  process.exit(1);
});
