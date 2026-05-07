import { type Address } from "viem";
import { NODE_URLS, BN254_PRIME, X402_SERVER_ADDRESS } from "@/lib/constants";

export interface Transfer {
  id: number;
  sender: string;
  receiver: string;
  txHash: string | null;
  amountCommitment: string;
  amount: bigint;
  timestamp: Date;
}

type RawTransfer = {
  Transfer: {
    id: number;
    sender: string;
    receiver: string;
    tx_hash: string | null;
    amount_commitment: string;
    amount_share: string;
    timestamp: string;
  };
};

export async function fetchPrivateBalanceShares(address: Address): Promise<bigint> {
  const fetchShare = async (url: string, nodeIndex: number) => {
    const res = await fetch(`${url}/balance/${address}`);
    if (!res.ok) throw new Error(`node ${nodeIndex} returned HTTP ${res.status}: ${res.statusText}`);
    return BigInt(await res.text());
  };
  const [s0, s1, s2] = await Promise.all(NODE_URLS.map((url, i) => fetchShare(url, i)));
  return (s0 + s1 + s2) % BN254_PRIME;
}

export async function fetchTransactions(): Promise<Transfer[]> {
  const params = new URLSearchParams({ limit: "100", type: "Transfer" });
  if (X402_SERVER_ADDRESS) params.set("receiver", X402_SERVER_ADDRESS);
  const fetchFromNode = async (url: string, i: number): Promise<RawTransfer[]> => {
    const res = await fetch(`${url}/transactions?${params}`);
    if (!res.ok) throw new Error(`node ${i} returned HTTP ${res.status}: ${res.statusText}`);
    return res.json();
  };
  const [txs0, txs1, txs2] = await Promise.all(NODE_URLS.map((url, i) => fetchFromNode(url, i)));
  return txs0.map((tx0, i) => {
    const t0 = tx0.Transfer;
    const t1 = txs1[i]!.Transfer;
    const t2 = txs2[i]!.Transfer;
    const amount = (BigInt(t0.amount_share) + BigInt(t1.amount_share) + BigInt(t2.amount_share)) % BN254_PRIME;
    return {
      id: t0.id,
      sender: t0.sender,
      receiver: t0.receiver,
      txHash: t0.tx_hash,
      amountCommitment: t0.amount_commitment,
      amount,
      timestamp: new Date(t0.timestamp),
    };
  });
}
