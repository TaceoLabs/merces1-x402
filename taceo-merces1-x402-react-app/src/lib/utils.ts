import { formatUnits } from "viem";

export function formatUSDC(amount: bigint, decimals?: number): string {
  const s = formatUnits(amount, 6);
  const full = s.includes(".") ? s : `${s}.0`;
  if (decimals === undefined) return full;
  return parseFloat(full).toFixed(decimals);
}

export function truncateAddress(addr: string): string {
  return `${addr.slice(0, 4)}...${addr.slice(-4)}`;
}
