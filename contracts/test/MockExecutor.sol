// Copyright 2022 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

pragma solidity ^0.7.3;

import {IExecutor} from "../src/interfaces/IExecutor.sol";
import {IExecutorIO} from "../src/interfaces/IExecutorIO.sol";

/// @notice An executor used for testing
/// This simple increments 1 to the state hash with each step
contract MockExecutor is IExecutor {
    function step(bytes32 snapshotHash) external pure override returns (bytes32) {
        return bytes32(uint256(snapshotHash) + 1);
    }
}

// these are NOOPs for the mock memory model
contract MockExecutorIO is IExecutorIO {
    function readOutput(bytes32 snapshotHash) external pure override returns (bytes32) {
        return snapshotHash;
    }

    // @notice Write an input hash to the designated memory location
    function writeInput(bytes32 snapshotHash, bytes32) external pure override returns (bytes32) {
        return snapshotHash;
    }
}
