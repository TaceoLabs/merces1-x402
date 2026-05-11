import { x402Client, wrapFetchWithPayment, x402HTTPClient } from "@x402/fetch";
import { ConfidentialEvmScheme } from "../src/client/scheme";
import { privateKeyToAccount } from "viem/accounts";

const privateKey = process.env.PRIVATE_KEY as `0x${string}`;
const serverUrl = process.env.SERVER_URL;

async function main(): Promise<void> {
  const signer = privateKeyToAccount(privateKey);

  const client = new x402Client();
  client.register("eip155:*", new ConfidentialEvmScheme(signer));

  const fetchWithPayment = wrapFetchWithPayment(fetch, client);

  const response = await fetchWithPayment(`${serverUrl}/api/protected`, { method: "GET" });
  console.log("Response status:", response.status);
  console.log("Response body:", await response.text());

  if (response.ok) {
    const paymentResponse = new x402HTTPClient(client).getPaymentSettleResponse(name =>
      response.headers.get(name),
    );
    console.log("\nPayment response:", JSON.stringify(paymentResponse, null, 2));
  } else {
    console.log(`\nNo payment settled (response status: ${response.status})`);
  }
}

main().catch(error => {
  console.error(error?.response?.data?.error ?? error);
  process.exit(1);
}).finally(() => {
  process.exit(0);
});
