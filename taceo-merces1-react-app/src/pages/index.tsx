import { useState, useEffect, useRef } from "react";
import { ConnectButton } from "@rainbow-me/rainbowkit";
import { useAccount, useWalletClient, useSwitchChain, usePublicClient, useBalance } from "wagmi";
import { formatUnits, parseUnits, type Address } from "viem";
import { Dialog } from "radix-ui";
import Footer from "@/components/Footer";
import { PublicBalanceCard } from "@/components/PublicBalanceCard";
import { PrivateBalanceCard } from "@/components/PrivateBalanceCard";
import { CardStackSlider } from "@/components/CardStackSlider";
import { Client } from "@taceolabs/taceo-merces1-client-js";

const NODE_URLS = ["/api/node0", "/api/node1", "/api/node2"];
const FAUCET_URL = "/api/faucet";
const MERCES_CONTRACT_ADDRESS = process.env.NEXT_PUBLIC_MERCES_CONTRACT_ADDRESS! as Address;
const TOKEN_CONTRACT_ADDRESS = process.env.NEXT_PUBLIC_TOKEN_CONTRACT_ADDRESS! as Address;
const CHAIN_ID = Number(process.env.NEXT_PUBLIC_CHAIN_ID!);

interface TxResult {
  queued: string | null;
  completed: string | null;
  error: string | null;
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
  const [error, setError] = useState<string | null>(null);
  const [initLoading, setInitLoading] = useState(false);
  const [faucetClaiming, setFaucetClaiming] = useState(false);
  const [faucetError, setFaucetError] = useState<string | null>(null);

  const [privateBalance, setPrivateBalance] = useState<string | null>(null);
  const [privateBalanceLoading, setPrivateBalanceLoading] = useState(false);

  const [depositDialogOpen, setDepositDialogOpen] = useState(false);
  const [withdrawDialogOpen, setWithdrawDialogOpen] = useState(false);
  const [transferDialogOpen, setTransferDialogOpen] = useState(false);

  // Reset client when wallet disconnects
  useEffect(() => {
    if (!isConnected) {
      clientRef.current = null;
      setClientReady(false);
      setError(null);
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
  const [amount, setAmount] = useState("");
  const [depositResult, setDepositResult] = useState<TxResult | null>(null);
  const [depositLoading, setDepositLoading] = useState(false);

  // private_withdraw state
  const [withdrawResult, setWithdrawResult] = useState<TxResult | null>(null);
  const [withdrawLoading, setWithdrawLoading] = useState(false);

  // private_transfer state
  const [transferReceiver, setTransferReceiver] = useState("");
  const [transferResult, setTransferResult] = useState<TxResult | null>(null);
  const [transferLoading, setTransferLoading] = useState(false);

  async function handleConnect() {
    if (!isConnected || !walletClient || !publicClient) return;
    setInitLoading(true);
    setError(null);
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
      setError(String(e));
    } finally {
      setInitLoading(false);
    }
  }

  async function handleClaim() {
    if (!address) return;
    setFaucetClaiming(true);
    try {
      const res = await fetch(`${FAUCET_URL}/claim/${address}`, { method: "POST" });
      if (res.status === 429) {
        const msg = await res.text();
        setFaucetError(msg || "You can only claim once every 24 hours.");
        return;
      }
    } finally {
      setFaucetClaiming(false);
      await fetchPrivateBalance();
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
        parseUnits(amount, decimals)
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
        parseUnits(amount, decimals)
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
        parseUnits(amount, decimals)
      );
      await fetchPrivateBalance();
      setTransferResult({ queued: queuedTxHash, completed: completedTxHash, error: null });
    } catch (e: unknown) {
      setTransferResult({ queued: null, completed: null, error: String(e) });
    } finally {
      setTransferLoading(false);
    }
  }

  return (
    <div className="min-h-screen text-zinc-900 font-sans antialiased flex flex-col">
      {/* Header */}
      <header className="flex h-14 items-center justify-between border-b border-zinc-200 px-4">
        <a href="https://merces.taceo.io" className="flex items-center gap-2.5">
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img src="/favicon.svg" width="36" height="36" alt="Merces logo" />
          <span className="flex flex-col leading-tight"><span className="text-sm font-semibold tracking-tight">Merces by TACEO</span><span className="text-xs font-normal text-zinc-800">Confidential Payments</span></span>
        </a>
        <ConnectButton.Custom>
          {({ account, chain, openConnectModal, openAccountModal, mounted }) => {
            if (!mounted) return null;
            const connected = mounted && account && chain;
            if (!connected) {
              return (
                <button
                  onClick={openConnectModal}
                  style={{ background: "#000", color: "#fff", padding: "0.5rem 0.75rem", borderRadius: "9999px", fontSize: "0.875rem", fontWeight: 600, cursor: "pointer", border: "none", lineHeight: 1.25 }}
                >
                  Connect
                </button>
              );
            }
            return (
              <button
                onClick={openAccountModal}
                style={{ height: "2.5rem", borderRadius: "9999px", border: "1px solid #e4e4e7", background: "#f9f8f5", padding: "0.125rem 1rem 0.125rem 0.125rem", display: "flex", alignItems: "center", gap: "0.5rem", cursor: "pointer", fontSize: "0.875rem", fontWeight: 600 }}
              >
                <span style={{ height: "2.25rem", width: "2.25rem", borderRadius: "9999px", border: "1px solid #e4e4e7", background: "radial-gradient(120% 95% at 24% 22%, #255b4d 0%, transparent 56%), radial-gradient(95% 95% at 70% 86%, #62ffd1 0%, transparent 62%), linear-gradient(145deg, #173f36 8%, #52ffc5 58%, #e5dbbc 100%)", display: "inline-block", flexShrink: 0 }} />
                <span>{account.address.slice(0, 4)}...{account.address.slice(-4)}</span>
              </button>
            );
          }}
        </ConnectButton.Custom>
      </header>

      {/* Main */}
      <main className="flex-1 flex flex-col items-center py-10 gap-10">

        {/* Title + description */}
        <div className="w-full max-w-2xl text-center px-6">
          <h1 className="text-3xl font-bold tracking-tight leading-tight mb-3">Confidential Payments</h1>
          <p className="text-zinc-600 leading-relaxed">
            Connect your wallet, claim some testnet Confidential USDC, and try out deposit, withdraw, and transfer.
          </p>
        </div>

        {/* Card slider */}
        {isConnected && (
          <CardStackSlider
            aria-label="Account cards"
            className="w-full"
            style={{ "--stack-side-peek": "9rem", "--stack-inactive-scale": 0.8, "--stack-edge-translate": "1.5rem" }}
          >
            <PublicBalanceCard
              balance={tokenBalance?.formatted ?? null}
              loading={tokenBlanceIsLoading}
            />
            <PrivateBalanceCard
              balance={privateBalance}
              loading={privateBalanceLoading}
              onClaim={handleClaim}
              claiming={faucetClaiming}
              claimError={faucetError}
              onClaimErrorDismiss={() => setFaucetError(null)}
            />
          </CardStackSlider>
        )}

        {/* Actions column */}
        <div className="w-full max-w-[29.5rem] flex flex-col gap-5">

          {!isConnected ? (
            <div className="px-6">
              <div className="rounded-lg border border-[#e4e4e7] bg-white px-4 py-3 text-sm text-zinc-600 text-center">
                Connect your wallet to access the protected resource.
              </div>
            </div>
          ) : (
            <div className="px-6 flex flex-col gap-5">

              {/* Access button box */}
              <div className="flex justify-center mb-10 gap-1">
                <button
                  onClick={() => setDepositDialogOpen(true)}
                  className="w-full inline-flex min-h-[6.375rem] flex-col items-center justify-start gap-3 rounded-lg border border-[#e4e4e7] bg-transparent px-2 py-3 text-center cursor-pointer transition-[box-shadow] duration-[160ms] ease-out hover:shadow-[0_0.25rem_0.75rem_rgb(0_0_0_/_4%)] hover:border-[#e8e8eb]"
                >
                  <span aria-hidden="true" className="inline-flex items-center justify-center rounded-full w-12 h-12 bg-[#52ffc5] shrink-0">
                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="size-6 text-[#1f1f23]">
                      <path d="M12 17V3"/><path d="m6 11 6 6 6-6"/><path d="M19 21H5"/>
                    </svg>
                  </span>
                  <span className="text-sm font-semibold text-[#09090b]">Deposit</span>
                </button>
                <button
                  onClick={() => setWithdrawDialogOpen(true)}
                  className="w-full inline-flex min-h-[6.375rem] flex-col items-center justify-start gap-3 rounded-lg border border-[#e4e4e7] bg-transparent px-2 py-3 text-center cursor-pointer transition-[box-shadow] duration-[160ms] ease-out hover:shadow-[0_0.25rem_0.75rem_rgb(0_0_0_/_4%)] hover:border-[#e8e8eb]"
                >
                  <span aria-hidden="true" className="inline-flex items-center justify-center rounded-full w-12 h-12 bg-[#52ffc5] shrink-0">
                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="size-6 text-[#1f1f23]">
                      <path d="m18 9-6-6-6 6"/><path d="M12 3v14"/><path d="M5 21h14"/>
                    </svg>
                  </span>
                  <span className="text-sm font-semibold text-[#09090b]">Withdraw</span>
                </button>
                <button
                  onClick={() => setTransferDialogOpen(true)}
                  className="w-full inline-flex min-h-[6.375rem] flex-col items-center justify-start gap-3 rounded-lg border border-[#e4e4e7] bg-transparent px-2 py-3 text-center cursor-pointer transition-[box-shadow] duration-[160ms] ease-out hover:shadow-[0_0.25rem_0.75rem_rgb(0_0_0_/_4%)] hover:border-[#e8e8eb]"
                >
                  <span aria-hidden="true" className="inline-flex items-center justify-center rounded-full w-12 h-12 bg-[#52ffc5] shrink-0">
                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="size-6 text-[#1f1f23]">
                      <path d="m22 2-7 20-4-9-9-4Z"/><path d="M22 2 11 13"/>
                    </svg>
                  </span>
                  <span className="text-sm font-semibold text-[#09090b]">Send</span>
                </button>
              </div>

              {/* Deposit dialog */}
              <Dialog.Root open={depositDialogOpen} onOpenChange={setDepositDialogOpen}>
                <Dialog.Portal>
                  <Dialog.Overlay className="fixed inset-0 z-50 bg-black/50" />
                  <Dialog.Content className="fixed left-1/2 top-1/2 z-50 w-full max-w-sm -translate-x-1/2 -translate-y-1/2 rounded-lg border border-zinc-200 bg-white p-6 shadow-lg outline-none">
                    <Dialog.Title className="text-xl font-semibold text-zinc-900 mb-1">
                      Deposit
                    </Dialog.Title>
                    <p className="text-sm text-zinc-600 leading-relaxed mb-5">
                      Enter the amount of USDC you want to deposit.
                    </p>
                    <input
                      id="promo-code"
                      type="text"
                      value={amount}
                      onChange={(e) => setAmount(e.target.value)}
                      placeholder="Amount in USDC, e.g. 10.0"
                      className="w-full h-12 rounded-lg border border-black/10 bg-white px-3 text-sm leading-tight text-zinc-800 placeholder:text-zinc-500/50 focus:outline-none"
                    />
                    <div className="mt-5 flex justify-end gap-2">
                      <Dialog.Close className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-zinc-200 bg-white h-9 px-4 text-sm font-medium shadow-xs hover:bg-zinc-100 transition-all cursor-pointer">
                        Cancel
                      </Dialog.Close>
                      <button
                        onClick={handleDeposit}
                        className="inline-flex items-center justify-center whitespace-nowrap rounded-md h-9 px-4 text-sm font-semibold text-zinc-900 transition-colors cursor-pointer border-0 bg-[#52ffc5] hover:bg-[#33e0a8]"
                      >
                        Confirm
                      </button>
                    </div>
                  </Dialog.Content>
                </Dialog.Portal>
              </Dialog.Root>

              {/* Withdraw dialog */}
              <Dialog.Root open={withdrawDialogOpen} onOpenChange={setWithdrawDialogOpen}>
                <Dialog.Portal>
                  <Dialog.Overlay className="fixed inset-0 z-50 bg-black/50" />
                  <Dialog.Content className="fixed left-1/2 top-1/2 z-50 w-full max-w-sm -translate-x-1/2 -translate-y-1/2 rounded-lg border border-zinc-200 bg-white p-6 shadow-lg outline-none">
                    <Dialog.Title className="text-xl font-semibold text-zinc-900 mb-1">
                      Withdraw
                    </Dialog.Title>
                    <p className="text-sm text-zinc-600 leading-relaxed mb-5">
                      Enter the amount of USDC you want to withdraw.
                    </p>
                    <input
                      id="promo-code"
                      type="text"
                      value={amount}
                      onChange={(e) => setAmount(e.target.value)}
                      placeholder="Amount in USDC, e.g. 10.0"
                      className="w-full h-12 rounded-lg border border-black/10 bg-white px-3 text-sm leading-tight text-zinc-800 placeholder:text-zinc-500/50 focus:outline-none"
                    />
                    <div className="mt-5 flex justify-end gap-2">
                      <Dialog.Close className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-zinc-200 bg-white h-9 px-4 text-sm font-medium shadow-xs hover:bg-zinc-100 transition-all cursor-pointer">
                        Cancel
                      </Dialog.Close>
                      <button
                        onClick={handleWithdraw}
                        className="inline-flex items-center justify-center whitespace-nowrap rounded-md h-9 px-4 text-sm font-semibold text-zinc-900 transition-colors cursor-pointer border-0 bg-[#52ffc5] hover:bg-[#33e0a8]"
                      >
                        Confirm
                      </button>
                    </div>
                  </Dialog.Content>
                </Dialog.Portal>
              </Dialog.Root>

              {/* Transfer dialog */}
              <Dialog.Root open={transferDialogOpen} onOpenChange={setTransferDialogOpen}>
                <Dialog.Portal>
                  <Dialog.Overlay className="fixed inset-0 z-50 bg-black/50" />
                  <Dialog.Content className="fixed left-1/2 top-1/2 z-50 w-full max-w-sm -translate-x-1/2 -translate-y-1/2 rounded-lg border border-zinc-200 bg-white p-6 shadow-lg outline-none">
                    <Dialog.Title className="text-xl font-semibold text-zinc-900 mb-1">
                      Send
                    </Dialog.Title>
                    <p className="text-sm text-zinc-600 leading-relaxed mb-5">
                      Enter the recipient's address and the amount of USDC you want to send.
                    </p>
                    <input
                      id="receiver"
                      type="text"
                      value={transferReceiver}
                      onChange={(e) => setTransferReceiver(e.target.value)}
                      placeholder="Recipient address, e.g. 0xabc123..."
                      className="w-full h-12 rounded-lg border border-black/10 bg-white px-3 text-sm leading-tight text-zinc-800 placeholder:text-zinc-500/50 focus:outline-none mb-4"
                    />
                    <input
                      id="amount"
                      type="text"
                      value={amount}
                      onChange={(e) => setAmount(e.target.value)}
                      placeholder="Amount in USDC, e.g. 10.0"
                      className="w-full h-12 rounded-lg border border-black/10 bg-white px-3 text-sm leading-tight text-zinc-800 placeholder:text-zinc-500/50 focus:outline-none"
                    />
                    <div className="mt-5 flex justify-end gap-2">
                      <Dialog.Close className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-zinc-200 bg-white h-9 px-4 text-sm font-medium shadow-xs hover:bg-zinc-100 transition-all cursor-pointer">
                        Cancel
                      </Dialog.Close>
                      <button
                        onClick={handleTransfer}
                        className="inline-flex items-center justify-center whitespace-nowrap rounded-md h-9 px-4 text-sm font-semibold text-zinc-900 transition-colors cursor-pointer border-0 bg-[#52ffc5] hover:bg-[#33e0a8]"
                      >
                        Confirm
                      </button>
                    </div>
                  </Dialog.Content>
                </Dialog.Portal>
              </Dialog.Root>

              {error && (
                <section className="flex flex-col gap-2">
                  <h2 className="text-sm font-normal leading-tight text-[#737373] text-center">Error</h2>
                  <div className="rounded-xl px-4 py-3 text-sm bg-red-50 border border-red-200 text-red-600 break-all leading-relaxed">
                    {error}
                  </div>
                </section>
              )}
            </div>
          )}
        </div>
      </main>

      <Footer />
    </div>
  );
}
