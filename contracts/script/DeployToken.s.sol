// SPDX-License-Identifier: MIT
pragma solidity ^0.8.20;

import {Script, console} from "forge-std/Script.sol";
import {USDT0Token, USDCToken} from "../src/Token.sol";

contract DeployToken is Script {
    function run() external {
        // TODO better value
        uint256 initialSupply = type(uint256).max;
        string memory token = vm.envString("TOKEN");

        if (keccak256(bytes(token)) == keccak256(bytes("erc20"))) {
            console.log("Deploying USDCToken with parameters:");
            console.log("  Initial Supply:", initialSupply);
            vm.startBroadcast();
            USDCToken tokenContract = new USDCToken(initialSupply);
            vm.stopBroadcast();
            console.log("USDCToken deployed at:", address(tokenContract));
        } else if (keccak256(bytes(token)) == keccak256(bytes("eip3009"))) {
            console.log("Deploying USDT0Token with parameters:");
            console.log("  Initial Supply:", initialSupply);
            vm.startBroadcast();
            USDT0Token tokenContract = new USDT0Token(initialSupply);
            vm.stopBroadcast();
            console.log("USDT0Token deployed at:", address(tokenContract));
        } else {
            require(false, "invalid token");
        }
    }
}
