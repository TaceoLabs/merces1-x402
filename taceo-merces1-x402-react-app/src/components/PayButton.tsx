import SpinnerButton from "@/components/SpinnerButton";
import TooltipWrapper from "@/components/TooltipWrapper";

interface PayButtonProps {
  onClick: () => void;
  loading?: boolean;
  disabled?: boolean;
  tooltip?: string;
}

export default function PayButton({ onClick, loading, disabled, tooltip }: PayButtonProps) {
  return (
    <TooltipWrapper text={tooltip ?? ""} show={!!tooltip}>
      <SpinnerButton
        onClick={onClick}
        disabled={disabled}
        loading={loading}
        loadingLabel="Paying…"
        className="h-9 px-4 rounded-[0.5rem] bg-[#52ffc5] text-sm font-semibold text-zinc-900 hover:bg-[#33e0a8] transition-colors cursor-pointer border-0 disabled:opacity-50 disabled:cursor-not-allowed"
      >
        Access protected content
      </SpinnerButton>
    </TooltipWrapper>
  );
}
