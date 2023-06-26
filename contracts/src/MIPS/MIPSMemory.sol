// Copyright 2022 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

pragma solidity ^0.7.3;

import {CannonMIPSMemory} from "./CannonMIPSMemory.sol";
import {IExecutorIO} from "../interfaces/IExecutorIO.sol";

/// @notice An example IO implementation for a Cannon style MIPSMemory
///         This handles reading and writing to the correct addresses as expected
///         by a cannon MIPS execution
contract MIPSMemory is IExecutorIO, CannonMIPSMemory {
    // input should be placed at this address to produce the initial snapshot for an execution trace
    uint32 constant INPUT_MEMORY_ADDR = 0x30000000;

    // If the execution successfully terminated
    // TERMINATE_VAL should be written to address TERMINATE_MEMORY_ADDR
    uint32 constant TERMINATE_MEMORY_ADDR = 0xC0000080;
    uint32 constant TERMINATE_VAL = 0x5EAD0000;

    // If the execution wrote an output then
    // OUTPUT_WRITTEN_VAL should be written to OUTPUT_WRITTEN_ADDR
    // and the value is located at OUTPUT_ADDR
    uint32 constant OUTPUT_WRITTEN_ADDR = 0x30000800;
    uint32 constant OUTPUT_WRITTEN_VAL = 0x1337f00d;
    uint32 constant OUTPUT_ADDR = 0x30000804;

    function readOutput(bytes32 snapshotHash) external view override returns (bytes32) {
        require(
            ReadMemory(snapshotHash, TERMINATE_MEMORY_ADDR) == TERMINATE_VAL,
            "the final executor state is not stopped (PC != 0x5EAD0000)"
        );
        require(
            ReadMemory(snapshotHash, OUTPUT_WRITTEN_ADDR) == OUTPUT_WRITTEN_VAL,
            "the output has not been written to the predefined memory location"
        );
        return ReadBytes32(snapshotHash, OUTPUT_ADDR);
    }

    function writeInput(bytes32 snapshotHash, bytes32 input) external override returns (bytes32) {
        return WriteBytes32(snapshotHash, INPUT_MEMORY_ADDR, input);
    }
}
