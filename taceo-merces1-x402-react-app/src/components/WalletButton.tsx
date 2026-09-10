import { ScanLine, Wallet } from "lucide-react";
import { useState, type ReactNode } from "react";
import { type Connector, useAccount, useConnect, useConnectors, useDisconnect } from "wagmi";

function truncateAddress(address: string) {
  return `${address.slice(0, 4)}…${address.slice(-4)}`;
}

function ConnectorOptionRow({
  name,
  icon,
  onClick,
}: {
  name: string;
  icon: ReactNode;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="flex w-full items-center gap-3 rounded-lg px-3 py-2.5 text-left text-sm font-medium text-zinc-800 transition-colors hover:bg-zinc-50 cursor-pointer border-0 bg-transparent"
    >
      {icon}
      {name}
    </button>
  );
}

export default function WalletButton() {
  const { address, isConnected } = useAccount();
  const { connect } = useConnect();
  const { disconnect } = useDisconnect();
  const connectors = useConnectors();
  const injectedConnectors = connectors.filter((connector) => connector.type === "injected");
  const walletConnectConnector = connectors.find((connector) => connector.type === "walletConnect");
  const [isPickerOpen, setIsPickerOpen] = useState(false);
  const [isAccountMenuOpen, setIsAccountMenuOpen] = useState(false);

  const handleSelectConnector = (connector: Connector | undefined) => {
    if (!connector) return;
    setIsPickerOpen(false);
    connect({ connector });
  };

  if (!isConnected || !address) {
    return (
      <>
        <button
          onClick={() => setIsPickerOpen(true)}
          className="inline-flex items-center justify-center h-9 px-4 rounded-[9999px] bg-zinc-900 text-sm font-semibold text-white hover:bg-zinc-700 transition-colors cursor-pointer border-0"
        >
          Connect wallet
        </button>
        {isPickerOpen && (
          <div
            className="fixed inset-0 z-50 flex items-center justify-center p-4"
            onClick={() => setIsPickerOpen(false)}
          >
            <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" />
            <div
              className="relative z-10 w-full max-w-xs rounded-xl border border-zinc-200 bg-white shadow-xl"
              onClick={(e) => e.stopPropagation()}
            >
              <div className="px-5 py-4 border-b border-zinc-100">
                <p className="text-sm font-semibold text-zinc-800">Connect a Wallet</p>
              </div>
              <div className="flex flex-col gap-1 p-2">
                {injectedConnectors.map((connector) => (
                  <ConnectorOptionRow
                    key={connector.uid}
                    name={connector.name}
                    icon={
                      connector.icon ? (
                        // eslint-disable-next-line @next/next/no-img-element
                        <img src={connector.icon} alt="" className="size-8 rounded-lg object-cover" />
                      ) : (
                        <span className="flex size-8 items-center justify-center rounded-lg bg-zinc-100">
                          <Wallet className="size-4 text-zinc-500" />
                        </span>
                      )
                    }
                    onClick={() => handleSelectConnector(connector)}
                  />
                ))}
                {walletConnectConnector && (
                  <ConnectorOptionRow
                    key={walletConnectConnector.uid}
                    name="WalletConnect"
                    icon={
                      <span className="flex size-8 items-center justify-center rounded-lg bg-[#3396FF]">
                        <ScanLine className="size-4 text-white" />
                      </span>
                    }
                    onClick={() => handleSelectConnector(walletConnectConnector)}
                  />
                )}
              </div>
            </div>
          </div>
        )}
      </>
    );
  }

  return (
    <>
      <button
        onClick={() => setIsAccountMenuOpen(true)}
        className="inline-flex items-center gap-2 h-9 px-3 pr-4 rounded-full border border-zinc-200 bg-[#f4f4f5] text-sm font-semibold text-zinc-800 cursor-pointer hover:bg-zinc-200 transition-colors"
      >
        <span
          style={{
            height: "1.5rem",
            width: "1.5rem",
            borderRadius: "9999px",
            background:
              "radial-gradient(120% 95% at 24% 22%, #255b4d 0%, transparent 56%), radial-gradient(95% 95% at 70% 86%, #62ffd1 0%, transparent 62%), linear-gradient(145deg, #173f36 8%, #52ffc5 58%, #e5dbbc 100%)",
            display: "inline-block",
            flexShrink: 0,
          }}
        />
        {truncateAddress(address)}
      </button>
      {isAccountMenuOpen && (
        <div
          className="fixed inset-0 z-50 flex items-center justify-center p-4"
          onClick={() => setIsAccountMenuOpen(false)}
        >
          <div className="absolute inset-0 bg-black/40 backdrop-blur-sm" />
          <div
            className="relative z-10 w-full max-w-xs rounded-xl border border-zinc-200 bg-white shadow-xl"
            onClick={(e) => e.stopPropagation()}
          >
            <div className="px-5 py-4 border-b border-zinc-100">
              <p className="text-sm font-semibold text-zinc-800">{truncateAddress(address)}</p>
            </div>
            <div className="p-2">
              <button
                onClick={() => {
                  setIsAccountMenuOpen(false);
                  disconnect();
                }}
                className="flex w-full items-center rounded-lg px-3 py-2.5 text-left text-sm font-medium text-red-600 transition-colors hover:bg-red-50 cursor-pointer border-0 bg-transparent"
              >
                Disconnect
              </button>
            </div>
          </div>
        </div>
      )}
    </>
  );
}
