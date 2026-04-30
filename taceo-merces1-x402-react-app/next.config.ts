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
};

export default nextConfig;
