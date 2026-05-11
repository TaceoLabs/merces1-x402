import { getDefaultConfig } from "@rainbow-me/rainbowkit";
import { metaMaskWallet } from "@rainbow-me/rainbowkit/wallets";
import { baseSepolia, foundry } from "viem/chains";

export const config = getDefaultConfig({
  appName: "Confidential x402 Demo",
  appDescription: "Demo of the confidential x402 payment scheme by Merces / TACEO",
  appUrl: process.env.NEXT_PUBLIC_APP_URL ?? "",
  projectId: process.env.NEXT_PUBLIC_WC_PROJECT_ID ?? "demo",
  chains: [foundry, baseSepolia],
  ssr: true,
  wallets: [
    {
      groupName: "Wallets",
      wallets: [metaMaskWallet],
    },
  ],
});
