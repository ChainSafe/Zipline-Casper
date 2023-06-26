// Copyright 2022 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

pragma solidity ^0.7.3;

/// @title IExecutor interface
/// @author ChainSafe Systems
/// @notice An interface to a machine which can perform a single step execution on a program snapshot
interface IExecutor {
    /// @notice Given a snapshot hash (includes code & registers), execute the next instruction and returns
    ///         the update snapshot hash.
    function step(bytes32 snapshotHash) external returns (bytes32);
}
