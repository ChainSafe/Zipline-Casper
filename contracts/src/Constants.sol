// Copyright 2022 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

pragma solidity ^0.7.3;

uint256 constant K = 15; // How many sections to split the trace into each turn of a challenge game. This is the k in k-section

uint256 constant CHALLENGE_TOTAL_TIME_BLOCKS = 30; // blocks allowed for each participant in the challenge game. Cumulitive like a chess clock

uint64 constant FINALIZATION_TIME_BLOCKS = 100; // Number of blocks an update must be pending and unchallenged before it can finalize
