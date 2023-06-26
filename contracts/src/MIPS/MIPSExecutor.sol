// Copyright 2022 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

pragma solidity ^0.7.3;

import {MIPS} from "./MIPS.sol";
import {MIPSMemory} from "./MIPSMemory.sol";

import {IExecutor} from "../interfaces/IExecutor.sol";

/// @notice An example integrating the Optimism MIPS executor
///         Just a super thin wrapper in this instance
contract MIPSExecutor is IExecutor, MIPS {
    constructor(MIPSMemory _m) MIPS(_m) {}

    function step(bytes32 snapshotHash) external override returns (bytes32) {
        return Step(snapshotHash);
    }
}
