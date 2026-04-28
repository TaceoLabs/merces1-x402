// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {ERC20} from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

contract USDCToken is ERC20 {
    address owner;

    // The error codes
    error Unauthorized();

    modifier onlyOwner() {
        _onlyOwner();
        _;
    }

    constructor(uint256 initialSupply) ERC20("USDC", "USDC") {
        owner = msg.sender;
        _mint(msg.sender, initialSupply);
    }

    function decimals() public pure override returns (uint8) {
        return 6;
    }

    function _onlyOwner() internal view virtual {
        if (msg.sender != owner) revert Unauthorized();
    }

    function mint(address to, uint256 amount) public onlyOwner {
        _mint(to, amount);
    }
}

contract USDT0Token is ERC20 {
    address owner;

    // EIP-712 Domain
    bytes32 public constant DOMAIN_TYPEHASH =
        keccak256("EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)");

    // Version for EIP-712
    string public constant version = "1";

    // EIP-3009 Type Hashes (same as USDC/USDT0)
    bytes32 public constant TRANSFER_WITH_AUTHORIZATION_TYPEHASH =
        0x7c7c6cdb67a18743f49ec6fa9b35f50d52ed05cbed4cc592e13b44501c1a2267;

    bytes32 public constant RECEIVE_WITH_AUTHORIZATION_TYPEHASH =
        0xd099cc98ef71107a616c4f0f941f04c322d8e254fe26b3c6668db87aae413de8;

    bytes32 public constant CANCEL_AUTHORIZATION_TYPEHASH =
        0x158b0a9edf7a828aad02f63cd515c68ef2f50ba807396f6d12842833a1597429;

    // Authorization states
    mapping(address => mapping(bytes32 => bool)) private _authorizationStates;

    // Events
    event AuthorizationUsed(address indexed authorizer, bytes32 indexed nonce);
    event AuthorizationCanceled(address indexed authorizer, bytes32 indexed nonce);

    // The error codes
    error Unauthorized();

    modifier onlyOwner() {
        _onlyOwner();
        _;
    }

    constructor(uint256 initialSupply) ERC20("USDT0", "USDT0") {
        owner = msg.sender;
        _mint(msg.sender, initialSupply);
    }

    function decimals() public pure override returns (uint8) {
        return 6;
    }

    function _onlyOwner() internal view virtual {
        if (msg.sender != owner) revert Unauthorized();
    }

    function mint(address to, uint256 amount) public onlyOwner {
        _mint(to, amount);
    }

    /**
     * @notice Returns the domain separator for EIP-712
     */
    function DOMAIN_SEPARATOR() public view returns (bytes32) {
        // return keccak256(
        //     abi.encode(
        //         DOMAIN_TYPEHASH, keccak256(bytes(name())), keccak256(bytes(version)), block.chainid, address(this)
        //     )
        // );

        // keccak256(bytes(name()))
        string memory n = name();
        bytes32 hashname;
        assembly {
            hashname := keccak256(add(n, 32), mload(n))
        }

        // keccak256(bytes(version));
        string memory v = version;
        bytes32 hashversion;
        assembly {
            hashversion := keccak256(add(v, 32), mload(v))
        }

        //  keccak256(
        //     abi.encode(
        //         DOMAIN_TYPEHASH, keccak256(bytes(name())), keccak256(bytes(version)), block.chainid, address(this)
        //     )
        // );
        bytes32 ds;
        bytes32 _DOMAIN_TYPEHASH = DOMAIN_TYPEHASH;
        assembly {
            let mPtr := mload(0x40)
            mstore(mPtr, _DOMAIN_TYPEHASH)
            mstore(add(mPtr, 32), hashname)
            mstore(add(mPtr, 64), hashversion)
            mstore(add(mPtr, 96), chainid())
            mstore(add(mPtr, 128), address())
            ds := keccak256(mPtr, 160)
        }
        return ds;
    }

    /**
     * @notice Execute a transfer with a signed authorization
     * @dev EIP-3009 transferWithAuthorization
     */
    function transferWithAuthorization(
        address from,
        address to,
        uint256 value,
        uint256 validAfter,
        uint256 validBefore,
        bytes32 nonce,
        uint8 v,
        bytes32 r,
        bytes32 s
    ) external {
        require(block.timestamp > validAfter, "USDT0: authorization is not yet valid");
        require(block.timestamp < validBefore, "USDT0: authorization is expired");
        require(!_authorizationStates[from][nonce], "USDT0: authorization is used");

        // bytes32 structHash = keccak256(
        //     abi.encode(TRANSFER_WITH_AUTHORIZATION_TYPEHASH, from, to, value, validAfter, validBefore, nonce)
        // );
        bytes32 structHash;
        assembly {
            let mPtr := mload(0x40)
            mstore(mPtr, TRANSFER_WITH_AUTHORIZATION_TYPEHASH)
            mstore(add(mPtr, 32), from)
            mstore(add(mPtr, 64), to)
            mstore(add(mPtr, 96), value)
            mstore(add(mPtr, 128), validAfter)
            mstore(add(mPtr, 160), validBefore)
            mstore(add(mPtr, 192), nonce)
            structHash := keccak256(mPtr, 224)
        }

        // bytes32 hash = keccak256(abi.encodePacked("\x19\x01", DOMAIN_SEPARATOR(), structHash));
        bytes32 ds = DOMAIN_SEPARATOR();
        bytes32 hash;
        assembly {
            let mPtr := mload(0x40)
            mstore(mPtr, "\x19\x01")
            mstore(add(mPtr, 2), ds)
            mstore(add(mPtr, 34), structHash)
            hash := keccak256(mPtr, 66)
        }

        address recoveredAddress = ecrecover(hash, v, r, s);
        require(recoveredAddress != address(0) && recoveredAddress == from, "USDT0: invalid signature");

        _authorizationStates[from][nonce] = true;
        emit AuthorizationUsed(from, nonce);

        _transfer(from, to, value);
    }

    /**
     * @notice Receive a transfer with a signed authorization from the payer
     * @dev EIP-3009 receiveWithAuthorization with payee verification
     */
    function receiveWithAuthorization(
        address from,
        address to,
        uint256 value,
        uint256 validAfter,
        uint256 validBefore,
        bytes32 nonce,
        uint8 v,
        bytes32 r,
        bytes32 s
    ) external {
        require(to == _msgSender(), "USDT0: caller must be the payee");
        require(block.timestamp > validAfter, "USDT0: authorization is not yet valid");
        require(block.timestamp < validBefore, "USDT0: authorization is expired");
        require(!_authorizationStates[from][nonce], "USDT0: authorization is used");

        // bytes32 structHash =
        //     keccak256(abi.encode(RECEIVE_WITH_AUTHORIZATION_TYPEHASH, from, to, value, validAfter, validBefore, nonce));
        bytes32 structHash;
        assembly {
            let mPtr := mload(0x40)
            mstore(mPtr, RECEIVE_WITH_AUTHORIZATION_TYPEHASH)
            mstore(add(mPtr, 32), from)
            mstore(add(mPtr, 64), to)
            mstore(add(mPtr, 96), value)
            mstore(add(mPtr, 128), validAfter)
            mstore(add(mPtr, 160), validBefore)
            mstore(add(mPtr, 192), nonce)
            structHash := keccak256(mPtr, 224)
        }

        // bytes32 hash = keccak256(abi.encodePacked("\x19\x01", DOMAIN_SEPARATOR(), structHash));
        bytes32 ds = DOMAIN_SEPARATOR();
        bytes32 hash;
        assembly {
            let mPtr := mload(0x40)
            mstore(mPtr, "\x19\x01")
            mstore(add(mPtr, 2), ds)
            mstore(add(mPtr, 34), structHash)
            hash := keccak256(mPtr, 66)
        }

        address recoveredAddress = ecrecover(hash, v, r, s);
        require(recoveredAddress != address(0) && recoveredAddress == from, "USDT0: invalid signature");

        _authorizationStates[from][nonce] = true;
        emit AuthorizationUsed(from, nonce);

        _transfer(from, to, value);
    }

    /**
     * @notice Attempt to cancel an authorization
     * @dev EIP-3009 cancelAuthorization
     */
    function cancelAuthorization(address authorizer, bytes32 nonce, uint8 v, bytes32 r, bytes32 s) external {
        require(!_authorizationStates[authorizer][nonce], "USDT0: authorization is used");

        // bytes32 structHash = keccak256(abi.encode(CANCEL_AUTHORIZATION_TYPEHASH, authorizer, nonce));
        bytes32 structHash;
        assembly {
            let mPtr := mload(0x40)
            mstore(mPtr, CANCEL_AUTHORIZATION_TYPEHASH)
            mstore(add(mPtr, 32), authorizer)
            mstore(add(mPtr, 64), nonce)
            structHash := keccak256(mPtr, 96)
        }

        // bytes32 hash = keccak256(abi.encodePacked("\x19\x01", DOMAIN_SEPARATOR(), structHash));
        bytes32 ds = DOMAIN_SEPARATOR();
        bytes32 hash;
        assembly {
            let mPtr := mload(0x40)
            mstore(mPtr, "\x19\x01")
            mstore(add(mPtr, 2), ds)
            mstore(add(mPtr, 34), structHash)
            hash := keccak256(mPtr, 66)
        }

        address recoveredAddress = ecrecover(hash, v, r, s);
        require(recoveredAddress != address(0) && recoveredAddress == authorizer, "USDT0: invalid signature");

        _authorizationStates[authorizer][nonce] = true;
        emit AuthorizationCanceled(authorizer, nonce);
    }
}
