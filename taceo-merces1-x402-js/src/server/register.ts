import { x402ResourceServer } from "@x402/core/server";
import { Network } from "@x402/core/types";
import { ConfidentialEvmScheme, ConfidentialServerSchemeConfig } from "./scheme";

/**
 * Configuration for registering the confidential EVM scheme on a resource server.
 */
export interface ConfidentialEvmResourceServerConfig extends ConfidentialServerSchemeConfig {
  /** Optional specific networks to register. Defaults to eip155:* wildcard. */
  networks?: Network[];
}

/**
 * Registers the confidential EVM scheme to an x402ResourceServer instance.
 */
export function registerConfidentialEvmScheme(
  server: x402ResourceServer,
  config: ConfidentialEvmResourceServerConfig,
): x402ResourceServer {
  const scheme = new ConfidentialEvmScheme(config);

  if (config.networks && config.networks.length > 0) {
    config.networks.forEach(network => {
      server.register(network, scheme);
    });
  } else {
    server.register("eip155:*", scheme);
  }

  return server;
}
