const STEPS = [
  {
    title: "GET /api/protected",
    description: "Initial request to the protected endpoint",
    actor: "client" as const,
  },
  {
    title: "402 Payment Required",
    description: "Server returns payment requirements",
    actor: "server" as const,
  },
  {
    title: "ZK proof generation",
    description: "Confidential payment proof generated locally",
    actor: "client" as const,
  },
  {
    title: "GET /api/protected",
    description: "Request retried with payment proof attached",
    actor: "client" as const,
  },
  {
    title: "Facilitator /verify",
    description: "Server verifies the payment proof",
    actor: "facilitator" as const,
  },
  {
    title: "Facilitator /settle",
    description: "Server settles the confidential payment",
    actor: "facilitator" as const,
  },
  {
    title: "Content + receipt",
    description: "Protected content and payment response delivered",
    actor: "server" as const,
  },
];

const ACTOR_STYLES = {
  client: "bg-sky-50 text-sky-600",
  server: "bg-violet-50 text-violet-600",
  facilitator: "bg-amber-50 text-amber-600",
} as const;

interface Props {
  // null = idle, 0–6 = that step is active, 7 = all complete
  step: number | null;
}

export default function RequestFlowStepper({ step }: Props) {
  const allDone = step !== null && step >= STEPS.length;

  return (
    <div className="rounded-lg border border-zinc-200 bg-white p-6">
      <div className="flex items-center justify-between mb-6">
        <div>
          <p className="text-sm font-medium text-zinc-700">Request flow</p>
          <p className="text-xs text-zinc-400 mt-0.5">confidential x402 lifecycle</p>
        </div>
        {allDone && (
          <span className="inline-flex items-center gap-1.5 text-xs font-medium text-emerald-700 bg-emerald-50 border border-emerald-200 px-2.5 py-1 rounded-full">
            <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
              <path d="M2 5l2 2 4-4" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
            </svg>
            Complete
          </span>
        )}
        {step !== null && !allDone && (
          <span className="inline-flex items-center gap-1.5 text-xs font-medium text-zinc-500 bg-zinc-50 border border-zinc-200 px-2.5 py-1 rounded-full">
            <span className="w-1.5 h-1.5 rounded-full bg-[#52ffc5] animate-pulse" />
            In progress
          </span>
        )}
      </div>

      <div className="flex flex-col">
        {STEPS.map((s, i) => {
          const isComplete = step !== null && i < step;
          const isActive = step === i;
          const isLast = i === STEPS.length - 1;

          return (
            <div key={i} className="flex gap-4">
              {/* Timeline column */}
              <div className="flex flex-col items-center">
                {/* Circle */}
                <div
                  className={`w-7 h-7 rounded-full flex items-center justify-center shrink-0 transition-all duration-500 ${
                    isComplete
                      ? "bg-[#52ffc5]"
                      : isActive
                        ? "ring-2 ring-[#52ffc5] bg-[#52ffc5]/10"
                        : "bg-zinc-100"
                  }`}
                >
                  {isComplete ? (
                    <svg width="11" height="11" viewBox="0 0 11 11" fill="none">
                      <path
                        d="M2 5.5l2.5 2.5 4.5-4.5"
                        stroke="#14532d"
                        strokeWidth="1.5"
                        strokeLinecap="round"
                        strokeLinejoin="round"
                      />
                    </svg>
                  ) : isActive ? (
                    <svg className="animate-spin" width="14" height="14" viewBox="0 0 14 14" fill="none">
                      <circle cx="7" cy="7" r="5" stroke="#d4fae8" strokeWidth="2" />
                      <path d="M7 2A5 5 0 0 1 12 7" stroke="#52ffc5" strokeWidth="2" strokeLinecap="round" />
                    </svg>
                  ) : (
                    <span className="text-[10px] font-semibold text-zinc-400">{i + 1}</span>
                  )}
                </div>

                {/* Connector line */}
                {!isLast && (
                  <div
                    className={`w-px transition-colors duration-500 ${
                      isComplete ? "bg-[#52ffc5]" : "bg-zinc-100"
                    }`}
                    style={{ height: "24px", marginTop: "3px", marginBottom: "3px" }}
                  />
                )}
              </div>

              {/* Content */}
              <div className="flex-1 pb-1" style={{ minHeight: "36px" }}>
                <div className="flex items-center gap-2 flex-wrap">
                  <span
                    className={`text-sm font-medium font-mono transition-colors duration-300 ${
                      isComplete
                        ? "text-zinc-600"
                        : isActive
                          ? "text-zinc-900"
                          : "text-zinc-300"
                    }`}
                  >
                    {s.title}
                  </span>
                  <span
                    className={`text-[10px] font-medium px-1.5 py-0.5 rounded transition-opacity duration-300 ${ACTOR_STYLES[s.actor]} ${
                      isComplete || isActive ? "opacity-100" : "opacity-25"
                    }`}
                  >
                    {s.actor}
                  </span>
                </div>
                <p
                  className={`text-xs mt-0.5 transition-colors duration-300 ${
                    isComplete || isActive ? "text-zinc-400" : "text-zinc-200"
                  }`}
                >
                  {s.description}
                </p>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}
