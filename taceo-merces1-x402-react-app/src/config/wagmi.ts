import { baseSepolia, foundry, mainnet } from "viem/chains";
import { createConfig, http } from "wagmi";
import { walletConnect } from "wagmi/connectors";

const isProduction = process.env.NODE_ENV === "production";

export const config = createConfig({
  // Need to include mainnet for the WalletConnect QR code to work, even though
  // the app itself doesn't interact with mainnet.
  chains: [isProduction ? mainnet : foundry, baseSepolia],
  connectors: [
    walletConnect({
      projectId: process.env.NEXT_PUBLIC_WC_PROJECT_ID ?? "demo",
      showQrModal: true,
    }),
  ],
  ssr: true,
  transports: {
    [mainnet.id]: http(),
    [foundry.id]: http(),
    [baseSepolia.id]: http(),
  },
});
