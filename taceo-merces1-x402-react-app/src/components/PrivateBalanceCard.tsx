import { Dialog } from "radix-ui";
import gradientStyles from "@/styles/card-gradients.module.css";

type PrivateBalanceCardProps = {
  balance: string | null;
  loading: boolean;
  onClaim?: () => void;
  claiming?: boolean;
  claimError?: string | null;
  onClaimErrorDismiss?: () => void;
};

export function PrivateBalanceCard({
  balance,
  loading,
  onClaim,
  claiming,
  claimError,
  onClaimErrorDismiss,
}: PrivateBalanceCardProps) {
  const balanceValue = loading ? "…" : balance !== null ? balance : "—";

  return (
    <>
      <article
        aria-label={`Private Account, ${balanceValue} USDC`}
        className="relative grid w-full overflow-hidden rounded-[0.625rem] border border-[#E4E4E7] text-white"
        style={{ aspectRatio: "472 / 268" }}
      >
        <div
          aria-hidden="true"
          className={`col-start-1 row-start-1 z-0 h-full w-full ${gradientStyles.privateCardGradient}`}
        />

        <div className="col-start-1 row-start-1 z-10 flex h-full flex-col justify-between p-5">
          <header>
            <p className="flex items-start gap-2 text-[1rem] leading-[1.25rem]">
              <span className="font-semibold">Private Account</span>
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
            {onClaim && (
              <div className="justify-self-end self-end">
                <button
                  onClick={onClaim}
                  disabled={claiming}
                  className="inline-flex items-center justify-center whitespace-nowrap rounded-md text-sm font-medium bg-black text-white hover:bg-black/90 h-8 px-3 transition-all disabled:opacity-50 disabled:pointer-events-none"
                >
                  {claiming ? "Claiming…" : "Claim Funds"}
                </button>
              </div>
            )}
          </footer>
        </div>
      </article>

      <Dialog.Root open={!!claimError} onOpenChange={(open: boolean) => !open && onClaimErrorDismiss?.()}>
        <Dialog.Portal>
          <Dialog.Overlay className="fixed inset-0 z-50 bg-black/50" />
          <Dialog.Content className="fixed left-1/2 top-1/2 z-50 w-full max-w-sm -translate-x-1/2 -translate-y-1/2 rounded-lg border border-zinc-200 bg-white p-6 shadow-lg outline-none">
            <Dialog.Title className="text-base font-semibold text-zinc-900">
              Daily limit reached
            </Dialog.Title>
            <Dialog.Description className="mt-2 text-sm text-zinc-600">
              {claimError}
            </Dialog.Description>
            <div className="mt-5 flex justify-end">
              <Dialog.Close className="inline-flex items-center justify-center whitespace-nowrap rounded-md border border-zinc-200 bg-white h-9 px-4 text-sm font-medium shadow-xs hover:bg-zinc-100 transition-all">
                Close
              </Dialog.Close>
            </div>
          </Dialog.Content>
        </Dialog.Portal>
      </Dialog.Root>
    </>
  );
}
