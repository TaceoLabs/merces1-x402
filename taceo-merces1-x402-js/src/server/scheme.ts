import {
  AssetAmount,
  Network,
  PaymentRequirements,
  Price,
  SchemeNetworkServer,
} from "@x402/core/types";
import { ConfidentialExtra } from "../types";

/**
 * EVM server implementation for the Confidential payment scheme.
 *
 * Parses prices and enhances payment requirements with confidential-specific
 * extra data (contract address, EIP-712 domain, MPC public keys).
 */
export class ConfidentialEvmScheme implements SchemeNetworkServer {
  readonly scheme = "confidential";

  constructor(private readonly config: ConfidentialServerSchemeConfig) { }

  /**
   * Parse price into asset amount.
   * If already an AssetAmount, returns directly.
   * If a number/string, converts to 6-decimal USDC units.
   */
  async parsePrice(price: Price, _network: Network): Promise<AssetAmount> {
    if (typeof price === "object" && price !== null && "amount" in price) {
      if (!price.asset) {
        throw new Error("Asset address must be specified for AssetAmount");
      }
      return {
        amount: price.amount,
        asset: price.asset,
        extra: price.extra || {},
      };
    }

    // Parse money to decimal
    const amount = typeof price === "number" ? price : parseFloat(String(price).replace(/^\$/, "").trim());
    if (isNaN(amount)) {
      throw new Error(`Invalid money format: ${price}`);
    }

    // Convert to 6-decimal token units (USDC)
    const [intPart, decPart = ""] = String(amount).split(".");
    const paddedDec = decPart.padEnd(6, "0").slice(0, 6);
    const tokenAmount = (intPart + paddedDec).replace(/^0+/, "") || "0";

    return {
      amount: tokenAmount,
      asset: this.config.asset,
      extra: {},
    };
  }

  /**
   * Enhance payment requirements by merging the facilitator's extra data
   * (confidentialToken, eip712Domain, mpcPublicKeys) into the requirements.
   */
  enhancePaymentRequirements(
    paymentRequirements: PaymentRequirements,
    supportedKind: {
      x402Version: number;
      scheme: string;
      network: Network;
      extra?: Record<string, unknown>;
    },
    _extensionKeys: string[],
  ): Promise<PaymentRequirements> {
    // Merge extra from facilitator into requirements
    const facilitatorExtra = supportedKind.extra as Partial<ConfidentialExtra> | undefined;

    return Promise.resolve({
      ...paymentRequirements,
      extra: {
        ...paymentRequirements.extra,
        ...facilitatorExtra,
      },
    });
  }
}

/**
 * Configuration for the confidential server scheme.
 */
export interface ConfidentialServerSchemeConfig {
  /** The underlying ERC-20 asset address (e.g., USDC) */
  asset: string;
}
