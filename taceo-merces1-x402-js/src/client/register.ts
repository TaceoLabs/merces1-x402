import { x402Client } from "@x402/core/client";
import { Network } from "@x402/core/types";
import { ClientEvmSigner } from "@x402/evm";
import { ConfidentialEvmScheme } from "./scheme";

/**
 * Configuration for registering the confidential EVM scheme on a client.
 */
export interface ConfidentialEvmClientConfig {
  /** The EVM signer for signing payment authorizations */
  signer: ClientEvmSigner;
  /** Optional specific networks to register. Defaults to eip155:* wildcard. */
  networks?: Network[];
}

/**
 * Registers the confidential EVM scheme to an x402Client instance.
 */
export function registerConfidentialEvmScheme(
  client: x402Client,
  config: ConfidentialEvmClientConfig,
): x402Client {
  const scheme = new ConfidentialEvmScheme(config.signer);

  if (config.networks && config.networks.length > 0) {
    config.networks.forEach(network => {
      client.register(network, scheme);
    });
  } else {
    client.register("eip155:*", scheme);
  }

  return client;
}
