import { useState, useEffect, useMemo } from "react";
import WalletButton from "@/components/WalletButton";
import { useAccount, useWalletClient, useSwitchChain } from "wagmi";
import { type Address } from "viem";
import { x402Client, wrapFetchWithPayment, x402HTTPClient } from "@x402/fetch";
import { ConfidentialEvmScheme } from "@taceo/confidential-x402";
import Footer from "@/components/Footer";
import Sidebar from "@/components/Sidebar";
import TierBarChart, { inferPriceTier, type PriceTier } from "@/components/TierBarChart";
import PriceTierSelect from "@/components/PriceTierSelect";
import TxTable, { type Transfer } from "@/components/TxTable";
import X402ModeToggle, { X402Mode } from "@/components/X402ModeToggle";
import PaymentResultDialog from "@/components/PaymentResultDialog";
import ErrorDialog from "@/components/ErrorDialog";
import SpinnerButton from "@/components/SpinnerButton";
import { CHAIN_ID, X402_SERVER_URL, FAUCET_URL, X402_SERVER_ADDRESS, BLOCK_EXPLORER_URL } from "@/lib/constants";
import { fetchPrivateBalanceShares, fetchTransactions } from "@/lib/api";
import { formatUSDC, truncateAddress } from "@/lib/utils";

interface PaymentSettleResponse {
  success: boolean;
  transaction?: string;
  network?: string;
  payer?: string;
}


const tocItems = [
  { label: "Confidential x402", href: "#intro" },
  { label: "The client", href: "#client" },
  { label: "The resource server", href: "#resource-server" },
  { label: "On-chain transactions", href: "#transaction-log" },
  { label: "Why privacy matters", href: "#why-privacy" },
];

export default function ArticlePage() {
  const { address, isConnected, chainId } = useAccount();
  const { switchChain } = useSwitchChain();
  const { data: walletClient } = useWalletClient();

  const [privateBalance, setPrivateBalance] = useState<string | null>(null);
  const [privateBalanceLoading, setPrivateBalanceLoading] = useState(false);
  const [serverBalance, setServerBalance] = useState<string | null>(null);
  const [serverBalanceLoading, setServerBalanceLoading] = useState(false);
  const [faucetClaiming, setFaucetClaiming] = useState(false);
  const [faucetError, setFaucetError] = useState<string | null>(null);
  const [priceTier, setPriceTier] = useState("");
  const [content, setContent] = useState<string | null>(null);
  const [paymentResponse, setPaymentResponse] = useState<PaymentSettleResponse | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [x402Mode, setX402Mode] = useState<X402Mode>("confidential");
  const [txs, setTxs] = useState<Transfer[]>([]);
  const [txsLoading, setTxsLoading] = useState(true);
  const [paying, setPaying] = useState(false);

  const tierStats = useMemo(() => {
    if (txs.length === 0) return null;
    const totalRevenue = txs.reduce((acc, tx) => acc + tx.amount, BigInt(0));
    const avgPayment = totalRevenue / BigInt(txs.length);
    const tierCounts: Record<PriceTier, number> = { Standard: 0, STARTUP: 0, GROWTH: 0, ENTERPRISE: 0 };
    const tierRevenue: Record<PriceTier, bigint> = { Standard: BigInt(0), STARTUP: BigInt(0), GROWTH: BigInt(0), ENTERPRISE: BigInt(0) };
    for (const tx of txs) {
      const tier = inferPriceTier(tx.amount);
      tierCounts[tier]++;
      tierRevenue[tier] += tx.amount;
    }
    return { totalRevenue, avgPayment, tierCounts, tierRevenue };
  }, [txs]);

  async function refreshServerBalance() {
    if (!X402_SERVER_ADDRESS) return;
    setServerBalanceLoading(true);
    try {
      const raw = await fetchPrivateBalanceShares(X402_SERVER_ADDRESS as Address);
      setServerBalance(formatUSDC(raw));
    } catch {
      setServerBalance(null);
    } finally {
      setServerBalanceLoading(false);
    }
  }

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

  async function refreshTransactions() {
    setTxsLoading(true);
    try {
      setTxs(await fetchTransactions());
    } catch {
      // leave existing txs on error
    } finally {
      setTxsLoading(false);
    }
  }

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

  useEffect(() => {
    refreshTransactions();
    refreshServerBalance();
  }, []);

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

  async function handleAccess() {
    if (!walletClient?.account) return;
    setContent(null);
    setPaymentResponse(null);
    setError(null);
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
      const response = await fetchWithPayment(`${X402_SERVER_URL}/api/protected`, { method: "GET", headers });
      if (response.ok) {
        const data = await response.text();
        setContent(data);
        const httpClient = new x402HTTPClient(client);
        const pr = httpClient.getPaymentSettleResponse((name) => response.headers.get(name));
        setPaymentResponse(pr as PaymentSettleResponse);
        if (address) refreshPrivateBalance(address);
        refreshTransactions();
        refreshServerBalance();
      } else {
        const msg = await response.text();
        setError(`Request failed (${response.status}): ${msg}`);
      }
    } catch (e: unknown) {
      setError(String(e));
    } finally {
      setPaying(false);
    }
  }

  function reset() {
    setContent(null);
    setPaymentResponse(null);
    setError(null);
  }

  return (
    <div className="flex flex-col md:flex-row min-h-screen text-zinc-900 font-sans antialiased" style={{ scrollBehavior: "smooth" }}>
      <Sidebar />
      <div className="flex-1 flex flex-col min-w-0">
      <main className="flex-1 flex flex-col px-6 pb-12 lg:px-10">
      {/* Body */}
      <div className="flex flex-col lg:flex-row lg:items-start gap-10 max-w-screen-xl mx-auto w-full">

        {/* Spacer to balance the TOC and keep the article centered */}
        <div className="hidden lg:block lg:flex-none lg:w-52" />

        {/* Article — center */}
        <article className="w-full min-w-0 max-w-3xl mx-auto flex flex-col gap-y-4 pt-12">

          {/* Intro */}
          <div id="intro" style={{ scrollMarginTop: "5rem" }}>
            <h1 className="text-3xl font-medium tracking-tight text-zinc-900 mb-6">Confidential x402</h1>
            <p className="text-base text-zinc-500 leading-relaxed">
              <a href="https://x402.org/" className="underline underline-offset-4 hover:text-zinc-900 transition-colors">x402</a> is an open HTTP payment protocol for machine-to-machine payments. A resource server responds with <code>HTTP 402 Payment Required</code> when a request lacks a valid payment. The client attaches a cryptographically signed payment to its next request; the server verifies it and settles it on-chain before responding — no API keys, no subscriptions, no billing infrastructure.
            </p>
            <p className="text-base text-zinc-500 leading-relaxed mt-3">
              The default x402 flow is fully public: every payment is visible on-chain as a plain ERC-20 token transfer. This works for flat-rate APIs, but breaks down once pricing becomes dynamic — per-customer rates, volume discounts, and AI agent spending patterns are all exposed. <em>Merces</em> by <a href="https://taceo.io/" className="underline underline-offset-4 hover:text-zinc-900 transition-colors">TACEO</a> extends x402 with a confidential transfer scheme: the payment settles on-chain, but the amount stays hidden.
            </p>
          </div>

          {/* Client */}
          <div id="client" style={{ scrollMarginTop: "5rem" }}>
            <h2 className="text-xl font-medium text-zinc-900 mt-6 mb-3">The client</h2>

            {/* Connection + balance line */}
            {!isConnected ? (
              <>
                <p className="text-base text-zinc-500 leading-relaxed">
                  To follow along, connect a wallet first. Your balance will be held as a secret-shared encrypted value distributed across three MPC nodes — no single node knows the plaintext amount.
                </p>
                <div className="mt-4 flex justify-center">
                  <WalletButton />
                </div>
              </>
            ) : (
              <>
                <p className="text-base text-zinc-500 leading-relaxed">
                  Your balance is held as a secret-shared encrypted value distributed across three MPC nodes — no single node knows the plaintext amount.
                </p>
                <div className="mt-3 flex items-center justify-center gap-3">
                  <WalletButton />
                  <span className="text-lg font-semibold text-[#192b25]">
                    {privateBalanceLoading
                      ? <span className="text-zinc-400 font-normal">loading…</span>
                      : privateBalance !== null
                        ? <>{privateBalance} <span className="font-medium text-zinc-400">USDC</span></>
                        : <span className="text-zinc-400 font-normal">—</span>}
                  </span>
                </div>

                <div className="flex flex-col gap-6 mt-6">
                  {/* Step 1 */}
                  <div>
                    <p className="text-base text-zinc-500 leading-relaxed">
                      <span className="font-medium text-zinc-700">Fund from the faucet.</span>{" "}
                      Receive 1,000 testnet USDC credited to your private balance. The faucet can be used once every 24 hours.
                    </p>
                    <div className="mt-3 flex justify-center">
                      <SpinnerButton
                        onClick={handleClaim}
                        loading={faucetClaiming}
                        loadingLabel="Claiming…"
                        className="h-9 px-4 rounded-[0.5rem] border border-zinc-200 bg-white text-sm font-medium text-zinc-800 hover:bg-zinc-50 transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
                      >
                        Claim 1,000 USDC from faucet
                      </SpinnerButton>
                      {faucetError && (
                        <ErrorDialog message={faucetError} onClose={() => setFaucetError(null)} />
                      )}
                    </div>
                  </div>

                  {/* Step 2 */}
                  <div>
                    <p className="text-base text-zinc-500 leading-relaxed">
                      <span className="font-medium text-zinc-700">Select a price tier.</span>{" "}
                      The resource server applies per-customer pricing. Choose a tier — whatever rate you pay stays hidden on-chain.
                    </p>
                    <div className="mt-3 flex justify-center">
                      <PriceTierSelect value={priceTier} onChange={setPriceTier} />
                    </div>
                  </div>

                  {/* Step 3 */}
                  <div>
                    <p className="text-base text-zinc-500 leading-relaxed">
                      <span className="font-medium text-zinc-700">Pay for access.</span>{" "}
                      Sign a confidential payment and call the protected endpoint. Your wallet signs a typed-data message; the server verifies and settles it on-chain before responding with the protected content.
                    </p>
                    <div className="mt-3 flex justify-center">
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
                  </div>

                  {error && (
                    <ErrorDialog message={error} onClose={() => setError(null)} />
                  )}

                  {content && (
                    <PaymentResultDialog
                      content={content}
                      paymentResponse={paymentResponse}
                      onClose={reset}
                    />
                  )}
                </div>
              </>
            )}
          </div>

          {/* Resource server */}
          <div id="resource-server" style={{ scrollMarginTop: "5rem" }}>
            <h2 className="text-xl font-medium text-zinc-900 mt-6 mb-3">The resource server</h2>
            <p className="text-base text-zinc-500 leading-relaxed">
              The resource server issues a <code>402 Payment Required</code> challenge when no payment is attached, then forwards the client's signed payload to the TACEO facilitator for verification before serving the protected content. Its accumulated private balance grows with each successful payment — individual amounts are never exposed on-chain. The server tracks them directly, and they can be reconstructed from the MPC network if needed.
            </p>

            {/* Server balance */}
            <div className="mt-4 flex items-center justify-center gap-3">
              {X402_SERVER_ADDRESS && (
                <div className="inline-flex items-center gap-2 h-9 px-3 pr-4 rounded-full border border-zinc-200 bg-[#f4f4f5] text-sm font-semibold text-zinc-800">
                  <span style={{ height: "1.5rem", width: "1.5rem", borderRadius: "9999px", background: "radial-gradient(120% 95% at 24% 22%, #255b4d 0%, transparent 56%), radial-gradient(95% 95% at 70% 86%, #62ffd1 0%, transparent 62%), linear-gradient(145deg, #173f36 8%, #52ffc5 58%, #e5dbbc 100%)", display: "inline-block", flexShrink: 0 }} />
                  {truncateAddress(X402_SERVER_ADDRESS)}
                </div>
              )}
              <span className="text-lg font-semibold text-[#192b25]">
                {serverBalanceLoading
                  ? <span className="text-zinc-400 font-normal">loading…</span>
                  : serverBalance !== null
                    ? <>{serverBalance} <span className="font-medium text-zinc-400">USDC</span></>
                    : <span className="text-zinc-400 font-normal">—</span>}
              </span>
            </div>

            <p className="text-base text-zinc-500 leading-relaxed mt-5">
              This pricing tier breakdown is reconstructed from the server's records of each payment's plaintext amount and the corresponding price tier. No public on-chain data reveals how many payments were made at each tier, or how much revenue each tier generated. With <em>Standard x402</em>, all this information would be visible on-chain as plain ERC-20 transfers. Switch to <em>Confidential x402</em> to see how it looks on-chain.
            </p>

            {/* Mode toggle */}
            <div className="flex items-center justify-center mt-5 mb-4">
              <X402ModeToggle mode={x402Mode} onChange={setX402Mode} />
            </div>

            {/* Pricing tier chart */}
            <div className="mt-4 rounded-[0.5rem] border border-zinc-200 bg-white p-5 shadow-[0_2px_4px_rgba(0,0,0,0.04)]">
              <TierBarChart stats={tierStats} txsLoading={txsLoading} txMode={x402Mode} />
            </div>
          </div>

          {/* Transaction log */}
          <div id="transaction-log" style={{ scrollMarginTop: "5rem" }}>
            <h2 className="text-xl font-medium text-zinc-900 mt-6 mb-3">On-chain transaction log</h2>
            <p className="text-base text-zinc-500 leading-relaxed">
              Every payment settles as an on-chain transaction. The toggle below switches between two views of the same data — the <em>Standard x402</em> view shows the plaintext amount, while the <em>Confidential x402</em> view shows only the amount commitment that appears on-chain. The amounts never touch the public chain in cleartext.
            </p>

            {/* Mode toggle */}
            <div className="flex items-center justify-center mt-5 mb-4">
              <X402ModeToggle mode={x402Mode} onChange={setX402Mode} />
            </div>

            <TxTable
              txs={txs}
              txsLoading={txsLoading}
              txMode={x402Mode}
              blockExplorerUrl={BLOCK_EXPLORER_URL}
            />
          </div>

          {/* Why privacy matters */}
          <div id="why-privacy" style={{ scrollMarginTop: "5rem" }}>
            <h2 className="text-xl font-medium text-zinc-900 mt-6 mb-3">Why payment privacy matters</h2>
            <p className="text-base text-zinc-500 leading-relaxed mb-3">
              Standard x402 settles payments as plain ERC-20 token transfers — every amount is permanently visible on-chain. This works for flat-rate APIs, but breaks down the moment pricing becomes dynamic:
            </p>
            <ul className="flex flex-col gap-3 pl-4">
              <li className="text-base text-zinc-500 leading-relaxed list-disc">
                <span className="font-medium text-zinc-700">Competitors read your pricing strategy off the blockchain.</span>{" "}
                Every <code>transferWithAuthorization</code> call exposes exactly what each customer paid — volume discounts, enterprise rates, and promotional pricing become public record.
              </li>
              <li className="text-base text-zinc-500 leading-relaxed list-disc">
                <span className="font-medium text-zinc-700">Per-customer deals are impossible to keep confidential.</span>{" "}
                A buyer on a higher tier cites on-chain evidence to demand the rate paid by others.
              </li>
              <li className="text-base text-zinc-500 leading-relaxed list-disc">
                <span className="font-medium text-zinc-700">AI agents reveal their economic strategy.</span>{" "}
                Spending patterns across API providers expose which data sources an agent values — and by how much budget it allocates to each.
              </li>
            </ul>
            <p className="text-base text-zinc-500 leading-relaxed mt-4">
              With <em>Confidential x402</em>, the on-chain record reveals that a payment was made — including sender and receiver addresses — but not how much. Privacy is enforced by a combination of Multi-Party Computation (MPC) and Zero-Knowledge Proofs (ZKP), so no single party ever sees the plaintext amount.
            </p>
          </div>

        </article>

        {/* TOC — right, sticky */}
        <aside className="lg:flex-none lg:w-52 lg:sticky lg:top-14 lg:self-start hidden lg:block lg:pt-12">
          <p className="text-xs font-semibold uppercase tracking-wider text-zinc-400 mb-3 px-2">On this page</p>
          <nav className="flex flex-col">
            {tocItems.map(({ label, href }) => (
              <a
                key={href}
                href={href}
                className="text-sm text-zinc-500 hover:text-zinc-900 px-2 py-1 rounded transition-colors leading-snug"
              >
                {label}
              </a>
            ))}
          </nav>
        </aside>

      </div>
      </main>

      <Footer />
      </div>
    </div>
  );
}
