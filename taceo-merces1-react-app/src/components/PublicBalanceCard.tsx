import gradientStyles from "@/styles/card-gradients.module.css";

type PublicBalanceCardProps = {
  balance: string | null;
  loading: boolean;
};

export function PublicBalanceCard({ balance, loading }: PublicBalanceCardProps) {
  const balanceValue = loading ? "…" : balance !== null ? balance : "—";

  return (
    <article
      aria-label={`Public Account, ${balanceValue} USDC`}
      className="relative grid w-full overflow-hidden rounded-[0.625rem] border border-[#E4E4E7] text-white"
      style={{ aspectRatio: "472 / 268" }}
    >
      <div
        aria-hidden="true"
        className={`col-start-1 row-start-1 z-0 h-full w-full ${gradientStyles.publicCardGradient}`}
      />

      <div className="col-start-1 row-start-1 z-10 flex h-full flex-col justify-between p-5">
        <header>
          <p className="flex items-start gap-2 text-[1rem] leading-[1.25rem]">
            <span className="font-semibold">Public Account</span>
            <span aria-hidden="true" className="text-white/70">•</span>
            <span className="font-normal text-white/75">USDC</span>
          </p>
        </header>

        <footer className="grid grid-cols-[minmax(0,1fr)_auto] items-end gap-x-3 gap-y-2">
          <p className="flex items-end gap-2 text-[2.75rem] leading-none font-medium">
            <span>{balanceValue}</span>
            <span className="pb-[0.1875rem] text-[1rem] leading-[1.25rem] font-normal uppercase text-white">
              USDC
            </span>
          </p>
        </footer>
      </div>
    </article>
  );
}
