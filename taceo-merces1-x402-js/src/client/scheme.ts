import {
  PaymentRequirements,
  SchemeNetworkClient,
  PaymentPayloadResult,
  PaymentPayloadContext,
} from "@x402/core/types";
import { ClientEvmSigner } from "@x402/evm";
import { ConfidentialEvmPayload, ConfidentialExtra } from "../types";
import { encodeAbiParameters, getAddress, keccak256, toHex } from "viem";
import * as snarkjs from 'snarkjs';
import { encodeCiphertexts, fetchWitnessWasm, fetchZkey, prepareTransfer } from "@taceo/merces1-client";

/**
 * Get the crypto object from the global scope.
 *
 * @returns The crypto object
 * @throws Error if crypto API is not available
 */
function getCrypto(): Crypto {
  const cryptoObj = globalThis.crypto as Crypto | undefined;
  if (!cryptoObj) {
    throw new Error("Crypto API not available");
  }
  return cryptoObj;
}

/**
 * Create a random 32-byte nonce for EIP-3009 authorization.
 *
 * @returns A 32-byte nonce
 */
export function createNonce(): bigint {
  return BigInt(toHex(getCrypto().getRandomValues(new Uint8Array(32))));
}


/**
 * EVM client implementation for the Confidential payment scheme.
 *
 * Creates payment payloads with Poseidon2 commitments, secret-shared ciphertexts,
 * and EIP-712 signed transferFrom authorizations.
 */
export class ConfidentialEvmScheme implements SchemeNetworkClient {
  readonly scheme = "confidential";

  constructor(
    private readonly signer: ClientEvmSigner,
  ) { }

  async createPaymentPayload(
    x402Version: number,
    paymentRequirements: PaymentRequirements,
    _context?: PaymentPayloadContext,
  ): Promise<PaymentPayloadResult> {
    const extra = paymentRequirements.extra as unknown as ConfidentialExtra;
    if (!extra?.confidentialToken || !extra?.eip712Domain) {
      throw new Error(
        "Payment requirements missing confidential extra (confidentialToken, eip712Domain)",
      );
    }

    const amount = BigInt(paymentRequirements.amount);
    const receiver = getAddress(paymentRequirements.payTo);
    const chainId = parseInt(paymentRequirements.network.split(":")[1]!);
    const confidentialToken = getAddress(extra.confidentialToken) as `0x${string}`;
    const mpcPublicKeys = extra.mpcPks.map(pk => ({ x: BigInt(pk[0]), y: BigInt(pk[1]) }));

    // ZK proof path: circuit computes commitment + encrypted shares + compressed proof
    const witnessWasm = await fetchWitnessWasm();
    const zkey = await fetchZkey();
    const { inputs, ciphertexts, senderPk, amountCommitment, amountR } = prepareTransfer(amount, mpcPublicKeys);
    let proof: snarkjs.Groth16Proof, publicSignals: snarkjs.PublicSignals;
    try {
      ({ proof, publicSignals } = await snarkjs.groth16.fullProve(inputs, witnessWasm, zkey));
    } catch (e) {
      throw new Error(`Proof or witness generation failed: ${e}`);
    }
    const beta = publicSignals[0]!;

    // 2. Generate random nonce and compute deadline
    const nonce = createNonce();
    const now = Math.floor(Date.now() / 1000);
    const deadline = BigInt(now + paymentRequirements.maxTimeoutSeconds);

    // 3. Compute ciphertext hash for EIP-712 signing
    const encodedCiphertexts = encodeCiphertexts(ciphertexts, senderPk);
    const ciphertextHash = keccak256(
      encodeAbiParameters(
        [
          {
            type: "tuple",
            components: [
              { name: "amount", type: "uint256[3]" },
              { name: "r", type: "uint256[3]" },
              {
                name: "senderPk",
                type: "tuple",
                components: [
                  { name: "x", type: "uint256" },
                  { name: "y", type: "uint256" },
                ],
              },
            ],
          },
        ],
        [
          encodedCiphertexts,
        ],
      ),
    );

    // 4. Sign EIP-712 TransferFromAuthorization
    const domain = {
      name: extra.eip712Domain.name,
      version: extra.eip712Domain.version,
      chainId,
      verifyingContract: confidentialToken,
    };

    const message = {
      sender: getAddress(this.signer.address),
      receiver,
      amountCommitment,
      ciphertextHash,
      beta,
      nonce,
      deadline,
    };

    const signature = await this.signer.signTypedData({
      domain,
      types: {
        TransferFromAuthorization: [
          { name: "sender", type: "address" },
          { name: "receiver", type: "address" },
          { name: "amountCommitment", type: "uint256" },
          { name: "ciphertextHash", type: "bytes32" },
          { name: "beta", type: "uint256" },
          { name: "nonce", type: "uint256" },
          { name: "deadline", type: "uint256" },
        ],
      },
      primaryType: "TransferFromAuthorization",
      message,
    });

    // 5. Build the payload
    const payload: ConfidentialEvmPayload = {
      signature,
      authorization: {
        from: getAddress(this.signer.address),
        to: receiver,
        amountCommitment: amountCommitment.toString(),
        amountR: amountR.toString(),
        beta: beta.toString(),
        ciphertexts: [ciphertexts.ciphertexts0[0].toString(), ciphertexts.ciphertexts0[1].toString(), ciphertexts.ciphertexts1[0].toString(), ciphertexts.ciphertexts1[1].toString(), ciphertexts.ciphertexts2[0].toString(), ciphertexts.ciphertexts2[1].toString()],
        senderPk: [senderPk.x.toString(), senderPk.y.toString()],
        nonce: toHex(nonce, { size: 32 }),
        deadline: deadline.toString(),
        proof,
      }
    };

    return {
      x402Version,
      payload,
    };
  }
}
