// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

// import "forge-std/console.sol";
import {Action, ActionItem, ActionQueue, ActionQueueLib} from "./ActionQueue.sol";
import {BabyJubJub} from "@taceo/babyjubjub/BabyJubJub.sol";
import {ERC165} from "@openzeppelin/contracts/utils/introspection/ERC165.sol";
import {IERC20} from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {Poseidon2T2_BN254} from "./Poseidon2.sol";
import {SafeERC20} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";

interface IVerifierClient {
    function verifyCompressedProof(uint256[4] calldata compressedProof, uint256[15] calldata input) external view;
}

interface IVerifierServer {
    function verifyCompressedProof(uint256[4] calldata compressedProof, uint256[3] calldata input) external view;
}

interface IMercesMpc {
    function getNextActionIndex() external view returns (uint256);
    function getQueueSize() external returns (uint256);
    function processMPC(
        uint256 num_transactions,
        uint256[100] calldata commitments,
        bool[50] calldata valid,
        uint256 beta,
        uint256[4] calldata proof
    ) external returns (uint256[50] memory);
    function readQueue(uint256 num_items)
        external
        returns (uint256[] memory, ActionItem[] memory, Merces.Ciphertext[] memory);
    function retrieveFunds(address receiver) external;
}

string constant MERCES_TAG = "merces1";

contract Merces is ERC165, IMercesMpc {
    using BabyJubJub for BabyJubJub.Affine;
    using ActionQueueLib for ActionQueue;
    using SafeERC20 for IERC20;

    /// @notice The immutable domain tag of this contract
    /// @dev Used to verify the environment/customer context in `contractCompCheck`
    ///      Typically `keccak256("prod-{domain}")` or `keccak256("staging-{domain}")`
    bytes32 public immutable DOMAIN_TAG;

    // The verifier contracts
    IVerifierClient public immutable clientVerifier;
    IVerifierServer public immutable serverVerifier;

    // The token we use
    bool native; // If true, we use the native token
    IERC20 public immutable token;

    // The address of the MPC network allowed to post proofs
    address mpcAddress;

    // MPC public keys
    BabyJubJub.Affine public mpcPk1;
    BabyJubJub.Affine public mpcPk2;
    BabyJubJub.Affine public mpcPk3;

    // Stores the actions which are not yet processed
    ActionQueue public actionQueue;

    // Stores the commitments to the balances of users
    mapping(address => uint256) public balanceCommitments;

    // Stores the secret shares of the amount and randomness for a transfer
    mapping(uint256 => Ciphertext) private shares;

    // BN254 prime field
    uint256 constant PRIME = BabyJubJub.Q;

    // Batch size for processing actions
    uint256 private constant BATCH_SIZE = 50;
    // Commitment to zero balance commit(0, 0)
    uint256 private constant ZERO_COMMITMENT = 0x87f763a403ee4109adc79d4a7638af3cb8cb6a33f5b027bd1476ffa97361acb;

    // This is at most 2^80 / 10^18 = 1_208_925.8 ETH
    uint256 constant MAX_AMOUNT = 0xFFFFFFFFFFFFFFFFFFFF;
    uint256 private constant DS = 0xDEADBEEF;

    modifier onlyMpc() {
        _onlyMpc();
        _;
    }

    modifier validAmount(uint256 amount) {
        _validAmount(amount);
        _;
    }

    // We emit the location of the registered action indices for deposit, withdraw, and transfer
    event Deposit(uint256 actionIndex);
    event Withdraw(uint256 actionIndex);
    event Transfer(uint256 actionIndex);
    // We emit the location of the registered action indices which have been processed, as well as whether they were valid or not
    event ProcessedMPC(uint256[BATCH_SIZE] actionIndices, bool[BATCH_SIZE] valid);

    // The error codes
    error ContractComp();
    error Unauthorized();
    error InvalidAmount();
    error NotInPrimeField();
    error InvalidPoint();
    error InvalidTransfer();
    error InvalidMpcAction();
    error InvalidCommitment();

    // We do not store a nonce, since we assume that the sender_pk is randomly sampled each time
    struct Ciphertext {
        uint256[3] amount;
        uint256[3] r;
        BabyJubJub.Affine senderPk;
    }

    constructor(
        address _clientVerifier,
        address _serverVerifier,
        address _mpcAddress,
        address _tokenAddress,
        BabyJubJub.Affine memory _mpcPk1,
        BabyJubJub.Affine memory _mpcPk2,
        BabyJubJub.Affine memory _mpcPk3,
        string memory _environmentTag
    ) {
        _curveChecks(_mpcPk1);
        _curveChecks(_mpcPk2);
        _curveChecks(_mpcPk3);

        if (_tokenAddress == address(0)) {
            native = true;
        } else {
            native = false;
        }
        token = IERC20(_tokenAddress);

        clientVerifier = IVerifierClient(_clientVerifier);
        serverVerifier = IVerifierServer(_serverVerifier);
        mpcAddress = _mpcAddress;
        mpcPk1 = _mpcPk1;
        mpcPk2 = _mpcPk2;
        mpcPk3 = _mpcPk3;
        DOMAIN_TAG = keccak256(bytes(string.concat(_environmentTag, "-", MERCES_TAG)));

        // Initialize the default root for the layer
        ActionItem memory aq = ActionItem({action: Action.Dummy, sender: address(0), receiver: address(0), amount: 0});
        actionQueue.push(aq); // Dummy action at index 0
        actionQueue.lowestKey = 1; // We skip the dummy in the iterator
    }

    /// @notice Checks whether the contract implements a given interface
    /// @param interfaceId The interface ID to check (as per ERC-165)
    /// @return True if the contract supports the interface, false otherwise
    /// @dev This overrides `ERC165.supportsInterface` to include `IMercesMpc`
    function supportsInterface(bytes4 interfaceId) public view virtual override returns (bool) {
        return interfaceId == type(IMercesMpc).interfaceId || super.supportsInterface(interfaceId);
    }

    /// @notice Verifies that this contract matches a given interface and domain tag
    /// @param interfaceId The ERC-165 interface ID to check
    /// @param domainTag The domain tag (keccak256(bytes("{environment}-{domain}t"))environment) to verify
    /// @dev Reverts with `ContractComp()` if either check fails
    function contractCompCheck(bytes4 interfaceId, bytes32 domainTag) public view virtual {
        if (!supportsInterface(interfaceId) || DOMAIN_TAG != domainTag) {
            revert ContractComp();
        }
    }

    function getBalanceCommitment(address user) public view returns (uint256) {
        uint256 commitment = balanceCommitments[user];
        if (commitment == 0) {
            // Commitment is never zero with overwhelming probability
            return ZERO_COMMITMENT;
        }
        return commitment;
    }

    function getNextActionIndex() public view returns (uint256) {
        return actionQueue.peekIndex();
    }

    function readQueue(uint256 num_items)
        public
        view
        returns (uint256[] memory, ActionItem[] memory, Ciphertext[] memory)
    {
        uint256 size = actionQueue.len();
        if (num_items > size) {
            num_items = size;
        }
        ActionItem[] memory actions = new ActionItem[](num_items);
        uint256[] memory actionIndices = new uint256[](num_items);
        Ciphertext[] memory cts = new Ciphertext[](num_items);

        uint256 it = actionQueue.lowestKey;
        for (uint256 i = 0; i < num_items; i++) {
            actionIndices[i] = it;
            actions[i] = actionQueue.get(it);
            cts[i] = shares[it];
            it++;
        }
        return (actionIndices, actions, cts);
    }

    function getQueueSize() public view returns (uint256) {
        return actionQueue.len();
    }

    function getMpcPublicKeys()
        public
        view
        returns (BabyJubJub.Affine memory, BabyJubJub.Affine memory, BabyJubJub.Affine memory)
    {
        return (mpcPk1, mpcPk2, mpcPk3);
    }

    // TODO the following is just for a demo to be able to retrieve funds after it is done
    // Remove for a real deployment
    function retrieveFunds(address receiver) public onlyMpc {
        if (native) {
            (bool success,) = receiver.call{value: address(this).balance}("");
            require(success, "Token transfer failed");
        } else {
            token.safeTransfer(receiver, token.balanceOf(address(this)));
        }
    }

    function deposit(uint256 amount) public payable validAmount(amount) returns (uint256) {
        if (native && msg.value != amount) {
            revert InvalidAmount();
        }
        address receiver = msg.sender;
        if (amount == 0) {
            revert InvalidAmount();
        }

        ActionItem memory aq =
            ActionItem({action: Action.Deposit, sender: address(0), receiver: receiver, amount: amount});
        uint256 index = actionQueue.push(aq);

        emit Deposit(index);
        if (!native) {
            token.safeTransferFrom(msg.sender, address(this), amount);
        }
        return index;
    }

    function withdraw(uint256 amount) public validAmount(amount) returns (uint256) {
        address sender = msg.sender;
        // We do not check if the sender has a balance here, because it might be topped up by an action in the queue

        ActionItem memory aq =
            ActionItem({action: Action.Withdraw, sender: sender, receiver: address(0), amount: amount});
        uint256 index = actionQueue.push(aq);

        emit Withdraw(index);
        return index;
    }

    function transfer(
        address receiver,
        uint256 amountCommitment,
        // uint256 beta,
        Ciphertext calldata ciphertext,
        uint256[4] calldata proof
    ) public returns (uint256) {
        address sender = msg.sender;

        _checkParties(sender, receiver);
        _requireInPrimeField(amountCommitment);
        _checkCiphertext(ciphertext);

        // We do not check if the sender has a balance here, because it might be topped up by an action in the queue

        ActionItem memory aq =
            ActionItem({action: Action.Transfer, sender: sender, receiver: receiver, amount: amountCommitment});
        uint256 index = actionQueue.push(aq);

        _verifyTxClient(
            // beta,
            proof,
            [
                ciphertext.senderPk.x,
                ciphertext.senderPk.y,
                amountCommitment,
                ciphertext.amount[0],
                ciphertext.r[0],
                ciphertext.amount[1],
                ciphertext.r[1],
                ciphertext.amount[2],
                ciphertext.r[2],
                mpcPk1.x,
                mpcPk1.y,
                mpcPk2.x,
                mpcPk2.y,
                mpcPk3.x,
                mpcPk3.y
            ]
        );

        shares[index] = ciphertext;
        emit Transfer(index);
        return index;
    }

    // This function processes a batch of actions, updates the commitments,
    // and removes the actions from the queue.
    // Deposit and Withdraw are rewritten to be transfers
    function processMPC(
        uint256 num_transactions,
        uint256[BATCH_SIZE * 2] calldata commitments,
        bool[BATCH_SIZE] calldata valid,
        uint256 beta,
        uint256[4] calldata proof
    ) public onlyMpc returns (uint256[BATCH_SIZE] memory) {
        if (num_transactions > BATCH_SIZE) {
            revert InvalidMpcAction();
        }

        uint256[BATCH_SIZE * 6] memory publicInputs;
        uint256[BATCH_SIZE] memory indices;

        for (uint256 i = 0; i < num_transactions; i++) {
            uint256 validElement = valid[i] ? 1 : 0;
            uint256 currentIndex = actionQueue.peekIndex();
            indices[i] = currentIndex;
            ActionItem memory aq = actionQueue.pop();
            uint256 amount = aq.amount;

            if (aq.action == Action.Deposit) {
                uint256 receiverOldCommitment = getBalanceCommitment(aq.receiver);
                if (commitments[i * 2] != 0) {
                    revert InvalidCommitment();
                }
                _requireInPrimeField(commitments[i * 2 + 1]);

                // compute amount commitment
                uint256 amountCommitment = Poseidon2T2_BN254.compress([amount, 0], DS);

                // Update the commitments on-chain
                if (valid[i]) {
                    balanceCommitments[aq.receiver] = commitments[i * 2 + 1];
                }

                // Fill the publicInputs array for ZK proof verification
                publicInputs[i * 6] = amountCommitment; // sender_old_commitment
                publicInputs[i * 6 + 1] = ZERO_COMMITMENT; // sender_new_commitment
                publicInputs[i * 6 + 2] = receiverOldCommitment;
                publicInputs[i * 6 + 3] = commitments[i * 2 + 1]; // receiver_new_commitment
                publicInputs[i * 6 + 4] = amountCommitment;
                publicInputs[i * 6 + 5] = validElement;
            } else if (aq.action == Action.Withdraw) {
                uint256 senderOldCommitment = balanceCommitments[aq.sender];
                if (commitments[i * 2 + 1] != 0) {
                    revert InvalidCommitment();
                }
                _requireInPrimeField(commitments[i * 2]);

                // compute amount commitment
                uint256 amountCommitment = Poseidon2T2_BN254.compress([amount, 0], DS);

                // Update the commitments on-chain and send the actual tokens
                if (valid[i]) {
                    balanceCommitments[aq.sender] = commitments[i * 2];
                    _sendFunds(address(uint160(aq.sender)), aq.amount);
                }

                // Fill the commitments array for ZK proof verification
                publicInputs[i * 6] = senderOldCommitment;
                publicInputs[i * 6 + 1] = commitments[i * 2]; // sender_new_commitment
                publicInputs[i * 6 + 2] = ZERO_COMMITMENT; // receiver_old_commitment
                publicInputs[i * 6 + 3] = amountCommitment; // receiver_new_commitment
                publicInputs[i * 6 + 4] = amountCommitment;
                publicInputs[i * 6 + 5] = validElement;
            } else if (aq.action == Action.Transfer) {
                uint256 senderOldCommitment = balanceCommitments[aq.sender];
                uint256 receiverOldCommitment = getBalanceCommitment(aq.receiver);
                _requireInPrimeField(commitments[i * 2]);
                _requireInPrimeField(commitments[i * 2 + 1]);

                // Update the commitments on-chain
                if (valid[i]) {
                    balanceCommitments[aq.sender] = commitments[i * 2];
                    balanceCommitments[aq.receiver] = commitments[i * 2 + 1];
                }

                // Fill the commitments array for ZK proof verification
                publicInputs[i * 6] = senderOldCommitment;
                publicInputs[i * 6 + 1] = commitments[i * 2]; // sender_new_commitment
                publicInputs[i * 6 + 2] = receiverOldCommitment;
                publicInputs[i * 6 + 3] = commitments[i * 2 + 1]; // receiver_new_commitment
                publicInputs[i * 6 + 4] = amount; // Is already a commitment
                publicInputs[i * 6 + 5] = validElement;

                // delete shares[index]; // Actually costs more gas
            } else {
                revert InvalidMpcAction();
            }
        }

        for (uint256 i = num_transactions; i < BATCH_SIZE; i++) {
            // Do nothing, just add zeros to the commitments
            if (commitments[i * 2] != 0) {
                revert InvalidCommitment();
            }
            if (commitments[i * 2 + 1] != 0) {
                revert InvalidCommitment();
            }
            publicInputs[i * 6] = ZERO_COMMITMENT;
            publicInputs[i * 6 + 1] = ZERO_COMMITMENT;
            publicInputs[i * 6 + 2] = ZERO_COMMITMENT;
            publicInputs[i * 6 + 3] = ZERO_COMMITMENT;
            publicInputs[i * 6 + 4] = ZERO_COMMITMENT;
            publicInputs[i * 6 + 5] = valid[i] ? 1 : 0;
            // indices[i] = 0; // Dummy index
        }

        _verifyTxServer(beta, proof, publicInputs);
        emit ProcessedMPC(indices, valid);
        return indices;
    }

    ////////////////////////////////////////////////////////////
    //                  Internal Helpers                      //
    ////////////////////////////////////////////////////////////

    function _onlyMpc() internal view virtual {
        if (msg.sender != mpcAddress) revert Unauthorized();
    }

    function _validAmount(uint256 amount) internal pure virtual {
        if (amount == 0 || amount > MAX_AMOUNT) revert InvalidAmount();
    }

    function _requireInPrimeField(uint256 value) internal pure {
        if (value >= PRIME) revert NotInPrimeField();
    }

    function _requireArrayInPrimeField(uint256[3] calldata arr) internal pure {
        _requireInPrimeField(arr[0]);
        _requireInPrimeField(arr[1]);
        _requireInPrimeField(arr[2]);
    }

    function _checkCiphertext(Ciphertext calldata ciphertext) internal pure {
        _curveChecks(ciphertext.senderPk);
        _requireArrayInPrimeField(ciphertext.amount);
        _requireArrayInPrimeField(ciphertext.r);
    }

    /// Performs sanity checks on BabyJubJub elements. If either the point
    ///     * is the identity
    ///     * is not on the curve
    ///     * is not in the large sub-group
    ///
    /// this method will revert the call.
    function _curveChecks(BabyJubJub.Affine memory element) internal pure virtual {
        if (
            BabyJubJub.isIdentity(element) || !BabyJubJub.isOnCurve(element)
                || !BabyJubJub.isInCorrectSubgroupAssumingOnCurve(element)
        ) {
            revert InvalidPoint();
        }
    }

    function _checkParties(address sender, address receiver) internal pure {
        if (sender == receiver) {
            revert InvalidTransfer();
        }
    }

    function _sendFunds(address receiver, uint256 amount) internal {
        if (native) {
            (bool success,) = receiver.call{value: amount}("");
            require(success, "Token transfer failed");
        } else {
            token.safeTransfer(receiver, amount);
        }
    }

    function _computeSha256(bytes memory input) internal pure returns (uint256 alpha) {
        bytes32 hash = sha256(input);
        alpha = uint256(hash);
        alpha = (alpha << 3) >> 3; // Drop three bits from the calculated hash
    }

    function _computeUhfServer(uint256 alphaParam, uint256 beta, uint256[BATCH_SIZE * 6] memory x)
        internal
        pure
        returns (uint256 gamma)
    {
        uint256 seed = alphaParam;
        unchecked {
            seed += beta;
        }

        uint256 mul = 0;
        for (uint256 i = x.length - 1; i > 0; i--) {
            mul = mulmod(seed, mul + x[i], PRIME);
        }

        gamma = addmod(mul, x[0], PRIME);
    }

    function _computeUhfClient(uint256 alphaParam, uint256 beta, uint256[15] memory x)
        internal
        pure
        returns (uint256 gamma)
    {
        uint256 seed = alphaParam;
        unchecked {
            seed += beta;
        }

        uint256 mul = 0;
        for (uint256 i = x.length - 1; i > 0; i--) {
            mul = mulmod(seed, mul + x[i], PRIME);
        }

        gamma = addmod(mul, x[0], PRIME);
    }

    /// @notice Computes compressed public input hashes and verifies the client proof.
    /// @dev This function derives `alpha` and `gamma` to compress the full set of public inputs, reducing the number of inputs required by the verifier and lowering on-chain gas costs. See https://eprint.iacr.org/2025/1500 for details on the compression technique.
    ///
    /// `alpha` is computed as the SHA256 hash of the public inputs. The hash output is truncated (dropping the three highest bits) inside `_computeSHA256`, ensuring the resulting value lies within the scalar field.
    ///
    /// `beta` is supplied by the caller. Since `beta` is passed as a public input to the verifier contract, the verifier contract enforces that it lies in the correct field. If `beta` exceeds the field modulus, the proof verification will fail.
    ///
    /// Note that an intermediate integer overflow may occur if `beta` is very large when calling the universal hash function (`_computeUHF`). This does not introduce a security issue and simply results in proof verification failing.
    ///
    /// After computing `alpha` and `gamma`, the function directly calls `clientTransferVerifier.verifyCompressedProof` to verify the proof on-chain.
    ///
    // / @param beta Random challenge provided by the user.
    /// @param proof The client proof array to verify.
    /// @param publicInputs The full set of public inputs of the circuit.
    function _verifyTxClient(uint256[4] calldata proof, uint256[15] memory publicInputs) internal view virtual {
        // uint256 alpha = _computeSha256(abi.encodePacked(publicInputs));
        // uint256 gamma = _computeUhfClient(alpha, beta, publicInputs);
        clientVerifier.verifyCompressedProof(proof, publicInputs);
    }

    function _verifyTxServer(uint256 beta, uint256[4] calldata proof, uint256[BATCH_SIZE * 6] memory publicInputs)
        internal
        view
        virtual
    {
        uint256 alpha = _computeSha256(abi.encodePacked(publicInputs));
        uint256 gamma = _computeUhfServer(alpha, beta, publicInputs);
        serverVerifier.verifyCompressedProof(proof, [beta, gamma, alpha]);
    }
}
