import { ConnectButton } from "@rainbow-me/rainbowkit";

export default function WalletButton() {
  return (
    <ConnectButton.Custom>
      {({ account, chain, openConnectModal, openAccountModal, mounted }) => {
        if (!mounted) return null;
        if (!account || !chain) {
          return (
            <button
              onClick={openConnectModal}
              className="inline-flex items-center justify-center h-9 px-4 rounded-[9999px] bg-zinc-900 text-sm font-semibold text-white hover:bg-zinc-700 transition-colors cursor-pointer border-0"
            >
              Connect wallet
            </button>
          );
        }
        return (
          <button
            onClick={openAccountModal}
            className="inline-flex items-center gap-2 h-9 px-3 pr-4 rounded-full border border-zinc-200 bg-[#f4f4f5] text-sm font-semibold text-zinc-800 cursor-pointer hover:bg-zinc-200 transition-colors"
          >
            <span style={{ height: "1.5rem", width: "1.5rem", borderRadius: "9999px", background: "radial-gradient(120% 95% at 24% 22%, #255b4d 0%, transparent 56%), radial-gradient(95% 95% at 70% 86%, #62ffd1 0%, transparent 62%), linear-gradient(145deg, #173f36 8%, #52ffc5 58%, #e5dbbc 100%)", display: "inline-block", flexShrink: 0 }} />
            {account.address.slice(0, 4)}…{account.address.slice(-4)}
          </button>
        );
      }}
    </ConnectButton.Custom>
  );
}
