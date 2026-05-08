import { useState, useEffect, useRef } from "react";
import WalletButton from "@/components/WalletButton";
import { useAccount, useWalletClient, useSwitchChain } from "wagmi";
import { type Address } from "viem";
import { x402Client, wrapFetchWithPayment, x402HTTPClient } from "@x402/fetch";
import { ConfidentialEvmScheme } from "@taceo/confidential-x402";
import Footer from "@/components/Footer";
import Sidebar from "@/components/Sidebar";
import PriceTierSelect from "@/components/PriceTierSelect";
import PaymentResultDialog from "@/components/PaymentResultDialog";
import ErrorDialog from "@/components/ErrorDialog";
import SpinnerButton from "@/components/SpinnerButton";
import RequestFlowStepper from "@/components/RequestFlowStepper";
import { CHAIN_ID, X402_SERVER_URL, FAUCET_URL } from "@/lib/constants";
import { fetchPrivateBalanceShares } from "@/lib/api";
import { formatUSDC } from "@/lib/utils";

interface PaymentSettleResponse {
  success: boolean;
  transaction?: string;
  network?: string;
  payer?: string;
}

export default function ClientPage() {
  const { address, isConnected, chainId } = useAccount();
  const { switchChain } = useSwitchChain();
  const { data: walletClient } = useWalletClient();

  const [privateBalance, setPrivateBalance] = useState<string | null>(null);
  const [privateBalanceLoading, setPrivateBalanceLoading] = useState(false);
  const [faucetClaiming, setFaucetClaiming] = useState(false);
  const [faucetError, setFaucetError] = useState<string | null>(null);
  const [priceTier, setPriceTier] = useState("");
  const [paying, setPaying] = useState(false);
  const [content, setContent] = useState<string | null>(null);
  const [paymentResponse, setPaymentResponse] = useState<PaymentSettleResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [flowStep, setFlowStep] = useState<number | null>(null);
  const flowTimers = useRef<ReturnType<typeof setTimeout>[]>([]);

  useEffect(() => {
    if (isConnected && chainId !== CHAIN_ID) switchChain({ chainId: CHAIN_ID });
  }, [isConnected, chainId]);

  useEffect(() => {
    if (isConnected && address) {
      refreshPrivateBalance(address);
    } else {
      setPrivateBalance(null);
    }
  }, [isConnected, address]);

  async function refreshPrivateBalance(addr: Address) {
    setPrivateBalanceLoading(true);
    try {
      const raw = await fetchPrivateBalanceShares(addr);
      setPrivateBalance(formatUSDC(raw));
    } catch {
      setPrivateBalance(null);
    } finally {
      setPrivateBalanceLoading(false);
    }
  }

  async function handleClaim() {
    if (!address) return;
    setFaucetClaiming(true);
    setFaucetError(null);
    try {
      const res = await fetch(`${FAUCET_URL}/claim/${address}`, { method: "POST" });
      if (res.status === 429) {
        const msg = await res.text();
        setFaucetError(msg || "You can only claim once every 24 hours.");
      }
    } finally {
      setFaucetClaiming(false);
      refreshPrivateBalance(address);
    }
  }

  function clearFlowTimers() {
    flowTimers.current.forEach(clearTimeout);
    flowTimers.current = [];
  }

  async function handleAccess() {
    if (!walletClient?.account) return;
    setContent(null);
    setPaymentResponse(null);
    setError(null);
    clearFlowTimers();
    setFlowStep(0);
    setPaying(true);

    try {
      const signer = {
        address: walletClient.account.address,
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        signTypedData: (args: any) => walletClient.signTypedData(args),
      };
      const client = new x402Client();
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      client.register("eip155:*", new ConfidentialEvmScheme(signer as any));
      const fetchWithPayment = wrapFetchWithPayment(fetch, client);
      const headers: Record<string, string> = {};
      if (priceTier) headers["x-price-tier"] = priceTier;

      // Advance through early steps while ZK proof generates inside fetchWithPayment
      flowTimers.current.push(setTimeout(() => setFlowStep(1), 400));
      flowTimers.current.push(setTimeout(() => setFlowStep(2), 900));

      const response = await fetchWithPayment(`${X402_SERVER_URL}/api/protected`, { method: "GET", headers });
      clearFlowTimers();

      if (response.ok) {
        const sleep = (ms: number) => new Promise<void>((r) => setTimeout(r, ms));
        setFlowStep(3);
        await sleep(220);
        setFlowStep(4);
        await sleep(320);
        setFlowStep(5);
        await sleep(320);
        setFlowStep(6);

        const data = await response.text();
        setContent(data);
        const httpClient = new x402HTTPClient(client);
        const pr = httpClient.getPaymentSettleResponse((name) => response.headers.get(name));
        setPaymentResponse(pr as PaymentSettleResponse);
        if (address) refreshPrivateBalance(address);

        setFlowStep(7);
      } else {
        setFlowStep(null);
        const msg = await response.text();
        setError(`Request failed (${response.status}): ${msg}`);
      }
    } catch (e: unknown) {
      clearFlowTimers();
      setFlowStep(null);
      setError(String(e));
    }
  }

  function reset() {
    setContent(null);
    setPaymentResponse(null);
    setError(null);
    setPaying(false);
  }

  return (
    <div className="flex min-h-screen text-zinc-900 font-sans antialiased">
      <Sidebar />
      <div className="flex-1 flex flex-col min-w-0">

        {/* Main */}
        <main className="flex-1 flex flex-col px-6 py-12">
          <div className="w-full max-w-5xl mx-auto flex flex-col gap-8">

            {/* Header */}
            <div className="flex items-start justify-between gap-4">
              <div>
                <h1 className="text-3xl font-semibold tracking-tight text-zinc-900">Client</h1>
                <p className="text-sm text-zinc-500 mt-5">
                  Connect your wallet, use the faucet to get 1,000 testnet USDC to you private wallet.
                  Pay for access to the protected resource using Confidential x402.
                </p>
              </div>
              <div className="pt-1 shrink-0">
                <WalletButton />
              </div>
            </div>

            {isConnected && (
              <>
                {/* Two-column dashboard */}
                <div className="grid grid-cols-2 gap-4 items-stretch">

                  {/* Left — Balance + Faucet */}
                  <div className="rounded-lg border border-zinc-200 bg-white p-6 flex flex-col">
                    <div className="flex flex-col gap-2">
                      <p className="text-xs font-medium text-zinc-400 uppercase tracking-wider">Your balance</p>
                      <div className="text-5xl font-semibold text-[#192b25] leading-tight mt-1">
                        {privateBalanceLoading
                          ? <span className="text-zinc-400 text-base font-normal">Loading…</span>
                          : privateBalance !== null
                            ? <>{privateBalance} <span className="text-lg font-medium text-zinc-500">USDC</span></>
                            : <span className="text-zinc-400 text-base font-normal">—</span>}
                      </div>
                      <p className="text-[10px] text-zinc-400">confidential on-chain balance</p>
                    </div>

                    <div className="flex-1" />

                    <div className="flex justify-end">
                      <SpinnerButton
                        onClick={handleClaim}
                        loading={faucetClaiming}
                        loadingLabel="Claiming…"
                        className="h-9 px-4 rounded-[0.5rem] border border-zinc-200 bg-white text-sm font-medium text-zinc-800 hover:bg-zinc-50 transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        Send 1,000 USDC
                      </SpinnerButton>
                    </div>
                    {faucetError && (
                      <ErrorDialog message={faucetError} onClose={() => setFaucetError(null)} />
                    )}
                  </div>

                  {/* Right — Tier + Pay */}
                  <div className="rounded-lg border border-zinc-200 bg-white p-6 flex flex-col gap-6">
                    <div className="flex flex-col gap-3">
                      <div>
                        <p className="text-sm font-medium text-zinc-700">Price tier</p>
                        <p className="text-xs text-zinc-400 mt-0.5">Your rate stays hidden on-chain regardless of which tier you choose.</p>
                      </div>
                      <div className="self-start">
                        <PriceTierSelect value={priceTier} onChange={setPriceTier} />
                      </div>
                    </div>

                    <div className="border-t border-zinc-100 pt-5 flex flex-col gap-3">
                      <div>
                        <p className="text-sm font-medium text-zinc-700">Pay for access</p>
                        <p className="text-xs text-zinc-400 mt-0.5">Sign a confidential payment and call the protected endpoint.</p>
                      </div>
                      <SpinnerButton
                        onClick={handleAccess}
                        disabled={!walletClient?.account}
                        loading={paying}
                        loadingLabel="Paying…"
                        className="h-9 px-4 rounded-[0.5rem] bg-[#52ffc5] text-sm font-semibold text-zinc-900 hover:bg-[#33e0a8] transition-colors cursor-pointer border-0 disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        Access protected content
                      </SpinnerButton>
                    </div>

                    {error && (
                      <ErrorDialog message={error} onClose={() => setError(null)} />
                    )}
                  </div>

                </div>

                {/* Flow stepper */}
                <div>
                  <h2 className="text-lg font-semibold text-zinc-900">How it works</h2>
                  <p className="text-sm text-zinc-500 mt-1 mb-4">Each payment flows through these steps. The ZK proof keeps your exact amount hidden — only you and the counterparty know what was paid.</p>
                  <RequestFlowStepper step={flowStep} />
                </div>

                {/* Success */}
                {content && flowStep === 7 && (
                  <PaymentResultDialog
                    content={content}
                    paymentResponse={paymentResponse}
                    onClose={reset}
                  />
                )}
              </>
            )}
          </div>
        </main>

        <Footer />
      </div>
    </div>
  );
}
