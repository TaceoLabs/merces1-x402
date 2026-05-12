import SpinnerButton from "@/components/SpinnerButton";

interface FaucetButtonProps {
  onClick: () => void;
  loading?: boolean;
  disabled?: boolean;
}

export default function FaucetButton({ onClick, disabled, loading }: FaucetButtonProps) {
  return (
    <span className="relative group inline-flex">
      <SpinnerButton
        onClick={onClick}
        loading={loading}
        disabled={disabled}
        loadingLabel="Claiming…"
        className="h-9 px-4 rounded-[0.5rem] border border-zinc-200 bg-white text-sm font-medium text-zinc-800 hover:bg-zinc-50 transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
      >
        Claim 1,000 USDC from faucet
      </SpinnerButton>
      {disabled && (
        <span className="pointer-events-none absolute bottom-full left-1/2 -translate-x-1/2 mb-2 whitespace-nowrap rounded bg-zinc-800 px-2 py-1 text-xs text-white opacity-0 group-hover:opacity-100 transition-opacity">
          Connect a wallet first
        </span>
      )}
    </span>
  );
}
