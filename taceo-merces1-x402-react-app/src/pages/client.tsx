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
import FaucetButton from "@/components/FaucetButton";
import PayButton from "@/components/PayButton";
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
        setPaying(false);
      } else {
        setFlowStep(null);
        const msg = await response.text();
        setError(`Request failed (${response.status}): ${msg}`);
        setPaying(false);
      }
    } catch (e: unknown) {
      clearFlowTimers();
      setFlowStep(null);
      setError(String(e));
      setPaying(false);
    }
  }

  function reset() {
    setContent(null);
    setPaymentResponse(null);
    setError(null);
    setPaying(false);
  }

  return (
    <div className="flex flex-col md:flex-row min-h-screen text-zinc-900 font-sans antialiased">
      <Sidebar />
      <div className="flex-1 flex flex-col min-w-0">

        {/* Main */}
        <main className="flex-1 flex flex-col px-6 py-12">
          <div className="w-full max-w-4xl mx-auto flex flex-col gap-8">

            {/* Header */}
            <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
              <div>
                <h1 className="text-3xl font-semibold tracking-tight text-zinc-900">Client</h1>
                <p className="text-base text-zinc-500 mt-5">
                  Connect your wallet, use the faucet to get 1,000 testnet USDC to you private wallet.
                  Pay for access to the protected resource using Confidential x402.
                </p>
              </div>
              <div className="sm:pt-1 sm:shrink-0">
                <WalletButton />
              </div>
            </div>

            {/* Dashboard */}
            <div className="rounded-xl border border-zinc-200 bg-white overflow-hidden">
              <div className="flex flex-col sm:flex-row divide-y sm:divide-y-0 sm:divide-x divide-zinc-100">

                {/* Balance + Faucet */}
                <div className="flex-1 p-6 flex flex-col gap-4">
                  <div>
                    <p className="text-base font-semibold">Private Balance</p>
                    <div className="mt-2 text-4xl font-semibold text-[#192b25] leading-none">
                      {privateBalanceLoading
                        ? <span className="text-zinc-400 text-base font-normal">Loading…</span>
                        : privateBalance !== null
                          ? <>{privateBalance} <span className="text-lg font-medium text-zinc-500">USDC</span></>
                          : <span className="text-zinc-400 text-base font-normal">—</span>}
                    </div>
                  </div>
                  <div>
                    <FaucetButton onClick={handleClaim} disabled={!isConnected} loading={faucetClaiming} />
                  </div>
                  {faucetError && <ErrorDialog message={faucetError} onClose={() => setFaucetError(null)} />}
                </div>

                {/* Tier + Pay */}
                <div className="flex-1 p-6 flex flex-col gap-5">
                  <div>
                    <p className="text-base font-semibold">Pay for access</p>
                    <p className="text-sm text-zinc-500 mt-1">Choose a price tier and pay to access the protected endpoint. The amount will be hidden onchain.</p>
                    <div className="mt-3">
                      <PriceTierSelect value={priceTier} onChange={setPriceTier} />
                    </div>
                    <div className="mt-3">
                      <PayButton onClick={handleAccess} disabled={!isConnected} loading={paying} />
                    </div>
                    {error && <ErrorDialog message={error} onClose={() => setError(null)} />}
                  </div>
                </div>

              </div>
            </div>

            {/* Flow stepper */}
            <div className="flex flex-col gap-4">
              <div>
                <h2 className="text-lg font-semibold text-zinc-900">Request Flow</h2>
                <p className="text-base text-zinc-500 mt-1">Each payment flows through these steps. MPC and ZK proof ensure that no payment amount are visible onchain.</p>
              </div>
              <div className="grid grid-cols-1 lg:grid-cols-[3fr_1.5fr] gap-4 items-start">
                <RequestFlowStepper step={flowStep} />

                {/* Right: actor legend */}
                <div className="rounded-lg border border-zinc-200 bg-white p-6 flex flex-col gap-4">
                  <div>
                    <p className="text-base font-semibold mb-3">Actors</p>
                    <div className="flex flex-col gap-4">
                      <div>
                        <span className="inline-block text-[10px] font-medium px-1.5 py-0.5 rounded bg-sky-50 text-sky-600 mb-1.5">client</span>
                        <p className="text-xs text-zinc-500">Your agent or browser. Initiates requests and generates ZK proofs entirely locally — nothing leaves your device unencrypted.</p>
                      </div>
                      <div>
                        <span className="inline-block text-[10px] font-medium px-1.5 py-0.5 rounded bg-violet-50 text-violet-600 mb-1.5">server</span>
                        <p className="text-xs text-zinc-500">The protected API. Returns 402 payment requirements, then serves content once payment is settled.</p>
                      </div>
                      <div>
                        <span className="inline-block text-[10px] font-medium px-1.5 py-0.5 rounded bg-amber-50 text-amber-600 mb-1.5">facilitator</span>
                        <p className="text-xs text-zinc-500">An offchain service that verifies the ZK proof and settles the payment onchain.</p>
                      </div>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* Success */}
            {content && flowStep === 7 && (
              <PaymentResultDialog
                content={content}
                paymentResponse={paymentResponse}
                onClose={reset}
              />
            )}
          </div>
        </main>

        <Footer />
      </div>
    </div>
  );
}
