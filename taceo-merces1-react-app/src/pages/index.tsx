import { useState, useRef, useEffect } from "react";
import { Address, formatUnits, parseUnits } from "viem";
import { ConnectButton } from "@rainbow-me/rainbowkit";
import { useAccount, useWalletClient, usePublicClient, useBalance, useSwitchChain } from "wagmi";
import { Client } from "@taceolabs/taceo-merces1-client-js";

const NODE_URLS = ["/api/node0", "/api/node1", "/api/node2"];
const MERCES_CONTRACT_ADDRESS = process.env.NEXT_PUBLIC_MERCES_CONTRACT_ADDRESS! as Address;
const TOKEN_CONTRACT_ADDRESS = process.env.NEXT_PUBLIC_TOKEN_CONTRACT_ADDRESS! as Address;
const CHAIN_ID = Number(process.env.NEXT_PUBLIC_CHAIN_ID!);

interface TxResult {
  queued: string | null;
  completed: string | null;
  error: string | null;
}

function TxResultBox({ result }: { result: TxResult }) {
  return (
    <div className="mt-3 rounded border border-zinc-200 bg-zinc-50 p-3 text-sm font-mono break-all dark:border-zinc-700 dark:bg-zinc-900">
      {result.error && <p className="text-red-600">{result.error}</p>}
      {result.queued && <p className="text-zinc-700 dark:text-zinc-300">Queued: {result.queued}</p>}
      {result.completed && <p className="text-green-600">Completed: {result.completed}</p>}
    </div>
  );
}

export default function Home() {
  const { address, isConnected, chainId } = useAccount();
  const { switchChain } = useSwitchChain();
  const { data: walletClient } = useWalletClient();
  const publicClient = usePublicClient();

  const { data: tokenBalance, isLoading: tokenBlanceIsLoading, refetch: tokenBalanceRefetch } = useBalance({
    address,
    token: TOKEN_CONTRACT_ADDRESS,
  })

  const clientRef = useRef<Client | null>(null);
  const [clientReady, setClientReady] = useState(false);
  const [initError, setInitError] = useState<string | null>(null);
  const [initLoading, setInitLoading] = useState(false);

  const [privateBalance, setPrivateBalance] = useState<string | null>(null);
  const [privateBalanceLoading, setPrivateBalanceLoading] = useState(false);

  // Reset client when wallet disconnects
  useEffect(() => {
    if (!isConnected) {
      clientRef.current = null;
      setClientReady(false);
      setInitError(null);
      setPrivateBalance(null);
    } else if (walletClient && chainId === CHAIN_ID && !clientReady && !initLoading) {
      handleConnect();
    }
  }, [isConnected, walletClient, chainId]);

  // After connecting via WalletConnect (which lands on mainnet), immediately
  // switch to chain id specified in env.
  useEffect(() => {
    if (isConnected && chainId !== CHAIN_ID) {
      switchChain({ chainId: CHAIN_ID });
    }
  }, [isConnected, chainId]);


  // private_deposit state
  const [depositAmount, setDepositAmount] = useState("");
  const [depositResult, setDepositResult] = useState<TxResult | null>(null);
  const [depositLoading, setDepositLoading] = useState(false);

  // private_withdraw state
  const [withdrawAmount, setWithdrawAmount] = useState("");
  const [withdrawResult, setWithdrawResult] = useState<TxResult | null>(null);
  const [withdrawLoading, setWithdrawLoading] = useState(false);

  // private_transfer state
  const [transferReceiver, setTransferReceiver] = useState("");
  const [transferAmount, setTransferAmount] = useState("");
  const [transferResult, setTransferResult] = useState<TxResult | null>(null);
  const [transferLoading, setTransferLoading] = useState(false);

  async function handleConnect() {
    if (!isConnected || !walletClient || !publicClient) return;
    setInitLoading(true);
    setInitError(null);
    setClientReady(false);
    clientRef.current = null;
    try {
      clientRef.current = new Client({
        nodeUrls: NODE_URLS,
        contractAddress: MERCES_CONTRACT_ADDRESS,
        walletClient,
        publicClient,
        token: { type: "ERC20", address: TOKEN_CONTRACT_ADDRESS },
      });
      setClientReady(true);
      await tokenBalanceRefetch();
      await fetchPrivateBalance();
    } catch (e: unknown) {
      setInitError(String(e));
    } finally {
      setInitLoading(false);
    }
  }

  async function fetchPrivateBalance() {
    if (!clientRef.current) return;
    setPrivateBalanceLoading(true);
    try {
      const decimals = await clientRef.current.getDecimals();
      const balance = await clientRef.current.getPrivateBalance();
      setPrivateBalance(formatUnits(balance, decimals));
    } catch {
      setPrivateBalance(null);
    } finally {
      setPrivateBalanceLoading(false);
    }
  }

  async function handleDeposit() {
    if (!clientRef.current) return;
    setDepositLoading(true);
    setDepositResult(null);
    try {
      const decimals = await clientRef.current.getDecimals();
      const { queuedTxHash, completedTxHash } = await clientRef.current.deposit(
        parseUnits(depositAmount, decimals)
      );
      await tokenBalanceRefetch();
      await fetchPrivateBalance();
      setDepositResult({ queued: queuedTxHash, completed: completedTxHash, error: null });
    } catch (e: unknown) {
      setDepositResult({ queued: null, completed: null, error: String(e) });
    } finally {
      setDepositLoading(false);
    }
  }

  async function handleWithdraw() {
    if (!clientRef.current) return;
    setWithdrawLoading(true);
    setWithdrawResult(null);
    try {
      const decimals = await clientRef.current.getDecimals();
      const { queuedTxHash, completedTxHash } = await clientRef.current.withdraw(
        parseUnits(withdrawAmount, decimals)
      );
      await tokenBalanceRefetch();
      await fetchPrivateBalance();
      setWithdrawResult({ queued: queuedTxHash, completed: completedTxHash, error: null });
    } catch (e: unknown) {
      setWithdrawResult({ queued: null, completed: null, error: String(e) });
    } finally {
      setWithdrawLoading(false);
    }
  }

  async function handleTransfer() {
    if (!clientRef.current) return;
    setTransferLoading(true);
    setTransferResult(null);
    try {
      const decimals = await clientRef.current.getDecimals();
      const { queuedTxHash, completedTxHash } = await clientRef.current.transfer(
        transferReceiver as Address,
        parseUnits(transferAmount, decimals)
      );
      await fetchPrivateBalance();
      setTransferResult({ queued: queuedTxHash, completed: completedTxHash, error: null });
    } catch (e: unknown) {
      setTransferResult({ queued: null, completed: null, error: String(e) });
    } finally {
      setTransferLoading(false);
    }
  }

  const inputClass = "w-full rounded border border-zinc-300 px-3 py-2 text-sm dark:border-zinc-600 dark:bg-zinc-800 dark:text-zinc-100";
  const btnClass = "mt-2 rounded bg-zinc-900 px-4 py-2 text-sm font-medium text-white transition-colors hover:bg-zinc-700 disabled:opacity-50 dark:bg-zinc-100 dark:text-zinc-900 dark:hover:bg-zinc-300";
  const sectionClass = "rounded-lg border border-zinc-200 p-6 dark:border-zinc-700";
  const labelClass = "mb-1 block text-xs font-medium uppercase tracking-wide text-zinc-500 dark:text-zinc-400";

  return (
    <div className="min-h-screen bg-zinc-50 dark:bg-zinc-950 py-12 px-4">
      <div className="mx-auto max-w-xl space-y-6">
        <h1 className="text-2xl font-semibold text-zinc-900 dark:text-zinc-50">Merces1 Demo</h1>

        {/* Connect */}
        <div className={sectionClass}>
          <h2 className="mb-4 text-lg font-medium text-zinc-900 dark:text-zinc-50">Connect</h2>
          <ConnectButton />
        </div>

        {initError && (
          <p className="rounded border border-red-300 bg-red-50 px-3 py-2 text-sm text-red-700 dark:border-red-700 dark:bg-red-950 dark:text-red-300">
            Init error: {initError}
          </p>
        )}

        {/* Private Balance */}
        <div className={sectionClass}>
          <h2 className="mb-4 text-lg font-medium text-zinc-900 dark:text-zinc-50">Private Balance</h2>
          {privateBalanceLoading ? 'Loading...' : `${privateBalance} ${tokenBalance?.symbol}`}
        </div>

        {/* ERC20 Balance */}
        <div className={sectionClass}>
          <h2 className="mb-4 text-lg font-medium text-zinc-900 dark:text-zinc-50">Public Balance</h2>
          {tokenBlanceIsLoading ? 'Loading...' : `${tokenBalance?.formatted} ${tokenBalance?.symbol}`}
        </div>

        {/* Deposit */}
        <div className={sectionClass}>
          <h2 className="mb-4 text-lg font-medium text-zinc-900 dark:text-zinc-50">Deposit</h2>
          <label className={labelClass}>Amount ({tokenBalance?.symbol})</label>
          <input
            className={inputClass}
            type="text"
            placeholder="e.g. 1.0"
            value={depositAmount}
            onChange={(e) => setDepositAmount(e.target.value)}
          />
          <button className={btnClass} onClick={handleDeposit} disabled={depositLoading || !depositAmount || !clientReady}>
            {depositLoading ? "Processing…" : "Deposit"}
          </button>
          {depositResult && <TxResultBox result={depositResult} />}
        </div>

        {/* Withdraw */}
        <div className={sectionClass}>
          <h2 className="mb-4 text-lg font-medium text-zinc-900 dark:text-zinc-50">Withdraw</h2>
          <label className={labelClass}>Amount ({tokenBalance?.symbol})</label>
          <input
            className={inputClass}
            type="text"
            placeholder="e.g. 0.5"
            value={withdrawAmount}
            onChange={(e) => setWithdrawAmount(e.target.value)}
          />
          <button className={btnClass} onClick={handleWithdraw} disabled={withdrawLoading || !withdrawAmount || !clientReady}>
            {withdrawLoading ? "Processing…" : "Withdraw"}
          </button>
          {withdrawResult && <TxResultBox result={withdrawResult} />}
        </div>

        {/* Transfer */}
        <div className={sectionClass}>
          <h2 className="mb-4 text-lg font-medium text-zinc-900 dark:text-zinc-50">Transfer</h2>
          <label className={labelClass}>Receiver Address</label>
          <input
            className={`${inputClass} mb-3`}
            type="text"
            placeholder="0x…"
            value={transferReceiver}
            onChange={(e) => setTransferReceiver(e.target.value)}
          />
          <label className={labelClass}>Amount ({tokenBalance?.symbol})</label>
          <input
            className={inputClass}
            type="text"
            placeholder="e.g. 0.1"
            value={transferAmount}
            onChange={(e) => setTransferAmount(e.target.value)}
          />
          <button className={btnClass} onClick={handleTransfer} disabled={transferLoading || !transferReceiver || !transferAmount || !clientReady}>
            {transferLoading ? "Processing…" : "Transfer"}
          </button>
          {transferResult && <TxResultBox result={transferResult} />}
        </div>
      </div>
    </div>
  );
}
