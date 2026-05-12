import SpinnerButton from "@/components/SpinnerButton";

interface PayButtonProps {
  onClick: () => void;
  loading?: boolean;
  disabled?: boolean;
}

export default function PayButton({ onClick, loading, disabled }: PayButtonProps) {
  return (
    <span className="relative group inline-flex">
      <SpinnerButton
        onClick={onClick}
        disabled={disabled}
        loading={loading}
        loadingLabel="Paying…"
        className="h-9 px-4 rounded-[0.5rem] bg-[#52ffc5] text-sm font-semibold text-zinc-900 hover:bg-[#33e0a8] transition-colors cursor-pointer border-0 disabled:opacity-50 disabled:cursor-not-allowed"
      >
        Access protected content
      </SpinnerButton>
      {disabled && (
        <span className="pointer-events-none absolute bottom-full left-1/2 -translate-x-1/2 mb-2 whitespace-nowrap rounded bg-zinc-800 px-2 py-1 text-xs text-white opacity-0 group-hover:opacity-100 transition-opacity">
          Connect a wallet first
        </span>
      )}
    </span>
  );
}
