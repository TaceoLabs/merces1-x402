import { getDefaultConfig } from "@rainbow-me/rainbowkit";
import { metaMaskWallet } from "@rainbow-me/rainbowkit/wallets";
import { baseSepolia, foundry, mainnet } from "viem/chains";

const isProduction = process.env.NODE_ENV === "production";

export const config = getDefaultConfig({
  appName: "Confidential x402 Demo",
  appDescription: "Demo of the confidential x402 payment scheme by Merces / TACEO",
  appUrl: process.env.NEXT_PUBLIC_APP_URL ?? "",
  projectId: process.env.NEXT_PUBLIC_WC_PROJECT_ID ?? "demo",
  // Need to include mainnet for the metamask app QR code to work, even though
  // the app itself doesn't interact with mainnet.
  chains: [isProduction ? mainnet : foundry, baseSepolia],
  ssr: true,
  wallets: [
    {
      groupName: "Wallets",
      wallets: [metaMaskWallet],
    },
  ],
});
