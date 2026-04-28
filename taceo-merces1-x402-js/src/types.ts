/**
 * The authorization data signed by the client for a confidential transferFrom.
 */
export type ConfidentialAuthorization = {
  from: `0x${string}`;
  to: `0x${string}`;
  amountCommitment: string;
  beta: string;
  ciphertexts: [string, string, string, string, string, string];
  senderPk: [string, string];
  nonce: string;
  deadline: string,
  proof: snarkjs.Groth16Proof,
};

/**
 * The full confidential payment payload sent by the client.
 */
export type ConfidentialEvmPayload = {
  signature: `0x${string}`;
  authorization: ConfidentialAuthorization;
};

/**
 * Extra data included in PaymentRequirements for the confidential scheme.
 * Provides the client with contract address, EIP-712 domain, and MPC public keys.
 */
export type ConfidentialExtra = {
  confidentialToken: `0x${string}`;
  eip712Domain: {
    name: string;
    version: string;
  };
  mpcPks: [[string, string], [string, string], [string, string]];
};
