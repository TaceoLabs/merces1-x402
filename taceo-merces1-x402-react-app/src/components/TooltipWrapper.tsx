import { useState, useRef, useCallback } from "react";

interface TooltipWrapperProps {
  text: string;
  show: boolean;
  children: React.ReactNode;
}

export default function TooltipWrapper({ text, show, children }: TooltipWrapperProps) {
  const [touched, setTouched] = useState(false);
  const touchTimer = useRef<ReturnType<typeof setTimeout> | null>(null);

  const handleTouchStart = useCallback(() => {
    if (!show) return;
    setTouched(true);
    if (touchTimer.current) clearTimeout(touchTimer.current);
    touchTimer.current = setTimeout(() => setTouched(false), 2500);
  }, [show]);

  const handleTouchEnd = useCallback(() => {
    // keep tooltip visible via the timer; don't clear immediately
  }, []);

  if (!show) return <>{children}</>;

  return (
    <span
      className="relative inline-flex group"
      onTouchStart={handleTouchStart}
      onTouchEnd={handleTouchEnd}
    >
      {children}
      <span
        role="tooltip"
        className={[
          "pointer-events-none absolute bottom-full left-1/2 -translate-x-1/2 mb-2",
          "whitespace-nowrap rounded-md bg-zinc-800 px-2.5 py-1.5 text-xs text-white shadow-md",
          "transition-opacity duration-150",
          touched ? "opacity-100" : "opacity-0 group-hover:opacity-100",
        ].join(" ")}
      >
        {text}
        <span className="absolute top-full left-1/2 -translate-x-1/2 border-4 border-transparent border-t-zinc-800" />
      </span>
    </span>
  );
}
