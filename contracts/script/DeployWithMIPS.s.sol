// Copyright 2022 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

pragma solidity ^0.7.3;

import "forge-std/Script.sol";

import {MIPSExecutor} from "../src/MIPS/MIPSExecutor.sol";
import {MIPSMemory} from "../src/MIPS/MIPSMemory.sol";
import {Zipline} from "../src/Zipline.sol";

contract DeployWithMIPS is Script {
    function run() external {
        string memory mnemonic = "test test test test test test test test test test test junk";
        uint256 deployerPrivateKey = vm.deriveKey(mnemonic, 0);

        bytes32 goldenHash = vm.envBytes32("GOLDEN_SNAPSHOT");
        uint256 trustedEpoch = vm.envUint("TRUSTED_EPOCH");
        bytes32 trustedBlockRoot = vm.envBytes32("TRUSTED_BLOCK_ROOT");

        vm.startBroadcast(deployerPrivateKey);

        MIPSMemory m = new MIPSMemory();
        MIPSExecutor e = new MIPSExecutor(m);
        new Zipline(e, m, goldenHash, 0, uint64(trustedEpoch), trustedBlockRoot);

        vm.stopBroadcast();
    }
}
