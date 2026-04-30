import { useState, useEffect } from "react";
import { ConnectButton } from "@rainbow-me/rainbowkit";
import { useAccount, useWalletClient, useSwitchChain } from "wagmi";
import { formatUnits, type Address } from "viem";
import { x402Client, wrapFetchWithPayment, x402HTTPClient } from "@x402/fetch";
import { ConfidentialEvmScheme } from "@taceolabs/taceo-merces1-x402-js";
import { Dialog } from "radix-ui";
import Footer from "@/components/Footer";
import { PrivateBalanceCard } from "@/components/PrivateBalanceCard";

const NODE_URLS = ["/api/node0", "/api/node1", "/api/node2"];
const X402_SERVER_URL = "/api/x402-server";
const FAUCET_URL = "/api/faucet";
const CHAIN_ID = Number(process.env.NEXT_PUBLIC_CHAIN_ID!);
const BN254_PRIME = BigInt(
  "0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001",
);

async function fetchPrivateBalanceShares(address: Address): Promise<bigint> {
  const fetchShare = async (url: string, nodeIndex: number) => {
    const res = await fetch(`${url}/balance/${address}`);
    if (!res.ok) {
      throw new Error(`node ${nodeIndex} returned HTTP ${res.status}: ${res.statusText}`);
    }
    return BigInt(await res.text());
  };
  const [s0, s1, s2] = await Promise.all(
    NODE_URLS.map((url, i) => fetchShare(url, i)),
  );
  return (s0 + s1 + s2) % BN254_PRIME;
}

interface PaymentSettleResponse {
  success: boolean;
  transaction?: string;
  network?: string;
  payer?: string;
}

export default function Home() {
  const { address, isConnected, chainId } = useAccount();
  const { switchChain } = useSwitchChain();
  const { data: walletClient } = useWalletClient();

  const [privateBalance, setPrivateBalance] = useState<string | null>(null);
  const [privateBalanceLoading, setPrivateBalanceLoading] = useState(false);
  const [faucetClaiming, setFaucetClaiming] = useState(false);
  const [faucetError, setFaucetError] = useState<string | null>(null);
  const [promoCode, setPromoCode] = useState("");
  const [promoDialogOpen, setPromoDialogOpen] = useState(false);
  const [content, setContent] = useState<string | null>(null);
  const [paymentResponse, setPaymentResponse] = useState<PaymentSettleResponse | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function refreshPrivateBalance(addr: Address) {
    setPrivateBalanceLoading(true);
    try {
      const raw = await fetchPrivateBalanceShares(addr);
      setPrivateBalance(formatUnits(raw, 6));
    } catch {
      setPrivateBalance(null);
    } finally {
      setPrivateBalanceLoading(false);
    }
  }


  // After connecting via WalletConnect (which lands on mainnet), immediately
  // switch to chain id specified in env.
  useEffect(() => {
    if (isConnected && chainId !== CHAIN_ID) {
      switchChain({ chainId: CHAIN_ID });
    }
  }, [isConnected, chainId]);

  useEffect(() => {
    if (isConnected && address) {
      refreshPrivateBalance(address);
    } else {
      setPrivateBalance(null);
    }
  }, [isConnected, address]);

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
      refreshPrivateBalance(address);
    }
  }


  async function handleAccess() {
    if (!walletClient?.account) return;

    setPromoDialogOpen(false);
    setContent(null);
    setPaymentResponse(null);
    setError(null);

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
      if (promoCode.trim()) headers["x-promo-code"] = promoCode.trim();
      const response = await fetchWithPayment(`${X402_SERVER_URL}/api/protected`, { method: "GET", headers });

      if (response.ok) {
        const data = await response.text();
        setContent(data);

        const httpClient = new x402HTTPClient(client);
        const pr = httpClient.getPaymentSettleResponse((name) => response.headers.get(name));
        setPaymentResponse(pr as PaymentSettleResponse);
        if (address) refreshPrivateBalance(address);
      } else {
        const msg = await response.text();
        setError(`Request failed (${response.status}): ${msg}`);
      }
    } catch (e: unknown) {
      setError(String(e));
    }
  }

  function reset() {
    setContent(null);
    setPaymentResponse(null);
    setError(null);
  }

  return (
    <div className="min-h-screen text-zinc-900 font-sans antialiased flex flex-col">
      {/* Header */}
      <header className="flex h-14 items-center justify-between border-b border-zinc-200 px-4">
        <a href="https://merces.taceo.io" className="flex items-center gap-2.5">
          {/* eslint-disable-next-line @next/next/no-img-element */}
          <img src="/favicon.svg" width="36" height="36" alt="Merces logo" />
          <span className="flex flex-col leading-tight"><span className="text-sm font-semibold tracking-tight">Merces by TACEO</span><span className="text-xs font-normal text-zinc-800">Confidential x402</span></span>
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
      <main className="flex-1 flex flex-col items-center px-6 py-10 gap-20">

        {/* Title + description */}
        <div className="w-full max-w-2xl text-center">
          <h1 className="text-3xl font-bold tracking-tight leading-tight mb-3">Confidential x402 Demo</h1>
          <p className="text-zinc-600 leading-relaxed">
            Connect your wallet, claim some testnet Confidential USDC, and use it to pay for access to the API of our x402 resource server.
            Compared to normal x402 payments, Confidential x402 ensures that the payment amount and balance remain private on-chain, while still allowing the server to verify payments and grant access accordingly.
          </p>
        </div>

        {/* Cards column */}
        <div className="w-full max-w-[29.5rem] flex flex-col gap-5">

          {/* Private balance card */}
          {isConnected && (
            <PrivateBalanceCard
              balance={privateBalance}
              loading={privateBalanceLoading}
              onClaim={handleClaim}
              claiming={faucetClaiming}
              claimError={faucetError}
              onClaimErrorDismiss={() => setFaucetError(null)}
            />
          )}

          {!isConnected ? (
            <div className="px-6">
              <div className="rounded-lg border border-[#e4e4e7] bg-white px-4 py-3 text-sm text-zinc-600 text-center">
                Connect your wallet to access the protected resource.
              </div>
            </div>
          ) : (
            <div className="px-6 flex flex-col gap-5">

              {/* Access button box */}
              <div className="flex justify-center mb-10">
                <button
                  onClick={() => setPromoDialogOpen(true)}
                  className="w-full inline-flex min-h-[6.375rem] flex-col items-center justify-start gap-3 rounded-lg border border-[#e4e4e7] bg-transparent px-2 py-3 text-center cursor-pointer transition-[box-shadow] duration-[160ms] ease-out hover:shadow-[0_0.25rem_0.75rem_rgb(0_0_0_/_4%)] hover:border-[#e8e8eb]"
                >
                  <span aria-hidden="true" className="inline-flex items-center justify-center rounded-full w-12 h-12 bg-[#52ffc5] shrink-0">
                    <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="size-6 text-[#1f1f23]">
                      <rect width="18" height="11" x="3" y="11" rx="2" ry="2" /><path d="M7 11V7a5 5 0 0 1 10 0v4" />
                    </svg>
                  </span>
                  <span className="text-sm font-semibold text-[#09090b]">Pay to Access</span>
                </button>
              </div>

              {/* Promo code dialog */}
              <Dialog.Root open={promoDialogOpen} onOpenChange={setPromoDialogOpen}>
                <Dialog.Portal>
                  <Dialog.Overlay className="fixed inset-0 z-50 bg-black/50" />
                  <Dialog.Content className="fixed left-1/2 top-1/2 z-50 w-full max-w-sm -translate-x-1/2 -translate-y-1/2 rounded-lg border border-zinc-200 bg-white p-6 shadow-lg outline-none">
                    <Dialog.Title className="text-xl font-semibold text-zinc-900 mb-1">
                      Have a promo code?
                    </Dialog.Title>
                    <p className="text-sm text-zinc-600 leading-relaxed mb-5">
                      Use a promo code to get a discount (try <code className="font-mono bg-zinc-100 px-1 py-0.5 rounded">SAVE20</code> for 20% off).
                      With Confidential x402, no other user can see the price you paid on-chain.
                    </p>
                    <input
                      id="promo-code"
                      type="text"
                      value={promoCode}
                      onChange={(e) => setPromoCode(e.target.value)}
                      placeholder="Promo Code (optional)"
                      className="w-full h-12 rounded-lg border border-black/10 bg-white px-3 text-sm leading-tight text-zinc-800 placeholder:text-zinc-500/50 focus:outline-none"
                    />
                    <div className="mt-5 flex justify-end gap-2">
                      <Dialog.Close className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-zinc-200 bg-white h-9 px-4 text-sm font-medium shadow-xs hover:bg-zinc-100 transition-all cursor-pointer">
                        Cancel
                      </Dialog.Close>
                      <button
                        onClick={handleAccess}
                        style={{ background: "#52ffc5" }}
                        onMouseEnter={(e) => (e.currentTarget.style.background = "#33e0a8")}
                        onMouseLeave={(e) => (e.currentTarget.style.background = "#52ffc5")}
                        className="inline-flex items-center justify-center whitespace-nowrap rounded-md h-9 px-4 text-sm font-semibold text-zinc-900 transition-colors cursor-pointer border-0"
                      >
                        Continue
                      </button>
                    </div>
                  </Dialog.Content>
                </Dialog.Portal>
              </Dialog.Root>

              {/* Result dialog */}
              <Dialog.Root open={!!content} onOpenChange={(open: boolean) => !open && reset()}>
                <Dialog.Portal>
                  <Dialog.Overlay className="fixed inset-0 z-50 bg-black/50" />
                  <Dialog.Content className="fixed left-1/2 top-1/2 z-50 w-full max-w-xl -translate-x-1/2 -translate-y-1/2 rounded-lg border border-zinc-200 bg-white p-6 shadow-lg outline-none">
                    <Dialog.Title className="text-xl font-semibold text-zinc-900">
                      Payment Successful!
                    </Dialog.Title>
                    <div className="mt-3 flex flex-col gap-4">
                      <div>
                        <h2 className="mb-1.5 font-semibold text-zinc-600">Response Body</h2>
                        <div className="rounded-xl px-4 py-3 leading-relaxed border border-black/10">
                          <pre className="text-sm whitespace-pre-wrap break-all">{JSON.stringify(content, null, 2)}</pre>
                        </div>
                      </div>
                      {paymentResponse && (
                        <div>
                          <h2 className="mb-1.5 font-semibold text-zinc-600">Payment Response</h2>
                          <div className="rounded-xl px-4 py-3 border border-black/10">
                            <pre className="text-sm whitespace-pre-wrap break-all">{JSON.stringify(paymentResponse, null, 2)}</pre>
                          </div>
                        </div>
                      )}
                    </div>
                    <div className="mt-5 flex justify-end">
                      <Dialog.Close className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-zinc-200 bg-white h-9 px-4 text-sm font-medium shadow-xs hover:bg-zinc-100 transition-all">
                        Close
                      </Dialog.Close>
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
