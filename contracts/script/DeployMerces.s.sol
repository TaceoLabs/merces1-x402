// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Script, console} from "forge-std/Script.sol";

import {BabyJubJub} from "@taceo/babyjubjub/BabyJubJub.sol";
import {Merces} from "../src/Merces.sol";
import {Verifier as ClientVerifier} from "../src/verifiers/VerifierClient.sol";
import {Verifier as ServerVerifier} from "../src/verifiers/VerifierServer.sol";

contract DeployMerces is Script {
    using BabyJubJub for BabyJubJub.Affine;

    function run() external returns (Merces) {
        // Read deployment parameters from environment variables
        address mpcAddress = vm.envAddress("MPC_ADDRESS");
        address tokenAddress = vm.envAddress("TOKEN_ADDRESS");
        string memory deployment = vm.envString("MERCES_DEPLOYMENT");

        // MPC public keys
        uint256 mpcPk1X = vm.envUint("MPC_PK1_X");
        uint256 mpcPk1Y = vm.envUint("MPC_PK1_Y");
        uint256 mpcPk2X = vm.envUint("MPC_PK2_X");
        uint256 mpcPk2Y = vm.envUint("MPC_PK2_Y");
        uint256 mpcPk3X = vm.envUint("MPC_PK3_X");
        uint256 mpcPk3Y = vm.envUint("MPC_PK3_Y");

        console.log("Deploying Merces with parameters:");
        console.log("  MPC Address:", mpcAddress);
        console.log("  Token Address:", tokenAddress);
        console.log("  Deployment:", deployment);

        BabyJubJub.Affine memory mpcPk1 = BabyJubJub.Affine({x: mpcPk1X, y: mpcPk1Y});

        BabyJubJub.Affine memory mpcPk2 = BabyJubJub.Affine({x: mpcPk2X, y: mpcPk2Y});

        BabyJubJub.Affine memory mpcPk3 = BabyJubJub.Affine({x: mpcPk3X, y: mpcPk3Y});

        address clientVerifierAddr = vm.envOr("CLIENT_VERIFIER_ADDRESS", address(0));
        address serverVerifierAddr = vm.envOr("SERVER_VERIFIER_ADDRESS", address(0));

        vm.startBroadcast();

        console.log("Deploying verifier contracts...");
        if (clientVerifierAddr == address(0)) {
            clientVerifierAddr = address(new ClientVerifier());
            console.log("  Client Verifier deployed at:", clientVerifierAddr);
        } else {
            console.log("  Client Verifier (existing):", clientVerifierAddr);
        }

        if (serverVerifierAddr == address(0)) {
            serverVerifierAddr = address(new ServerVerifier());
            console.log("  Server Verifier deployed at:", serverVerifierAddr);
        } else {
            console.log("  Server Verifier (existing):", serverVerifierAddr);
        }

        console.log("Deploying Merces contract...");
        Merces merces = new Merces(
            clientVerifierAddr, serverVerifierAddr, mpcAddress, tokenAddress, mpcPk1, mpcPk2, mpcPk3, deployment
        );

        vm.stopBroadcast();

        console.log("Merces deployed at:", address(merces));
        return merces;
    }
}
