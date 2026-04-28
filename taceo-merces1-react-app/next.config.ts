import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  reactStrictMode: true,
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
        source: "/api/rpc",
        destination: `${process.env.RPC_URL}`,
      },
    ];
  },
};

export default nextConfig;
