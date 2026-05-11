# Confidential x402

> Confidential x402 payment scheme for JavaScript/TypeScript — extends the [x402 protocol](https://www.x402.org) with privacy-preserving payment verification using MPC and ZK proofs.

`@taceo/confidential-x402` provides a custom x402 scheme (`ConfidentialEvmScheme`) that integrates with [`x402`](https://github.com/x402-foundation/x402/tree/main).

## Installation

```sh
npm install @taceo/confidential-x402
```

## Quick Start

### Protect Routes (Server)

Use `ConfidentialEvmScheme` with `@x402/express` to gate routes behind confidential on-chain payments:

```ts
import express from "express";
import { paymentMiddleware, x402ResourceServer } from "@x402/express";
import { HTTPFacilitatorClient } from "@x402/core/server";
import { ConfidentialEvmScheme } from "@taceo/confidential-x402/server";

const address = process.env.ADDRESS as `0x${string}`;
const facilitatorUrl = process.env.FACILITATOR_URL;
const facilitatorClient = new HTTPFacilitatorClient({ url: facilitatorUrl });

const app = express();

app.use(
  paymentMiddleware(
    {
      "GET /api/protected": {
        accepts: [
          {
            scheme: "confidential",
            price: "$1",
            network: "eip155:84532",
            payTo: address,
          },
        ]
      },
    },
    new x402ResourceServer(facilitatorClient)
      .register("eip155:84532", new ConfidentialEvmScheme({ asset: "0x4Ee80fFA1332525A8Cd100E1edf72Fe066f01c10" }))
  ),
);

app.get("/api/protected", (req, res) => {
  res.send("protected content");
});

app.listen(8080)
```

### Send Payments (Client)

Use `ConfidentialEvmScheme` with `@x402/fetch` to automatically handle confidential payments:

```ts
import { x402Client, wrapFetchWithPayment } from "@x402/fetch";
import { ConfidentialEvmScheme } from "@taceo/confidential-x402/client";
import { privateKeyToAccount } from "viem/accounts";

const privateKey = process.env.PRIVATE_KEY as `0x${string}`;
const serverUrl = process.env.SERVER_URL;

const signer = privateKeyToAccount(privateKey);

const client = new x402Client();
client.register("eip155:*", new ConfidentialEvmScheme(signer));

const fetchWithPayment = wrapFetchWithPayment(fetch, client);

const response = await fetchWithPayment(`${serverUrl}/api/protected`, { method: "GET" });
console.log("Response status:", response.status);
console.log("Response body:", await response.text());
```
