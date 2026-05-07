export type X402Mode = "normal" | "confidential";

export default function X402ModeToggle({
  mode,
  onChange,
}: {
  mode: X402Mode;
  onChange: (mode: X402Mode) => void;
}) {
  return (
    <div className="flex items-center gap-3">
      <button
        type="button"
        onClick={() => onChange("normal")}
        className="border-0 bg-transparent p-0 cursor-pointer"
      >
        <span className={`text-sm leading-5 tracking-[-0.01em] ${mode === "normal" ? "text-[#192b25] font-semibold" : "text-zinc-400 font-normal"}`}>
          Normal x402
        </span>
      </button>
      <button
        type="button"
        role="switch"
        aria-label="Transaction mode"
        aria-checked={mode === "confidential"}
        onClick={() => onChange(mode === "confidential" ? "normal" : "confidential")}
        className="relative flex w-14 items-center rounded-full border border-zinc-200 bg-[#f2f2f2] p-[3px] cursor-pointer focus-visible:outline-none"
        style={{ height: "calc(1.5rem + 6px)" }}
      >
        <span
          aria-hidden="true"
          className="w-6 h-6 shrink-0 rounded-full transition-transform duration-200 ease-out"
          style={{
            background: "radial-gradient(120% 95% at 24% 22%, #255b4d 0%, transparent 56%), radial-gradient(95% 95% at 70% 86%, #62ffd1 0%, transparent 62%), linear-gradient(145deg, #173f36 8%, #52ffc5 58%, #e5dbbc 100%)",
            boxShadow: "0 1px 2px rgb(0 0 0 / 12%)",
            transform: mode === "confidential" ? "translateX(calc(3.5rem - 1.5rem - 6px))" : "translateX(0)",
          }}
        />
      </button>
      <button
        type="button"
        onClick={() => onChange("confidential")}
        className="border-0 bg-transparent p-0 cursor-pointer"
      >
        <span className={`text-sm leading-5 tracking-[-0.01em] ${mode === "confidential" ? "text-[#192b25] font-semibold" : "text-zinc-400 font-normal"}`}>
          Confidential x402
        </span>
      </button>
    </div>
  );
}
