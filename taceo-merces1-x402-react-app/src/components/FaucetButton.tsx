import SpinnerButton from "@/components/SpinnerButton";
import TooltipWrapper from "@/components/TooltipWrapper";

interface FaucetButtonProps {
  onClick: () => void;
  loading?: boolean;
  disabled?: boolean;
  tooltip?: string;
}

export default function FaucetButton({ onClick, disabled, loading, tooltip }: FaucetButtonProps) {
  return (
    <TooltipWrapper text={tooltip ?? ""} show={!!tooltip}>
      <SpinnerButton
        onClick={onClick}
        loading={loading}
        disabled={disabled}
        loadingLabel="Claiming…"
        className="h-9 px-4 rounded-[0.5rem] border border-zinc-200 bg-white text-sm font-medium text-zinc-800 hover:bg-zinc-50 transition-colors cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
      >
        Claim 1,000 USDC from faucet
      </SpinnerButton>
    </TooltipWrapper>
  );
}
