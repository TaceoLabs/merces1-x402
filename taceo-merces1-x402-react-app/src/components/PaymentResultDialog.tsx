import { BLOCK_EXPLORER_URL } from "@/lib/constants";

interface PaymentSettleResponse {
  success: boolean;
  transaction?: string;
  network?: string;
  payer?: string;
}

interface Props {
  content: string;
  paymentResponse: PaymentSettleResponse | null;
  onClose: () => void;
}

export default function PaymentResultDialog({ content, paymentResponse, onClose }: Props) {
  const txHash = paymentResponse?.transaction;
  const explorerHref = txHash && BLOCK_EXPLORER_URL ? `${BLOCK_EXPLORER_URL}/tx/${txHash}` : null;

  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4" onClick={onClose}>
      <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" />
      <div
        className="relative z-10 w-full max-w-lg rounded-xl border border-zinc-200 bg-white shadow-xl flex flex-col"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-zinc-100">
          <div className="flex items-center gap-2.5">
            <span className="flex h-5 w-5 items-center justify-center rounded-full bg-[#52ffc5]">
              <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
                <path d="M2 5l2.5 2.5L8 3" stroke="#173f36" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round"/>
              </svg>
            </span>
            <p className="text-sm font-semibold text-zinc-900">Payment successful</p>
          </div>
          <button
            onClick={onClose}
            aria-label="Close"
            className="flex items-center justify-center h-7 w-7 rounded-md text-zinc-400 hover:text-zinc-700 hover:bg-zinc-100 transition-colors cursor-pointer border-0 bg-transparent"
          >
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none">
              <path d="M1 1L13 13M13 1L1 13" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round"/>
            </svg>
          </button>
        </div>

        {/* Body */}
        <div className="flex flex-col gap-5 p-5 overflow-y-auto max-h-[70vh]">

          {paymentResponse && (
            <div className="flex flex-col gap-3.5">
              {txHash && (
                <div className="flex flex-col gap-1">
                  <p className="text-sm font-semibold text-zinc-700">Transaction</p>
                  {explorerHref ? (
                    <a href={explorerHref} target="_blank" rel="noopener noreferrer" className="font-mono text-sm hover:underline" title={txHash}>
                      {txHash.slice(0, 10)}…{txHash.slice(-8)}
                    </a>
                  ) : (
                    <span className="font-mono text-sm text-zinc-700" title={txHash}>{txHash.slice(0, 10)}…{txHash.slice(-8)}</span>
                  )}
                </div>
              )}
              {paymentResponse.network && (
                <div className="flex flex-col gap-1">
                  <p className="text-sm font-semibold text-zinc-700">Network</p>
                  <span className="text-sm text-zinc-600">{paymentResponse.network}</span>
                </div>
              )}
              {paymentResponse.payer && (
                <div className="flex flex-col gap-1">
                  <p className="text-sm font-semibold text-zinc-700">Payer</p>
                  <span className="font-mono text-sm text-zinc-600">{paymentResponse.payer}</span>
                </div>
              )}
            </div>
          )}

          <div className="border-t border-zinc-100" />

          <div className="flex flex-col gap-1">
            <p className="text-sm font-semibold text-zinc-700">Response body</p>
            <div className="rounded-lg border border-zinc-200 bg-[#f9f8f5] px-4 py-3">
              <pre className="text-sm whitespace-pre-wrap break-all text-zinc-700">{JSON.stringify(content, null, 2)}</pre>
            </div>
          </div>
        </div>

        {/* Footer */}
        <div className="flex justify-end px-5 py-4 border-t border-zinc-100">
          <button
            onClick={onClose}
            className="inline-flex items-center justify-center h-8 px-4 rounded-lg bg-green-50 border border-green-200 text-sm font-medium text-green-700 hover:bg-green-100 transition-colors cursor-pointer"
          >
            Close
          </button>
        </div>
      </div>
    </div>
  );
}
