import * as Select from "@radix-ui/react-select";
import { PRICE_TIERS } from "@/components/TierBarChart";

const TIER_OPTIONS = [
  { value: "Standard",   label: "Standard",   price: "1.00" },
  { value: "STARTUP",    label: "STARTUP",    price: "0.20" },
  { value: "GROWTH",     label: "GROWTH",     price: "0.80" },
  { value: "ENTERPRISE", label: "ENTERPRISE", price: "1.50" },
];

interface PriceTierSelectProps {
  value: string;
  onChange: (tier: string) => void;
  className?: string;
}

export default function PriceTierSelect({ value, onChange, className }: PriceTierSelectProps) {
  const selected = PRICE_TIERS.find((t) => t.tier === (value || "Standard"))!;
  const radixValue = value || "Standard";

  return (
    <div className="inline-flex items-center gap-3">
    <Select.Root
      value={radixValue}
      onValueChange={(v) => onChange(v === "Standard" ? "" : v)}
    >
      <Select.Trigger
        className={`inline-flex items-center gap-2 h-9 rounded-[0.5rem] border border-zinc-200 bg-white pl-3 pr-2.5 text-sm text-zinc-800 focus:outline-none focus:ring-1 focus:ring-zinc-300 cursor-pointer select-none data-[state=open]:ring-1 data-[state=open]:ring-zinc-300 ${className ?? ""}`}
      >
        <span
          className="w-2 h-2 rounded-sm shrink-0"
          style={{ background: selected.color }}
        />
        <Select.Value />
        <Select.Icon>
          <svg width="14" height="14" viewBox="0 0 12 12" fill="none">
            <path d="M2.5 4.5L6 8L9.5 4.5" stroke="#a1a1aa" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
          </svg>
        </Select.Icon>
      </Select.Trigger>

      <Select.Portal>
        <Select.Content
          position="popper"
          sideOffset={5}
          className="z-50 min-w-[10rem] overflow-hidden rounded-lg border border-zinc-200 bg-white shadow-lg animate-in fade-in-0 zoom-in-95"
        >
          <Select.Viewport className="p-1">
            {TIER_OPTIONS.map(({ value: v, label, price }) => {
              const tier = PRICE_TIERS.find((t) => t.tier === v)!;
              return (
                <Select.Item
                  key={v}
                  value={v}
                  className="relative flex items-center gap-2.5 rounded-md px-3 py-2 text-sm text-zinc-800 cursor-pointer select-none outline-none hover:bg-zinc-50 focus:bg-zinc-50 data-[highlighted]:bg-zinc-50"
                >
                  <span
                    className="w-2 h-2 rounded-sm shrink-0"
                    style={{ background: tier.color }}
                  />
                  <Select.ItemText>{label}</Select.ItemText>
                  <span className="ml-auto text-xs text-zinc-400">${price}</span>
                </Select.Item>
              );
            })}
          </Select.Viewport>
        </Select.Content>
      </Select.Portal>
    </Select.Root>
    <span className="text-lg font-semibold text-[#192b25]">
      {selected.price} <span className="font-medium text-zinc-400">USDC</span>
    </span>
    </div>
  );
}
