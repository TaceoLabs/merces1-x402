import SpinnerButton from "@/components/SpinnerButton";

interface PayButtonProps {
  onClick: () => void;
  loading?: boolean;
  disabled?: boolean;
}

export default function PayButton({ onClick, loading, disabled }: PayButtonProps) {
  return (
    <SpinnerButton
      onClick={onClick}
      disabled={disabled}
      loading={loading}
      loadingLabel="Paying…"
      className="h-9 px-4 rounded-[0.5rem] bg-[#52ffc5] text-sm font-semibold text-zinc-900 hover:bg-[#33e0a8] transition-colors cursor-pointer border-0 disabled:opacity-50 disabled:cursor-not-allowed"
    >
      Access protected content
    </SpinnerButton>
  );
}
