import express from "express";
import { paymentMiddleware, x402ResourceServer } from "@x402/express";
import { HTTPFacilitatorClient } from "@x402/core/server";
import { ConfidentialEvmScheme } from "../src/server/scheme";

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
            network: "eip155:31337",
            payTo: address,
          },
        ]
      },
    },
    new x402ResourceServer(facilitatorClient)
      .register("eip155:31337", new ConfidentialEvmScheme({ asset: "0x5FC8d32690cc91D4c39d9d3abcBD16989F875707" }))
  ),
);

app.get("/api/protected", (req, res) => {
  res.send("protected content");
});

app.listen(8080, () => {
  console.log(`Server listening at http://localhost:${8080}`);
});
