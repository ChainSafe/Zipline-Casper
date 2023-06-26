// Copyright 2022 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

pragma solidity ^0.7.3;

/// @title IExecutorIO Interface
/// @author ChainSafe Systems
/// @notice Allows for reading and writing into designated places in an execution state
interface IExecutorIO {
    /// @notice Read the output value from the designated memory location
    /// Also checks that execution has terminated and that the output
    /// memory location was marked as written to
    function readOutput(bytes32 snapshotHash) external view returns (bytes32);

    /// @notice Write an input hash to the designated memory location and return the new memory snapshot
    function writeInput(bytes32 snapshotHash, bytes32 input) external returns (bytes32);
}
