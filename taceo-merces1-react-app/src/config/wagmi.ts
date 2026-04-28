import { getDefaultConfig } from "@rainbow-me/rainbowkit";
import { foundry } from "viem/chains";

export const config = getDefaultConfig({
  appName: "Merces1 Demo",
  appDescription: "Merces1 Demo",
  appUrl: process.env.NEXT_PUBLIC_APP_URL ?? "",
  projectId: process.env.NEXT_PUBLIC_WC_PROJECT_ID ?? "",
  chains: [foundry],
  ssr: true,
});
