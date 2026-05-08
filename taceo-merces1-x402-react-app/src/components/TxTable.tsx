import { useState, useMemo } from "react";
import { inferPriceTier, PRICE_TIERS } from "@/components/TierBarChart";
import { type Transfer } from "@/lib/api";
import { formatUSDC, truncateAddress } from "@/lib/utils";
import { X402Mode } from "./X402ModeToggle";

export type { Transfer };

const PAGE_SIZE = 5;

function formatTimestamp(d: Date): { date: string; time: string } {
  const locale = navigator.language;
  const timeZone = Intl.DateTimeFormat().resolvedOptions().timeZone;
  return {
    date: d.toLocaleDateString(locale, { month: "short", day: "numeric", year: "numeric", timeZone }),
    time: d.toLocaleTimeString(locale, { hour: "2-digit", minute: "2-digit", timeZone }),
  };
}

function buildPaginationItems(currentPage: number, totalPages: number): (number | "ellipsis")[] {
  if (totalPages <= 7) return Array.from({ length: totalPages }, (_, i) => i + 1);
  if (currentPage <= 4) return [1, 2, 3, 4, 5, "ellipsis", totalPages];
  if (currentPage >= totalPages - 3) return [1, "ellipsis", totalPages - 4, totalPages - 3, totalPages - 2, totalPages - 1, totalPages];
  return [1, "ellipsis", currentPage - 1, currentPage, currentPage + 1, "ellipsis", totalPages];
}


export default function TxTable({
  txs,
  txsLoading,
  txMode,
  blockExplorerUrl,
  emptyMessage = "No transactions yet.",
}: {
  txs: Transfer[];
  txsLoading: boolean;
  txMode: X402Mode;
  blockExplorerUrl?: string;
  emptyMessage?: string;
}) {
  const [currentPage, setCurrentPage] = useState(1);
  const totalPages = Math.ceil(txs.length / PAGE_SIZE);
  const paginationItems = useMemo(() => buildPaginationItems(currentPage, totalPages), [currentPage, totalPages]);
  const pagedTxs = txs.slice((currentPage - 1) * PAGE_SIZE, currentPage * PAGE_SIZE);

  return (
    <div>
      <div className="overflow-x-auto rounded-[0.5rem] border border-zinc-200 bg-white shadow-[0_2px_4px_rgba(0,0,0,0.04)]">
        <table className="w-full min-w-[36rem] border-collapse text-left">
          <thead>
            <tr className="bg-[#f4f4f5]">
              <th className="px-4 py-3 text-xs font-medium text-zinc-500 uppercase tracking-wider w-1/5">Tx</th>
              <th className="px-4 py-3 text-xs font-medium text-zinc-500 uppercase tracking-wider w-1/5">Sender</th>
              <th className="px-4 py-3 text-xs font-medium text-zinc-500 uppercase tracking-wider w-1/5">Receiver</th>
              <th className="px-4 py-3 text-xs font-medium text-zinc-500 uppercase tracking-wider w-1/5">
                <span className="flex items-center gap-2">
                  Amount
                  {txMode === "standard" ? (
                    <span className="inline-flex items-center gap-1 text-[10px] font-medium text-red-500 bg-red-50 border border-red-200 px-1.5 py-0.5 rounded-full normal-case tracking-normal">
                      <svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" /><circle cx="12" cy="12" r="3" /></svg>
                      public
                    </span>
                  ) : (
                    <span className="inline-flex items-center gap-1 text-[10px] font-medium text-zinc-400 bg-zinc-100 border border-zinc-200 px-1.5 py-0.5 rounded-full normal-case tracking-normal">
                      <svg xmlns="http://www.w3.org/2000/svg" width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24" /><line x1="1" y1="1" x2="23" y2="23" /></svg>
                      hidden
                    </span>
                  )}
                </span>
              </th>
              <th className="px-4 py-3 text-xs font-medium text-zinc-500 uppercase tracking-wider w-1/5">Timestamp</th>
            </tr>
          </thead>
          <tbody>
            {txsLoading ? (
              <tr>
                <td colSpan={5} className="px-4 py-10 text-sm text-center text-zinc-400">Loading…</td>
              </tr>
            ) : pagedTxs.length === 0 ? (
              <tr>
                <td colSpan={5} className="px-4 py-10 text-sm text-center text-zinc-400">{emptyMessage}</td>
              </tr>
            ) : pagedTxs.map((tx) => {
              const { date, time } = formatTimestamp(tx.timestamp);
              const txLabel = tx.txHash ? `${tx.txHash.slice(0, 4)}...${tx.txHash.slice(-4)}` : `#${tx.id}`;
              const explorerHref = tx.txHash && blockExplorerUrl ? `${blockExplorerUrl}/tx/${tx.txHash}` : null;

              return (
                <tr key={tx.id} className="border-t border-zinc-100 hover:bg-zinc-50 transition-colors">
                  <td className="px-4 py-2.5 text-sm font-semibold">
                    {explorerHref
                      ? <a href={explorerHref} target="_blank" rel="noopener noreferrer" title={tx.txHash ?? undefined} className="hover:underline">{txLabel}</a>
                      : <span title={tx.txHash ?? undefined}>{txLabel}</span>}
                  </td>
                  <td className="px-4 py-2.5 text-sm font-mono text-zinc-600" title={tx.sender}>
                    {truncateAddress(tx.sender)}
                  </td>
                  <td className="px-4 py-2.5 text-sm font-mono text-zinc-600" title={tx.receiver}>
                    {truncateAddress(tx.receiver)}
                  </td>
                  <td className="px-4 py-2.5 text-sm font-semibold">
                    {txMode === "standard" ? (() => {
                      const tier = inferPriceTier(tx.amount);
                      const { color } = PRICE_TIERS.find((t) => t.tier === tier)!;
                      const bg = tier === "Standard" ? "#d4d4d8" : color;
                      const fg = tier === "Standard" ? "#525252" : "#173f36";
                      return (
                        <span className="inline-block px-2 py-0.5 rounded-full text-xs font-semibold" style={{ background: bg, color: fg }}>
                          {formatUSDC(tx.amount)} USDC
                        </span>
                      );
                    })() : (
                      <span className="text-zinc-700 font-mono">•••••••</span>
                    )}
                  </td>
                  <td className="px-4 py-2.5">
                    <div className="flex flex-col gap-0.5">
                      <span className="text-sm leading-5 text-zinc-700">{date}</span>
                      <span className="text-xs tracking-[0.02em] text-zinc-400">{time}</span>
                    </div>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {totalPages > 1 && (
        <div className="flex items-center justify-between mt-3 px-1">
          <button
            onClick={() => setCurrentPage((p) => Math.max(1, p - 1))}
            disabled={currentPage === 1}
            className="inline-flex items-center gap-1.5 h-9 px-3 rounded-md text-sm font-medium text-zinc-700 hover:bg-zinc-100 transition-colors cursor-pointer border-0 bg-transparent disabled:text-zinc-400 disabled:cursor-not-allowed"
          >
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><polyline points="15 18 9 12 15 6" /></svg>
            Previous
          </button>
          <div className="flex items-center gap-1">
            {paginationItems.map((item, index) =>
              item === "ellipsis" ? (
                <span key={`ellipsis-${index}`} className="inline-flex size-8 items-center justify-center text-sm text-zinc-400">…</span>
              ) : (
                <button
                  key={item}
                  onClick={() => setCurrentPage(item as number)}
                  className={`inline-flex size-8 items-center justify-center rounded-[0.5rem] text-xs font-normal border-0 cursor-pointer transition-colors ${item === currentPage ? "bg-[#e5fff6] text-[#025964]" : "bg-transparent text-zinc-700 hover:bg-zinc-100"}`}
                >
                  {item}
                </button>
              )
            )}
          </div>
          <button
            onClick={() => setCurrentPage((p) => Math.min(totalPages, p + 1))}
            disabled={currentPage === totalPages}
            className="inline-flex items-center gap-1.5 h-9 px-3 rounded-md text-sm font-medium text-zinc-700 hover:bg-zinc-100 transition-colors cursor-pointer border-0 bg-transparent disabled:text-zinc-400 disabled:cursor-not-allowed"
          >
            Next
            <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><polyline points="9 18 15 12 9 6" /></svg>
          </button>
        </div>
      )}
    </div>
  );
}
