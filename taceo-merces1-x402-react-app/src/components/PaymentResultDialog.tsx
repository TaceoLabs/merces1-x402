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
  return (
    <div className="fixed inset-0 z-50 flex items-center justify-center p-4" onClick={onClose}>
      <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" />
      <div
        className="relative z-10 w-full max-w-lg rounded-xl border border-zinc-200 bg-white shadow-xl flex flex-col"
        onClick={(e) => e.stopPropagation()}
      >
        {/* Header */}
        <div className="flex items-center justify-between px-5 py-4 border-b border-zinc-100">
          <p className="text-sm font-semibold text-[#192b25]">Payment successful</p>
          <button
            onClick={onClose}
            aria-label="Close"
            className="flex items-center justify-center h-7 w-7 rounded-md text-zinc-400 hover:text-zinc-700 hover:bg-zinc-100 transition-colors cursor-pointer border-0 bg-transparent"
          >
            <svg width="14" height="14" viewBox="0 0 14 14" fill="none" xmlns="http://www.w3.org/2000/svg">
              <path d="M1 1L13 13M13 1L1 13" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round"/>
            </svg>
          </button>
        </div>

        {/* Body */}
        <div className="flex flex-col gap-4 p-5 overflow-y-auto max-h-[70vh]">
          <div>
            <p className="text-xs font-medium text-zinc-400 uppercase tracking-wider mb-1.5">Response body</p>
            <div className="rounded-lg border border-zinc-200 bg-[#f9f8f5] px-4 py-3">
              <pre className="text-sm whitespace-pre-wrap break-all text-zinc-700">{JSON.stringify(content, null, 2)}</pre>
            </div>
          </div>
          {paymentResponse && (
            <div>
              <p className="text-xs font-medium text-zinc-400 uppercase tracking-wider mb-1.5">Payment response</p>
              <div className="rounded-lg border border-zinc-200 bg-[#f9f8f5] px-4 py-3">
                <pre className="text-sm whitespace-pre-wrap break-all text-zinc-700">{JSON.stringify(paymentResponse, null, 2)}</pre>
              </div>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
