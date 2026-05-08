import { useRouter } from "next/router";

const NAV = [
  {
    href: "/",
    label: "Overview",
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="m3 9 9-7 9 7v11a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2z" />
        <polyline points="9 22 9 12 15 12 15 22" />
      </svg>
    ),
  },
  {
    href: "/intro",
    label: "Introduction",
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M14.5 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7.5L14.5 2z" />
        <polyline points="14 2 14 8 20 8" />
        <line x1="16" y1="13" x2="8" y2="13" />
        <line x1="16" y1="17" x2="8" y2="17" />
        <line x1="10" y1="9" x2="8" y2="9" />
      </svg>
    ),
  },
  {
    href: "/client",
    label: "Client",
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M21 12V7H5a2 2 0 0 1 0-4h14v4" />
        <path d="M3 5v14a2 2 0 0 0 2 2h16v-5" />
        <path d="M18 12a2 2 0 0 0 0 4h4v-4Z" />
      </svg>
    ),
  },
  {
    href: "/server",
    label: "Resource Server",
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <rect width="20" height="8" x="2" y="2" rx="2" />
        <rect width="20" height="8" x="2" y="14" rx="2" />
        <line x1="6" y1="6" x2="6.01" y2="6" />
        <line x1="6" y1="18" x2="6.01" y2="18" />
      </svg>
    ),
  },
  {
    href: "https://core.taceo.io",
    label: "Docs",
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M2 3h6a4 4 0 0 1 4 4v14a3 3 0 0 0-3-3H2z" />
        <path d="M22 3h-6a4 4 0 0 0-4 4v14a3 3 0 0 1 3-3h7z" />
      </svg>
    ),
  },
];

export default function Sidebar() {
  const { pathname } = useRouter();

  return (
    <aside className="w-56 shrink-0 sticky top-0 h-screen flex flex-col border-r border-zinc-200 bg-[#f9f8f5]">
      {/* Logo */}
      <div className="px-3 pt-5 pb-3">
        <a
          href="/"
          className="flex items-center gap-3 px-2 py-2 rounded-lg hover:bg-[#e5fff6] transition-colors"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="36" height="36" fill="none" className="shrink-0">
            <rect width="36" height="36" fill="#2C5749" rx="8" />
            <path fill="#52FFC5" d="m23 8 4.825 4.825L12.61 28.041l-4.826-4.825z" />
          </svg>
          <div className="flex flex-col leading-tight">
            <span className="text-sm font-semibold tracking-tight text-zinc-900">Merces by TACEO</span>
            <span className="text-xs text-zinc-500">Confidential x402</span>
          </div>
        </a>
      </div>

      {/* Nav */}
      <nav className="flex-1 px-3 py-2 flex flex-col gap-0.5 overflow-y-auto">
        {NAV.map(({ href, label, icon }, i) => {
          const isExternal = href.startsWith("http");
          const isActive = !isExternal && pathname === href;
          const prevIsExternal = i > 0 && NAV[i - 1].href.startsWith("http");
          const showDivider = isExternal && !prevIsExternal;
          return (
            <div key={href}>
              {showDivider && <hr className="my-2 border-zinc-200" />}
              <a
                href={href}
                {...(isExternal ? { target: "_blank", rel: "noopener noreferrer" } : {})}
                className={`flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-medium transition-colors ${
                  isActive
                    ? "bg-[#a7f3d0]"
                    : "hover:bg-[#a7f3d0]"
                }`}
              >
                {icon}
                {label}
              </a>
            </div>
          );
        })}
      </nav>
    </aside>
  );
}
