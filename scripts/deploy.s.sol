// SPDX-License-Identifier: MIT
pragma solidity ^0.8.24;

import "forge-std/Script.sol";
import "../src/ArbExecutor.sol";

contract DeployArbExecutor is Script {
    function run() external {
        uint256 pk = vm.envUint("DEPLOYER_PRIVATE_KEY");
        vm.startBroadcast(pk);

        ArbExecutor exec = new ArbExecutor();
        console2.log("ArbExecutor deployed at:", address(exec));

        vm.stopBroadcast();
    }
}
