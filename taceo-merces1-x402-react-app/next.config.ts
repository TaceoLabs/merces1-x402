import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  reactStrictMode: true,
  transpilePackages: [
    "@taceolabs/taceo-merces1-x402-js",
    "@taceolabs/taceo-merces1-client-js",
  ],
  async rewrites() {
    return [
      {
        source: "/api/node0/:path*",
        destination: `${process.env.NODE0_URL}/:path*`,
      },
      {
        source: "/api/node1/:path*",
        destination: `${process.env.NODE1_URL}/:path*`,
      },
      {
        source: "/api/node2/:path*",
        destination: `${process.env.NODE2_URL}/:path*`,
      },
      {
        source: "/api/x402-server/:path*",
        destination: `${process.env.X402_SERVER_URL}/:path*`,
      },
      {
        source: "/api/faucet/:path*",
        destination: `${process.env.FAUCET_URL}/:path*`,
      },
    ];
  },
  webpack: (config) => {
    config.resolve ??= {};
    // wagmi/connectors is a single barrel export, so importing `walletConnect` from it still
    // pulls in the other connectors' modules (metaMask, baseAccount, coinbaseWallet, safe, ...)
    // at bundle time. We only use the WalletConnect connector, so alias away the wallet SDKs
    // those other connectors depend on but that aren't installed as full dependencies here.
    config.resolve.alias = {
      ...(config.resolve.alias ?? {}),
      "@react-native-async-storage/async-storage": false,
      "@base-org/account": false,
      "@coinbase/wallet-sdk": false,
      "@metamask/sdk": false,
      "@safe-global/safe-apps-sdk": false,
      "@safe-global/safe-apps-provider": false,
      porto: false,
      "porto/internal": false,
    };
    return config;
  },
};

export default nextConfig;
