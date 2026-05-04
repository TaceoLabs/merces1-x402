import { decodeEventLog, etherUnits } from 'viem';
import type { WalletClient, PublicClient, Address } from 'viem';
import merces from '../merces.json' with { type: 'json' };
import erc20 from '../erc20.json' with { type: 'json' };
import { babyjubjub } from '@noble/curves/misc.js';
import { bn254 } from '@noble/curves/bn254.js';
import type { IField } from '@noble/curves/abstract/modular.js';
import type { AffinePoint } from '@noble/curves/abstract/curve.js';
import { randomBytes } from '@noble/hashes/utils.js';
import { sha256 } from '@noble/hashes/sha2.js';
import { bn254 as poseidon2 } from '@taceo/poseidon2';
import type { Groth16Proof } from 'snarkjs';
import * as snarkjs from 'snarkjs';

const witnessWasmUrl = new URL('../client.wasm', import.meta.url);
const zkeyUrl = new URL('../client.zkey', import.meta.url);

export async function fetchWitnessWasm(): Promise<Uint8Array | string> {
  if (typeof window === 'undefined') {
    // Node.js: just return the file path since snarkjs will read it via fs.open
    return witnessWasmUrl.pathname;
  } else {
    // Browser/webpack: fetch the wasm file and cache it in memory
    return fetch(witnessWasmUrl.href)
      .then(res => {
        if (!res.ok) {
          throw new Error(`Failed to fetch witnessWasm: ${res.status} ${res.statusText}`);
        }
        return res.arrayBuffer();
      })
      .then(buffer => {
        return new Uint8Array(buffer);
      });
  }
}

export async function fetchZkey(): Promise<Uint8Array | string> {
  if (typeof window === 'undefined') {
    // Node.js: just return the file path since snarkjs will read it via fs.open
    return zkeyUrl.pathname;
  } else {
    // Browser/webpack: fetch the zkey file and cache it in memory
    return fetch(zkeyUrl.href)
      .then(res => {
        if (!res.ok) {
          throw new Error(`Failed to fetch zkey: ${res.status} ${res.statusText}`);
        }
        return res.arrayBuffer();
      })
      .then(buffer => {
        return new Uint8Array(buffer);
      });
  }
}

export class InvalidTransactionError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "InvalidTransactionError";
  }
}

export class CannotTransferToSelfError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "CannotTransferToSelfError";
  }
}

export class InsufficientBalanceError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "InsufficientBalanceError";
  }
}

export class ProofError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ProofError";
  }
}

export class InvalidAmountError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "InvalidAmountError";
  }
}

export class TimeoutError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "TimeoutError";
  }
}

export type Token = { type: 'Native' } | { type: 'ERC20', address: Address };

export interface TransferProof {
  compressedProof: [string, string, string, string];
  beta: string;
  amountCommitment: bigint;
  ciphertext: {
    amount: [bigint, bigint, bigint];
    r: [bigint, bigint, bigint];
    senderPk: AffinePoint<bigint>;
  };
}

export interface ClientArgs {
  nodeUrls: string[];
  contractAddress: Address;
  walletClient: WalletClient;
  publicClient: PublicClient;
  token: Token;
  /** Timeout in ms to wait for a transaction receipt from the chain (default: 30_000) */
  txReceiptTimeout?: number;
  /** Timeout in ms to wait for the ProcessedMPC on-chain event (default: 30_000) */
  mpcEventTimeout?: number;
}

export class Client {
  nodeUrls: string[] = [];
  contractAddress: Address;
  token: Token;
  walletClient: WalletClient;
  publicClient: PublicClient;
  mpcPks: AffinePoint<bigint>[] | null = null;
  zkey: Uint8Array | string | null = null;
  witnessWasm: Uint8Array | string | null = null;
  decimals: number | null = null;
  symbol: string | null = null;
  name: string | null = null;
  txReceiptTimeout: number = 30_000;
  mpcEventTimeout: number = 30_000;

  constructor({ nodeUrls, contractAddress, walletClient, publicClient, token, txReceiptTimeout, mpcEventTimeout }: ClientArgs) {
    this.nodeUrls = nodeUrls;
    this.contractAddress = contractAddress;
    this.token = token;
    this.walletClient = walletClient;
    this.publicClient = publicClient;
    this.txReceiptTimeout = txReceiptTimeout ?? this.txReceiptTimeout;
    this.mpcEventTimeout = mpcEventTimeout ?? this.mpcEventTimeout;
  }

  private async getWitnessWasm(): Promise<Uint8Array | string> {
    if (this.witnessWasm) {
      return this.witnessWasm;
    }
    this.witnessWasm = await fetchWitnessWasm();
    return this.witnessWasm;
  }

  private async getZkey(): Promise<Uint8Array | string> {
    if (this.zkey) {
      return this.zkey;
    }
    this.zkey = await fetchZkey();
    return this.zkey;
  }


  private async getMpcPks(): Promise<AffinePoint<bigint>[]> {
    if (this.mpcPks) {
      return this.mpcPks;
    }
    const data = await this.publicClient.readContract({
      address: this.contractAddress,
      abi: merces,
      functionName: 'getMpcPublicKeys',
    });
    this.mpcPks = data as [AffinePoint<bigint>, AffinePoint<bigint>, AffinePoint<bigint>];
    return this.mpcPks;
  }

  public address(): Address {
    return this.walletClient.account!.address;
  }

  public async getDecimals(): Promise<number> {
    if (this.decimals !== null) {
      return this.decimals;
    }
    if (this.token.type === 'Native') {
      this.decimals = etherUnits.wei;
    } else if (this.token.type === 'ERC20') {
      const decimals = await this.publicClient.readContract({
        address: this.token.address,
        abi: erc20,
        functionName: 'decimals',
      });
      this.decimals = Number(decimals);
    }
    return this.decimals!;
  }

  public async getSymbol(): Promise<string> {
    if (this.symbol !== null) {
      return this.symbol;
    }
    if (this.token.type === 'Native') {
      this.symbol = 'ETH';
    } else if (this.token.type === 'ERC20') {
      const symbol = await this.publicClient.readContract({
        address: this.token.address,
        abi: erc20,
        functionName: 'symbol',
      });
      this.symbol = symbol as string;
    }
    return this.symbol!;
  }

  public async getName(): Promise<string> {
    if (this.name !== null) {
      return this.name;
    }
    if (this.token.type === 'Native') {
      this.name = 'Ether';
    } else if (this.token.type === 'ERC20') {
      const name = await this.publicClient.readContract({
        address: this.token.address,
        abi: erc20,
        functionName: 'name',
      });
      this.name = name as string;
    }
    return this.name!;
  }

  public async getPrivateBalance(): Promise<bigint> {
    const address = this.walletClient.account!.address;
    const fetchBalance = async (url: string, nodeIndex: number) => {
      const res = await fetch(`${url}/balance/${address}`);
      if (!res.ok) {
        throw new Error(`node ${nodeIndex} returned HTTP ${res.status}: ${res.statusText}`);
      }
      const balance = await res.text();
      return BigInt(balance);
    };
    const [balanceShare0, balanceShare1, balanceShare2] = await Promise.all([
      fetchBalance(this.nodeUrls[0]!, 0),
      fetchBalance(this.nodeUrls[1]!, 1),
      fetchBalance(this.nodeUrls[2]!, 2),
    ]);
    const BN254_PRIME = BigInt(
      "0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001",
    );

    const balance = balanceShare0 + balanceShare1 + balanceShare2;
    return balance % BN254_PRIME;
  }

  public async getNativeBalance(): Promise<bigint> {
    const address = this.walletClient.account!.address;
    return await this.publicClient.getBalance({ address });
  }

  public async getErc20Balance(): Promise<bigint> {
    if (this.token.type === 'ERC20') {
      const address = this.walletClient.account!.address;
      const balance = await this.publicClient.readContract({
        address: this.token.address,
        abi: erc20,
        functionName: 'balanceOf',
        args: [address],
      });
      return balance as bigint;
    } else {
      throw new Error('Token type is Native, not ERC20');
    }
  }

  public async deposit(
    amount: bigint,
  ): Promise<{ queuedTxHash: string, completedTxHash: string }> {
    if (amount <= 0n) {
      throw new InvalidAmountError('Amount must be greater than zero');
    }
    const balance = this.token.type === 'Native' ? await this.getNativeBalance() : await this.getErc20Balance();
    if (balance < amount) {
      throw new InsufficientBalanceError(`Insufficient balance: ${balance} < ${amount}`);
    }
    if (this.token.type === 'Native') {
      const { request } = await this.publicClient.simulateContract({
        address: this.contractAddress,
        abi: merces,
        functionName: 'deposit',
        args: [amount],
        value: amount,
        account: this.walletClient.account!,
        chain: this.walletClient.chain,
      });
      const hash = await this.walletClient.writeContract(request);
      const receipt = await this.publicClient.waitForTransactionReceipt({ hash, timeout: this.txReceiptTimeout });
      const blockNumber = receipt.blockNumber;
      const log = receipt.logs[0]!;
      const { args } = decodeEventLog({ abi: merces, data: log.data, topics: log.topics });
      const actionIndex = (args as unknown as { actionIndex: bigint }).actionIndex;
      return { queuedTxHash: hash, completedTxHash: await this.waitForProcessedMPC(actionIndex, blockNumber) };
    } else {
      const approve = await this.publicClient.simulateContract({
        address: this.token.address,
        abi: erc20,
        functionName: 'approve',
        args: [this.contractAddress, amount],
        account: this.walletClient.account!,
        chain: this.walletClient.chain,
      });
      const approveHash = await this.walletClient.writeContract(approve.request);
      await this.publicClient.waitForTransactionReceipt({ hash: approveHash, timeout: this.txReceiptTimeout });
      const deposit = await this.publicClient.simulateContract({
        address: this.contractAddress,
        abi: merces,
        functionName: 'deposit',
        args: [amount],
        account: this.walletClient.account!,
        chain: this.walletClient.chain,
      });
      const hash = await this.walletClient.writeContract(deposit.request);
      const receipt = await this.publicClient.waitForTransactionReceipt({ hash, timeout: this.txReceiptTimeout });
      const blockNumber = receipt.blockNumber;
      const log = receipt.logs[0]!;
      const { args } = decodeEventLog({ abi: merces, data: log.data, topics: log.topics });
      const actionIndex = (args as unknown as { actionIndex: bigint }).actionIndex;
      return { queuedTxHash: hash, completedTxHash: await this.waitForProcessedMPC(actionIndex, blockNumber) };
    }
  }

  public async withdraw(
    amount: bigint,
  ): Promise<{ queuedTxHash: string, completedTxHash: string }> {
    if (amount <= 0n) {
      throw new InvalidAmountError('Amount must be greater than zero');
    }
    const balance = await this.getPrivateBalance();
    if (balance < amount) {
      throw new InsufficientBalanceError(`Insufficient balance: ${balance} < ${amount}`);
    }
    const withdraw = await this.publicClient.simulateContract({
      address: this.contractAddress,
      abi: merces,
      functionName: 'withdraw',
      args: [amount],
      account: this.walletClient.account!,
      chain: this.walletClient.chain,
    });
    const hash = await this.walletClient.writeContract(withdraw.request);
    const receipt = await this.publicClient.waitForTransactionReceipt({ hash, timeout: this.txReceiptTimeout });
    const blockNumber = receipt.blockNumber;
    const log = receipt.logs[0]!;
    const { args } = decodeEventLog({ abi: merces, data: log.data, topics: log.topics });
    const actionIndex = (args as unknown as { actionIndex: bigint }).actionIndex;
    return { queuedTxHash: hash, completedTxHash: await this.waitForProcessedMPC(actionIndex, blockNumber) };
  }

  public async transfer(
    receiver: Address,
    amount: bigint,
  ): Promise<{ queuedTxHash: string, completedTxHash: string }> {
    const sender = this.walletClient.account!.address;
    const mpcPks = await this.getMpcPks();
    if (amount <= 0n) {
      throw new InvalidAmountError('Amount must be greater than zero');
    }
    if (sender.toLowerCase() === receiver.toLowerCase()) {
      throw new CannotTransferToSelfError('Sender and receiver addresses are the same');
    }
    const balance = await this.getPrivateBalance();
    if (balance < amount) {
      throw new InsufficientBalanceError(`Insufficient balance: ${balance} < ${amount}`);
    }
    const witnessWasm = await this.getWitnessWasm();
    const zkey = await this.getZkey();
    const { inputs, ciphertexts, senderPk, amountCommitment } = prepareTransfer(amount, mpcPks);
    let proof: snarkjs.Groth16Proof, publicSignals: snarkjs.PublicSignals;
    try {
      ({ proof, publicSignals } = await snarkjs.groth16.fullProve(inputs, witnessWasm, zkey));
    } catch (e) {
      throw new ProofError(`Proof or witness generation failed: ${e instanceof Error ? e.message : String(e)}`);
    }
    const beta = publicSignals[0]!;
    const transfer = await this.publicClient.simulateContract({
      address: this.contractAddress,
      abi: merces,
      functionName: 'transfer',
      args: [receiver, amountCommitment, BigInt(beta), encodeCiphertexts(ciphertexts, senderPk), compressProof(proof)],
      account: this.walletClient.account!,
      chain: this.walletClient.chain,
    });
    const hash = await this.walletClient.writeContract(transfer.request);
    const receipt = await this.publicClient.waitForTransactionReceipt({ hash, timeout: this.txReceiptTimeout });
    const blockNumber = receipt.blockNumber;
    const log = receipt.logs[0]!;
    const { args } = decodeEventLog({ abi: merces, data: log.data, topics: log.topics });
    const actionIndex = (args as unknown as { actionIndex: bigint }).actionIndex;
    return { queuedTxHash: hash, completedTxHash: await this.waitForProcessedMPC(actionIndex, blockNumber) };
  }

  private async waitForProcessedMPC(
    actionIndex: bigint,
    fromBlock: bigint
  ): Promise<string> {
    const { promise, resolve, reject } = Promise.withResolvers<string>();
    let settled = false;
    const settle = (txHash: string) => { if (!settled) { settled = true; clearTimeout(timer); resolve(txHash); } };
    const fail = (err: unknown) => { if (!settled) { settled = true; clearTimeout(timer); reject(err); } };

    const handleLog = (log: any) => {
      const { args } = decodeEventLog({ abi: merces, data: log.data, topics: log.topics });
      const { actionIndices, valid } = args as unknown as { actionIndices: bigint[], valid: boolean[] };
      const idx = actionIndices.findIndex(i => i === actionIndex);
      if (idx === -1) return false;
      if (valid[idx] === true) {
        settle(log.transactionHash!);
      } else {
        fail(new InvalidTransactionError('Transaction is invalid'));
      }
      return true;
    };

    // watch for new logs — actionIndices is not indexed so we cannot filter on-chain
    const unwatch = this.publicClient.watchContractEvent({
      address: this.contractAddress,
      abi: merces,
      eventName: 'ProcessedMPC',
      onLogs: (logs) => {
        for (const log of logs) {
          if (handleLog(log)) {
            unwatch();
            return;
          }
        }
      },
      onError: (err) => {
        unwatch();
        fail(err);
      }
    });

    const timer = setTimeout(() => {
      unwatch();
      fail(new TimeoutError('Timed out waiting for ProcessedMPC event'));
    }, this.mpcEventTimeout);

    // also check past logs
    this.publicClient.getContractEvents({
      address: this.contractAddress,
      abi: merces,
      eventName: 'ProcessedMPC',
      fromBlock,
    }).then(logs => {
      for (const log of logs) {
        if (handleLog(log)) {
          unwatch();
          return;
        }
      }
    }).catch(err => {
      unwatch();
      fail(err);
    });

    return promise;
  }
}

export function encodeCiphertexts(ciphertexts: { ciphertexts0: [bigint, bigint], ciphertexts1: [bigint, bigint], ciphertexts2: [bigint, bigint] }, senderPk: AffinePoint<bigint>): { amount: [bigint, bigint, bigint], r: [bigint, bigint, bigint], senderPk: AffinePoint<bigint> } {
  return {
    amount: [ciphertexts.ciphertexts0[0], ciphertexts.ciphertexts1[0], ciphertexts.ciphertexts2[0]],
    r: [ciphertexts.ciphertexts0[1], ciphertexts.ciphertexts1[1], ciphertexts.ciphertexts2[1]],
    senderPk,
  };
}

/** BN254 scalar field */
const Bn254Fr = babyjubjub.Point.Fp;
/** BabyJubJub scalar field (prime subgroup order, matches ark_babyjubjub::Fr) */
const BabyJubJubFr = babyjubjub.Point.Fn;

// DS constant for commit1 (Poseidon2-t2 based Pedersen commitment)
const DS_COMMIT1 = BigInt(0xDEADBEEF);

// DS constant for sym_encrypt2, from the SAFE-API paper (absorb 2, squeeze 2, domainsep = 0x4142)
// Matches Rust: ark_bn254::Fr::from_bigint(BigInt([0x00020000_00024142, 0x8000, 0, 0]))
// where BigInt limbs are little-endian 64-bit words: value = limb0 + limb1 * 2^64
const DS_ENCRYPT2 = BigInt('0x0002000000024142') + (BigInt('0x8000') << 64n);

function randomFieldElement(field: IField<bigint>): bigint {
  const bytes = randomBytes(48);
  let n = 0n;
  for (let i = 0; i < 48; i++) n = n * 256n + BigInt(bytes[i]!);
  return field.create(n);
}

// Poseidon2-t2 commitment: state = [value + DS, r], permute, return state[0] + value
function commit1(value: bigint, r: bigint): bigint {
  const state = [Bn254Fr.create(value + DS_COMMIT1), r];
  poseidon2.t2.permutationInPlace(state);
  return Bn254Fr.create(state[0]! + value);
}

// Poseidon2-t3 symmetric encryption of 2 field elements
// state = [key, nonce, DS], permute, ciphertext[i] = msg[i] + state[i]
function symEncrypt2(key: bigint, msg: [bigint, bigint], nonce: bigint): [bigint, bigint] {
  const state = [key, nonce, DS_ENCRYPT2];
  poseidon2.t3.permutationInPlace(state);
  return [Bn254Fr.create(msg[0] + state[0]!), Bn254Fr.create(msg[1] + state[1]!)];
}

// ECDH: x-coordinate of mpcPk * encryptSk on BabyJubJub
function dhKeyDerivation(encryptSk: bigint, mpcPk: AffinePoint<bigint>): bigint {
  return babyjubjub.Point.fromAffine(mpcPk).multiply(encryptSk).toAffine().x;
}

// SHA256 of all inputs serialized as 32-byte big-endian, top 3 bits masked out
function computeAlpha(hashInputs: bigint[]): bigint {
  const bytes = new Uint8Array(hashInputs.length * 32);
  for (let i = 0; i < hashInputs.length; i++) {
    let v = hashInputs[i]!;
    for (let j = 31; j >= 0; j--) {
      bytes[i * 32 + j] = Number(v & 0xffn);
      v >>= 8n;
    }
  }
  const hash = sha256(bytes);
  let alpha = 0n;
  for (const b of hash) alpha = (alpha << 8n) | BigInt(b);
  return alpha & ((1n << 253n) - 1n);
}

const Fp = bn254.fields.Fp;
const Fp2 = bn254.fields.Fp2;

function compressG1(x: bigint, y: bigint): bigint {
  if (x === 0n && y === 0n) return 0n;
  const y_sqrt = Fp.sqrt(Fp.add(Fp.pow(x, 3n), 3n));
  return y === y_sqrt ? x << 1n : (x << 1n) | 1n;
}

function hasFpSqrt(v: bigint): boolean {
  try {
    Fp.sqrt(v);
    return true;
  } catch {
    return false;
  }
}

function compressG2(
  x: { c0: bigint; c1: bigint },
  y: { c0: bigint; c1: bigint },
): [bigint, bigint] {
  if (x.c0 === 0n && x.c1 === 0n && y.c0 === 0n && y.c1 === 0n) return [0n, 0n];

  const n3ab = Fp.mul(Fp.mul(x.c0, x.c1), Fp.neg(3n));
  const a3 = Fp.pow(x.c0, 3n);
  const b3 = Fp.pow(x.c1, 3n);
  const inv82 = Fp.inv(82n);
  const frac_27_82 = Fp.mul(27n, inv82);
  const frac_3_82 = Fp.mul(3n, inv82);
  const y0_pos = Fp.add(Fp.add(Fp.mul(n3ab, x.c1), a3), frac_27_82);
  const y1_pos = Fp.neg(Fp.add(Fp.add(Fp.mul(n3ab, x.c0), b3), frac_3_82));
  const half = Fp.inv(2n);
  const d = Fp.sqrt(Fp.add(Fp.sqr(y0_pos), Fp.sqr(y1_pos)));
  const hint = !hasFpSqrt(Fp.mul(Fp.add(y0_pos, d), half));

  const y2 = { c0: y0_pos, c1: y1_pos };
  const y_computed = Fp2.sqrt(y2);
  const b0_base = x.c0 << 2n;
  const b1 = x.c1;

  if (Fp2.eql(y_computed, y)) {
    return [b0_base | (hint ? 2n : 0n), b1];
  } else if (Fp2.eql(Fp2.neg(y_computed), y)) {
    return [b0_base | (hint ? 3n : 1n), b1];
  } else {
    throw new Error('compressG2: y is neither sqrt nor -sqrt, point not on curve');
  }
}

/**
 * Compress a Groth16 proof (snarkjs format) into 4 bigint values for the Solidity verifier.
 * Matches the output of `taceo_groth16_sol::prepare_compressed_proof`.
 * Output order: [a, b1, b0, c]
 */
export function compressProof(proof: Groth16Proof): bigint[] {
  const ax = BigInt(proof.pi_a[0]!);
  const ay = BigInt(proof.pi_a[1]!);
  const bx = { c0: BigInt(proof.pi_b[0]![0]!), c1: BigInt(proof.pi_b[0]![1]!) };
  const by = { c0: BigInt(proof.pi_b[1]![0]!), c1: BigInt(proof.pi_b[1]![1]!) };
  const cx = BigInt(proof.pi_c[0]!);
  const cy = BigInt(proof.pi_c[1]!);

  const a = compressG1(ax, ay);
  const [b0, b1] = compressG2(bx, by);
  const c = compressG1(cx, cy);

  return [a, b1, b0, c];
}

export interface PreparedTransferOutput {
  inputs: Record<string, string | string[]>;
  ciphertexts: {
    ciphertexts0: [bigint, bigint];
    ciphertexts1: [bigint, bigint];
    ciphertexts2: [bigint, bigint];
  };
  amountCommitment: bigint;
  amountR: bigint;
  senderPk: AffinePoint<bigint>;
}

export function prepareTransfer(
  amount: bigint,
  mpcPks: AffinePoint<bigint>[],
): PreparedTransferOutput {
  if (mpcPks.length !== 3) throw new Error('mpcPks must have length 3');

  const amountR = randomFieldElement(Bn254Fr);
  const encryptSk = randomFieldElement(BabyJubJubFr);
  const shareAmount = [randomFieldElement(Bn254Fr), randomFieldElement(Bn254Fr)] as const;
  const shareAmountR = [randomFieldElement(Bn254Fr), randomFieldElement(Bn254Fr)] as const;

  const encryptPk = babyjubjub.Point.BASE.multiply(encryptSk).toAffine();
  const amountCommitment = commit1(amount, amountR);

  const amountShares: [bigint, bigint, bigint] = [
    shareAmount[0],
    shareAmount[1],
    Bn254Fr.create(amount - shareAmount[0] - shareAmount[1]),
  ];
  const amountRShares: [bigint, bigint, bigint] = [
    shareAmountR[0],
    shareAmountR[1],
    Bn254Fr.create(amountR - shareAmountR[0] - shareAmountR[1]),
  ];

  const ciphertextsArr = mpcPks.map((pk, i) => {
    const sk = dhKeyDerivation(encryptSk, pk);
    return symEncrypt2(sk, [amountShares[i]!, amountRShares[i]!], 0n);
  }) as [[bigint, bigint], [bigint, bigint], [bigint, bigint]];

  const hashInputs = [
    encryptPk.x, encryptPk.y, amountCommitment,
    ...ciphertextsArr.flatMap(c => [c[0], c[1]]),
    ...mpcPks.flatMap(pk => [pk.x, pk.y]),
  ];
  const alpha = computeAlpha(hashInputs);

  return {
    inputs: {
      amount: amount.toString(),
      amount_r: amountR.toString(),
      encrypt_sk: encryptSk.toString(),
      mpc_pks: mpcPks.flatMap(pk => [pk.x.toString(), pk.y.toString()]),
      share_amount: [shareAmount[0].toString(), shareAmount[1].toString()],
      share_amount_r: [shareAmountR[0].toString(), shareAmountR[1].toString()],
      alpha: alpha.toString(),
    },
    ciphertexts: {
      ciphertexts0: ciphertextsArr[0],
      ciphertexts1: ciphertextsArr[1],
      ciphertexts2: ciphertextsArr[2],
    },
    amountCommitment,
    amountR,
    senderPk: encryptPk,
  };
}
