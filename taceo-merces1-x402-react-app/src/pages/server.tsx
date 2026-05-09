import { useState, useEffect, useMemo } from "react";
import Footer from "@/components/Footer";
import Sidebar from "@/components/Sidebar";
import TierBarChart, { type PriceTier, inferPriceTier } from "@/components/TierBarChart";
import TxTable, { type Transfer } from "@/components/TxTable";
import X402ModeToggle, { X402Mode } from "@/components/X402ModeToggle";
import { BLOCK_EXPLORER_URL } from "@/lib/constants";
import { fetchTransactions } from "@/lib/api";
import { formatUSDC } from "@/lib/utils";

export default function ServerPage() {
  const [txs, setTxs] = useState<Transfer[]>([]);
  const [txsLoading, setTxsLoading] = useState(true);
  const [x402Mode, setX402Mode] = useState<X402Mode>("standard");

  const stats = useMemo(() => {
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

  async function refresh() {
    setTxsLoading(true);
    try {
      const fetchedTxs = await fetchTransactions();
      setTxs(fetchedTxs);
    } catch {
      // leave existing data on error
    } finally {
      setTxsLoading(false);
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  return (
    <div className="flex flex-col md:flex-row min-h-screen text-zinc-900 font-sans antialiased">
      <Sidebar />
      <div className="flex-1 flex flex-col min-w-0">
      {/* Main */}
      <main className="flex-1 flex flex-col px-6 py-12">
        <div className="w-full max-w-4xl mx-auto flex flex-col gap-8">

          {/* Title + toggle */}
          <div className="flex flex-col gap-4 sm:flex-row sm:items-start sm:justify-between">
            <div>
              <h1 className="text-3xl font-semibold tracking-tight text-zinc-900">Resource Server</h1>
              <p className="text-base text-zinc-500 mt-5">
                Shows the perspective of an API-provider that charges different prices per tier.
              </p>
            </div>
            <div className="sm:pt-1 sm:shrink-0">
              <X402ModeToggle mode={x402Mode} onChange={setX402Mode} />
            </div>
          </div>

          {/* Top stats row */}
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
            {/* Total payments */}
            <div className="rounded-lg border border-zinc-200 bg-white p-6 flex flex-col gap-2">
              <p className="text-xs font-medium text-zinc-400 uppercase tracking-wider">Total payments</p>
              <div className="text-4xl font-semibold text-[#192b25] leading-tight">
                {txsLoading ? <span className="text-zinc-400 text-base font-normal">Loading…</span> : txs.length}
              </div>
              <p className="text-[10px] text-zinc-400">all-time</p>
            </div>

            {/* Total revenue */}
            <div className="rounded-lg border border-zinc-200 bg-white p-6 flex flex-col gap-2">
              <div className="flex items-center justify-between gap-2">
                <p className="text-xs font-medium text-zinc-400 uppercase tracking-wider">Total revenue</p>
                {x402Mode === "standard" ? (
                  <span className="inline-flex items-center gap-1 text-[10px] font-medium text-red-500 bg-red-50 border border-red-200 px-1.5 py-0.5 rounded-full shrink-0">
                    <svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
                    public on-chain
                  </span>
                ) : (
                  <span className="inline-flex items-center gap-1 text-[10px] font-medium text-zinc-400 bg-zinc-100 border border-zinc-200 px-1.5 py-0.5 rounded-full shrink-0">
                    <svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"/><line x1="1" y1="1" x2="23" y2="23"/></svg>
                    hidden
                  </span>
                )}
              </div>
              <div className="text-4xl font-semibold text-[#192b25] leading-tight">
                {txsLoading
                  ? <span className="text-zinc-400 text-base font-normal">Loading…</span>
                  : x402Mode !== "standard"
                    ? <span className="text-zinc-300">???</span>
                    : stats
                      ? <>{formatUSDC(stats.totalRevenue)} <span className="text-base font-medium text-zinc-500">USDC</span></>
                      : <span className="text-zinc-400 text-base font-normal">—</span>}
              </div>
              <p className="text-[10px] text-zinc-400">sum of all payments</p>
            </div>

            {/* Average payment */}
            <div className="rounded-lg border border-zinc-200 bg-white p-6 flex flex-col gap-2">
              <div className="flex items-center justify-between gap-2">
                <p className="text-xs font-medium text-zinc-400 uppercase tracking-wider">Avg payment</p>
                {x402Mode === "standard" ? (
                  <span className="inline-flex items-center gap-1 text-[10px] font-medium text-red-500 bg-red-50 border border-red-200 px-1.5 py-0.5 rounded-full shrink-0">
                    <svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z"/><circle cx="12" cy="12" r="3"/></svg>
                    public on-chain
                  </span>
                ) : (
                  <span className="inline-flex items-center gap-1 text-[10px] font-medium text-zinc-400 bg-zinc-100 border border-zinc-200 px-1.5 py-0.5 rounded-full shrink-0">
                    <svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24"/><line x1="1" y1="1" x2="23" y2="23"/></svg>
                    hidden
                  </span>
                )}
              </div>
              <div className="text-4xl font-semibold text-[#192b25] leading-tight">
                {txsLoading
                  ? <span className="text-zinc-400 text-base font-normal">Loading…</span>
                  : x402Mode !== "standard"
                    ? <span className="text-zinc-300">???</span>
                    : stats
                      ? <>{formatUSDC(stats.avgPayment, 3)} <span className="text-base font-medium text-zinc-500">USDC</span></>
                      : <span className="text-zinc-400 text-base font-normal">—</span>}
              </div>
              <p className="text-[10px] text-zinc-400">per payment</p>
            </div>
          </div>

          {/* Promo tier breakdown */}
          <div>
            <h2 className="text-lg font-semibold text-zinc-700 mb-1">Pricing tier breakdown</h2>
            <p className="text-base text-zinc-500 leading-relaxed mb-3">
              In confidential mode, neither the payment amount nor the customer's price tier is visible on-chain. An outside observer cannot reconstruct this chart, the total revenue, or the average payment — the on-chain record contains only opaque commitments. The data here is tracked by the server directly; amounts can also be reconstructed from the MPC network.
            </p>
            <div className="rounded-lg border border-zinc-200 bg-white p-5">
              <TierBarChart stats={stats} txsLoading={txsLoading} txMode={x402Mode} />
            </div>
          </div>

          {/* Payments table */}
          <div>
            <h2 className="text-lg font-semibold text-zinc-700 mb-1">Payment history</h2>
            <p className="text-base text-zinc-500 leading-relaxed mb-3">
              A full log of every x402 payment received by this server. In confidential mode, amounts and price tiers are never exposed on-chain — the server tracks them directly, and they can be reconstructed from the MPC network if needed.
            </p>
            <TxTable
              txs={txs}
              txsLoading={txsLoading}
              txMode={x402Mode}
              blockExplorerUrl={BLOCK_EXPLORER_URL}
              emptyMessage="No payments yet."
            />
          </div>

        </div>
      </main>

      <Footer />
      </div>
    </div>
  );
}
