import { useState } from "react";
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

const Logo = () => (
  <>
    <svg xmlns="http://www.w3.org/2000/svg" width="36" height="36" fill="none" className="shrink-0">
      <rect width="36" height="36" fill="#2C5749" rx="8" />
      <path fill="#52FFC5" d="m23 8 4.825 4.825L12.61 28.041l-4.826-4.825z" />
    </svg>
    <div className="flex flex-col leading-tight whitespace-nowrap">
      <span className="text-sm font-semibold tracking-tight text-zinc-900">Merces by TACEO</span>
      <span className="text-xs text-zinc-500">Confidential x402</span>
    </div>
  </>
);

export default function Sidebar() {
  const { pathname } = useRouter();
  const [collapsed, setCollapsed] = useState(false);
  const [mobileOpen, setMobileOpen] = useState(false);

  return (
    <>
      {/* Mobile top bar */}
      <div className="md:hidden sticky top-0 z-30 flex items-center justify-between px-4 py-3 border-b border-zinc-200 bg-[#f9f8f5]">
        <a href="/" className="flex items-center gap-3">
          <Logo />
        </a>
        <button
          onClick={() => setMobileOpen(true)}
          className="flex items-center justify-center w-9 h-9 rounded-md text-zinc-500 hover:text-zinc-900 hover:bg-zinc-200 transition-colors border-0 bg-transparent cursor-pointer"
          aria-label="Open menu"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
            <line x1="3" y1="6" x2="21" y2="6" />
            <line x1="3" y1="12" x2="21" y2="12" />
            <line x1="3" y1="18" x2="21" y2="18" />
          </svg>
        </button>
      </div>

      {/* Mobile full-screen dropdown */}
      {mobileOpen && (
        <div className="md:hidden fixed inset-0 z-50 flex flex-col bg-[#f9f8f5]">
          {/* Header */}
          <div className="flex items-center justify-between px-4 py-3 border-b border-zinc-200">
            <a href="/" className="flex items-center gap-3" onClick={() => setMobileOpen(false)}>
              <Logo />
            </a>
            <button
              onClick={() => setMobileOpen(false)}
              className="flex items-center justify-center w-9 h-9 rounded-md text-zinc-500 hover:text-zinc-900 hover:bg-zinc-200 transition-colors border-0 bg-transparent cursor-pointer"
              aria-label="Close menu"
            >
              <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <polyline points="18 15 12 9 6 15" />
              </svg>
            </button>
          </div>

          {/* Nav items */}
          <nav className="flex-1 px-4 py-4 flex flex-col gap-1 overflow-y-auto">
            {NAV.map(({ href, label, icon }, i) => {
              const isExternal = href.startsWith("http");
              const isActive = !isExternal && pathname === href;
              const prevIsExternal = i > 0 && NAV[i - 1].href.startsWith("http");
              const showDivider = isExternal && !prevIsExternal;
              return (
                <div key={href}>
                  {showDivider && <hr className="my-3 border-zinc-200" />}
                  <a
                    href={href}
                    {...(isExternal ? { target: "_blank", rel: "noopener noreferrer" } : {})}
                    onClick={() => setMobileOpen(false)}
                    className={`flex items-center gap-4 rounded-xl text-base font-medium px-4 py-3.5 transition-colors ${
                      isActive ? "bg-[#a7f3d0] text-zinc-900" : "text-zinc-700 hover:bg-[#a7f3d0] hover:text-zinc-900"
                    }`}
                  >
                    {icon}
                    <span>{label}</span>
                    {isExternal && (
                      <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="ml-auto opacity-40" aria-hidden="true">
                        <path d="M18 13v6a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2h6" />
                        <polyline points="15 3 21 3 21 9" />
                        <line x1="10" y1="14" x2="21" y2="3" />
                      </svg>
                    )}
                  </a>
                </div>
              );
            })}
          </nav>
        </div>
      )}

      {/* Desktop sidebar */}
      <aside
        className={`hidden md:flex ${collapsed ? "w-14" : "w-64"} shrink-0 sticky top-0 h-screen flex-col border-r border-zinc-200 bg-[#f9f8f5] transition-[width] duration-200 overflow-hidden`}
      >
        {/* Logo */}
        <div className="pt-5 pb-3 px-2 flex items-center justify-between">
          {!collapsed && (
            <a
              href="/"
              className="flex items-center gap-3 rounded-lg hover:bg-[#e5fff6] transition-colors px-2 py-2"
            >
              <Logo />
            </a>
          )}
          <button
            onClick={() => setCollapsed((c) => !c)}
            className="ml-auto flex items-center justify-center w-9 h-9 rounded-md text-zinc-400 hover:text-zinc-700 hover:bg-zinc-200 transition-colors cursor-pointer border-0 bg-transparent"
            title={collapsed ? "Expand sidebar" : "Collapse sidebar"}
          >
            {collapsed ? (
              <svg xmlns="http://www.w3.org/2000/svg" width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <line x1="3" y1="6" x2="21" y2="6" />
                <line x1="3" y1="12" x2="21" y2="12" />
                <line x1="3" y1="18" x2="21" y2="18" />
              </svg>
            ) : (
              <svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <polyline points="15 18 9 12 15 6" />
              </svg>
            )}
          </button>
        </div>

        {/* Nav */}
        <nav className="flex-1 px-2 py-2 flex flex-col gap-0.5 overflow-y-auto">
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
                  title={collapsed ? label : undefined}
                  className={`flex items-center gap-3 rounded-lg text-sm font-medium transition-colors ${
                    collapsed ? "justify-center px-0 py-2.5" : "px-3 py-2.5"
                  } ${isActive ? "bg-[#a7f3d0]" : "hover:bg-[#a7f3d0]"}`}
                >
                  {icon}
                  {!collapsed && <span className="whitespace-nowrap">{label}</span>}
                </a>
              </div>
            );
          })}
        </nav>
      </aside>
    </>
  );
}
