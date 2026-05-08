import Footer from "@/components/Footer";
import Sidebar from "@/components/Sidebar";

const LINKS = [
  {
    href: "/intro",
    label: "Introduction",
    description: "Understand the confidential x402 protocol — how payments settle on-chain while amounts stay hidden.",
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
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
    description: "Connect a wallet, claim testnet USDC from the faucet, and pay for access to a protected resource.",
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <path d="M21 12V7H5a2 2 0 0 1 0-4h14v4" />
        <path d="M3 5v14a2 2 0 0 0 2 2h16v-5" />
        <path d="M18 12a2 2 0 0 0 0 4h4v-4Z" />
      </svg>
    ),
  },
  {
    href: "/server",
    label: "Resource Server",
    description: "See the server's payment history, revenue stats, and pricing tier breakdown — all reconstructed from MPC shares.",
    icon: (
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.75" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
        <rect width="20" height="8" x="2" y="2" rx="2" />
        <rect width="20" height="8" x="2" y="14" rx="2" />
        <line x1="6" y1="6" x2="6.01" y2="6" />
        <line x1="6" y1="18" x2="6.01" y2="18" />
      </svg>
    ),
  },
];

export default function Home() {
  return (
    <div className="flex flex-col md:flex-row min-h-screen text-zinc-900 font-sans antialiased">
      <Sidebar />
      <div className="flex-1 flex flex-col min-w-0">
        <main className="flex-1 flex flex-col justify-center px-6 py-20 gap-12">

          {/* Hero */}
          <div className="text-center max-w-xl mx-auto">
            <h1 className="text-4xl font-semibold tracking-tight text-zinc-900 mb-4">
              Confidential x402
            </h1>
            <div className="inline-flex items-center gap-2 text-xs font-medium text-[#192b25] bg-[#e5fff6] border border-[#a7f3d0] px-3 py-1 rounded-full mb-6">
              <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
                <path d="M17.94 17.94A10.07 10.07 0 0 1 12 20c-7 0-11-8-11-8a18.45 18.45 0 0 1 5.06-5.94M9.9 4.24A9.12 9.12 0 0 1 12 4c7 0 11 8 11 8a18.5 18.5 0 0 1-2.16 3.19m-6.72-1.07a3 3 0 1 1-4.24-4.24" />
                <line x1="1" y1="1" x2="23" y2="23" />
              </svg>
              Private agentic payments
            </div>
            <p className="text-base text-zinc-500 leading-relaxed">
              Merces extends the x402 HTTP payment protocol with MPC-based confidential transfers — payments settle on-chain, but amounts stay hidden from everyone except the parties involved.
            </p>
          </div>

          {/* Cards */}
          <div className="grid grid-cols-1 sm:grid-cols-3 gap-4 w-full max-w-3xl mx-auto">
            {LINKS.map(({ href, label, description, icon }) => (
              <a
                key={href}
                href={href}
                className="group rounded-xl border border-zinc-200 bg-white p-5 flex flex-col gap-4 hover:border-[#a7f3d0] hover:shadow-[0_2px_12px_rgba(82,255,197,0.12)] transition-all"
              >
                <div className="w-9 h-9 rounded-lg bg-[#e5fff6] flex items-center justify-center text-[#192b25]">
                  {icon}
                </div>
                <div>
                  <div className="text-sm font-semibold text-zinc-900 mb-1 group-hover:text-[#192b25] transition-colors">{label}</div>
                  <p className="text-xs text-zinc-400 leading-relaxed">{description}</p>
                </div>
                <div className="mt-auto flex items-center gap-1 text-xs font-medium text-[#192b25] opacity-0 group-hover:opacity-100 transition-opacity">
                  Open
                  <svg xmlns="http://www.w3.org/2000/svg" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.5" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><polyline points="9 18 15 12 9 6" /></svg>
                </div>
              </a>
            ))}
          </div>

        </main>
        <Footer />
      </div>
    </div>
  );
}
