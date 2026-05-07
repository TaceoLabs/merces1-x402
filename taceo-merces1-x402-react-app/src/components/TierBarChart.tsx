import { formatUnits } from "viem";
import { formatUSDC } from "@/lib/utils";
import {
  BarChart,
  Bar,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  Cell,
} from "recharts";

export type PriceTier = "Standard" | "STARTUP" | "GROWTH" | "ENTERPRISE";
export type X402Mode = "normal" | "confidential";

export const PRICE_TIERS: { tier: PriceTier; label: string; price: string; color: string }[] = [
  { tier: "Standard",   label: "Standard",   price: "1.00", color: "#f4f4f5" },
  { tier: "STARTUP",    label: "STARTUP",    price: "0.20", color: "#52ffc5" },
  { tier: "GROWTH",     label: "GROWTH",     price: "0.80", color: "#a7f3d0" },
  { tier: "ENTERPRISE", label: "ENTERPRISE", price: "1.50", color: "#d1fae5" },
];

export function inferPriceTier(amount: bigint): PriceTier {
  const usdc = Number(formatUnits(amount, 6));
  if (Math.abs(usdc - 0.2) < 0.01) return "STARTUP";
  if (Math.abs(usdc - 0.8) < 0.01) return "GROWTH";
  if (Math.abs(usdc - 1.5) < 0.01) return "ENTERPRISE";
  return "Standard";
}

export interface TierStats {
  totalRevenue: bigint;
  avgPayment: bigint;
  tierCounts: Record<PriceTier, number>;
  tierRevenue: Record<PriceTier, bigint>;
}

interface TierChartDatum {
  tier: PriceTier;
  label: string;
  price: string;
  color: string;
  count: number;
  revenue: bigint;
  pct: number;
}

interface TooltipProps {
  active?: boolean;
  payload?: { payload: TierChartDatum }[];
  txMode: X402Mode;
}

function makeRevenueLabel(txMode: X402Mode, data: TierChartDatum[]) {
  return function RevenueLabel(props: object) {
    const { x = 0, y = 0, width = 0, index = -1 } = props as {
      x?: number; y?: number; width?: number; index?: number;
    };
    const datum = data[index];
    const count = datum?.count ?? 0;
    const revenue = datum?.revenue ?? BigInt(0);

    if (count === 0) return null;

    const cx = x + width / 2;

    if (txMode === "normal") {
      return (
        <foreignObject x={cx - 52} y={y - 40} width={104} height={38}>
          <div style={{ display: "flex", flexDirection: "column", alignItems: "center", gap: 2 }}>
            <span style={{
              fontSize: 10, fontWeight: 600, color: "#ef4444",
              background: "#fef2f2", border: "1px solid #fecaca",
              padding: "1px 5px", borderRadius: 9999,
              display: "inline-flex", alignItems: "center", gap: 3,
              whiteSpace: "nowrap",
            }}>
              <svg xmlns="http://www.w3.org/2000/svg" width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" /><circle cx="12" cy="12" r="3" /></svg>
              public
            </span>
            <span style={{ fontSize: 10, fontWeight: 600, color: "#3f3f46", whiteSpace: "nowrap" }}>
              {formatUSDC(revenue)} USDC
            </span>
          </div>
        </foreignObject>
      );
    }

    return (
      <foreignObject x={cx - 32} y={y - 22} width={64} height={20}>
        <div style={{ display: "flex", justifyContent: "center" }}>
          <span style={{
            fontSize: 10, fontWeight: 500, color: "#a1a1aa",
            background: "#f4f4f5", border: "1px solid #e4e4e7",
            padding: "1px 5px", borderRadius: 9999,
            display: "inline-flex", alignItems: "center", gap: 3,
            whiteSpace: "nowrap",
          }}>
            <svg xmlns="http://www.w3.org/2000/svg" width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24" /><line x1="1" y1="1" x2="23" y2="23" /></svg>
            hidden
          </span>
        </div>
      </foreignObject>
    );
  };
}

function PublicFlair() {
  return (
    <span className="inline-flex items-center gap-1 text-[10px] font-medium text-red-500 bg-red-50 border border-red-200 px-1.5 py-0.5 rounded-full">
      <svg xmlns="http://www.w3.org/2000/svg" width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" /><circle cx="12" cy="12" r="3" /></svg>
      public
    </span>
  );
}

function HiddenFlair() {
  return (
    <span className="inline-flex items-center gap-1 text-[10px] font-medium text-zinc-400 bg-zinc-100 border border-zinc-200 px-1.5 py-0.5 rounded-full">
      <svg xmlns="http://www.w3.org/2000/svg" width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24" /><line x1="1" y1="1" x2="23" y2="23" /></svg>
      hidden
    </span>
  );
}

function TierTooltip({ active, payload, txMode }: TooltipProps) {
  if (!active || !payload?.length) return null;
  const d = payload[0].payload;
  const isNormal = txMode === "normal";
  return (
    <div className="rounded-lg border border-zinc-200 bg-white shadow-lg px-3.5 py-3 text-xs flex flex-col gap-1.5 min-w-[140px]">
      <div className="flex items-center gap-2 mb-0.5">
        <span
          className="px-2 py-0.5 rounded font-semibold text-[11px]"
          style={{ background: d.color, color: d.tier === "Standard" ? "#525252" : "#173f36" }}
        >
          {d.label}
        </span>
      </div>
      <div className="flex justify-between items-center gap-4">
        <span className="text-zinc-500">Payments</span>
        {isNormal
          ? <span className="flex items-center gap-1.5">
              <span className="font-semibold text-zinc-800">{d.count} <span className="font-normal text-zinc-400">({d.pct}%)</span></span>
              <PublicFlair />
            </span>
          : <HiddenFlair />}
      </div>
      <div className="flex justify-between items-center gap-4">
        <span className="text-zinc-500">Price</span>
        {isNormal
          ? <span className="flex items-center gap-1.5">
              <span className="font-semibold text-zinc-800">{d.price} USDC</span>
              <PublicFlair />
            </span>
          : <HiddenFlair />}
      </div>
      <div className="flex justify-between items-center gap-4">
        <span className="text-zinc-500">Revenue</span>
        {isNormal
          ? <span className="flex items-center gap-1.5">
              <span className="font-semibold text-zinc-800">{formatUSDC(d.revenue)} USDC</span>
              <PublicFlair />
            </span>
          : <HiddenFlair />}
      </div>
    </div>
  );
}

export default function TierBarChart({
  stats,
  txsLoading,
  txMode,
}: {
  stats: TierStats | null;
  txsLoading: boolean;
  txMode: X402Mode;
}) {
  const data: TierChartDatum[] = PRICE_TIERS.map(({ tier, label, price, color }) => {
    const count = stats?.tierCounts[tier] ?? 0;
    const total = stats ? Object.values(stats.tierCounts).reduce((a, b) => a + b, 0) : 0;
    return {
      tier, label, price, color,
      count,
      revenue: stats?.tierRevenue[tier] ?? BigInt(0),
      pct: total > 0 ? Math.round((count / total) * 100) : 0,
    };
  });

  if (txsLoading) {
    return <div className="h-52 flex items-center justify-center text-sm text-zinc-400">Loading…</div>;
  }

  const isEmpty = data.every((d) => d.count === 0);

  return (
    <div className="flex flex-col gap-4">
      <div className="flex items-center gap-3 flex-wrap">
        {PRICE_TIERS.map(({ tier, label, color }) => (
          <span key={tier} className="flex items-center gap-1.5 text-xs text-zinc-500">
            <span className="w-2.5 h-2.5 rounded-sm shrink-0" style={{ background: tier === "Standard" ? "#d4d4d8" : color }} />
            {label}
          </span>
        ))}
      </div>
      <ResponsiveContainer width="100%" height={250}>
        <BarChart data={data} barCategoryGap="28%" margin={{ top: 20, right: 8, left: -16, bottom: 0 }}>
          <CartesianGrid strokeDasharray="3 3" stroke="#f4f4f5" vertical={false} />
          <XAxis dataKey="label" tick={{ fontSize: 11, fill: "#a1a1aa" }} axisLine={false} tickLine={false} />
          <YAxis allowDecimals={false} tick={{ fontSize: 11, fill: "#a1a1aa" }} axisLine={false} tickLine={false} />
          <Tooltip content={<TierTooltip txMode={txMode} />} cursor={{ fill: "rgba(0,0,0,0.03)", radius: 6 }} />
          <Bar dataKey="count" radius={[5, 5, 0, 0]} isAnimationActive={!isEmpty} label={{ content: makeRevenueLabel(txMode, data) }}>
            {data.map(({ tier, color }) => (
              <Cell key={tier} fill={tier === "Standard" ? "#d4d4d8" : color} />
            ))}
          </Bar>
        </BarChart>
      </ResponsiveContainer>
    </div>
  );
}
