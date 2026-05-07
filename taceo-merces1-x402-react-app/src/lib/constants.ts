export const NODE_URLS = ["/api/node0", "/api/node1", "/api/node2"];
export const X402_SERVER_URL = "/api/x402-server";
export const FAUCET_URL = "/api/faucet";
export const CHAIN_ID = Number(process.env.NEXT_PUBLIC_CHAIN_ID!);
export const BN254_PRIME = BigInt(
  "0x30644e72e131a029b85045b68181585d2833e84879b9709143e1f593f0000001",
);
export const X402_SERVER_ADDRESS = process.env.NEXT_PUBLIC_X402_SERVER_ADDRESS;
export const BLOCK_EXPLORER_URL = process.env.NEXT_PUBLIC_BLOCK_EXPLORER_URL;
