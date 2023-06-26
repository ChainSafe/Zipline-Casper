// Copyright 2022 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

/// Manages k-section challenges over an execution trace of any IExecutor
/// Players directly call either `dissectChallenge` or `proveChallenge` depending on if the k-section has
/// narrowed the trace down to one step or is still in the process. Each call requires the FaultAssertion passed to
/// maintain the context (this is passed rather than stored to save storage costs)
///
/// Challenges always terminate from a timeout. Either from a party ceasing to respond or because the game has terminated and set the challengeHash to 0

pragma solidity ^0.7.3;
pragma abicoder v2;

import {IExecutor} from "./interfaces/IExecutor.sol";
import { CHALLENGE_TOTAL_TIME_BLOCKS } from "./Constants.sol";

abstract contract CGOrchestrator {
    /// @notice A player in a challenge
    struct Player {
        address addr;
        uint256 blocksRemaining;
    }

    /// @notice A challenge over an execution trace
    struct Challenge {
        bool active; // is the challenge currently open and underway
        uint256 lastMoveBlockHeight;
        bytes32 gameCommitmentHash; // commitment to the state of the challenge (e.g. the segments executed so far)
        Player current;
        Player next;
    }

    /// @notice A slice of the execution trace with a start and length
    struct Slice {
        uint256 start;
        uint256 length;
    }

    /// @notice A segmentation of the execution trace which is splitting
    /// a slice into a number of segments defined by the snapshots at the
    struct Segmentation {
        Slice span;
        bytes32[] snapshots;
    }

    /// @notice A fault assertion that a player makes about the execution trace
    struct FaultAssertion {
        Segmentation prevSegmentation;
        uint256 challengePosition;
    }

    /// @notice The ID of the latest challenge
    uint64 public nonce;

    /// @notice Map from challenge ID to challenge
    mapping(uint256 => Challenge) public challenges;

    IExecutor public immutable executor;
    uint256 public immutable k; // number of sections to dissect the trace each round

    /// @dev Callback that the inheriting contract must implement
    function challengeConcluded(uint256 challengeId, address winner, address loser) internal virtual;

    constructor(IExecutor executor_, uint256 k_) {
        executor = executor_;
        k = k_;
    }

    modifier onlyCurrentPlayer(uint64 challengeId) {
        require(challenges[challengeId].active, "Challenge does not exist");
        require(tx.origin == currentResponder(challengeId), "Not this players turn");
        require(!isTimedOut(challengeId), "Challenge has already timed out");
        _;
    }

    function newChallenge(
        bytes32[2] memory startAndEndSnapshots,
        uint256 nSteps,
        address defender,
        address challenger
    ) internal returns (uint64) {
        // grab the current challenge index and increment the nonce
        uint64 challengeId = ++nonce;

        // place initial sections
        bytes32[] memory sections = new bytes32[](2);
        sections[0] = startAndEndSnapshots[0];
        sections[1] = startAndEndSnapshots[1];

        Challenge storage challenge = challenges[challengeId];

        // set up the participants
        challenge.next = Player({addr: defender, blocksRemaining: CHALLENGE_TOTAL_TIME_BLOCKS});
        challenge.current = Player({addr: challenger, blocksRemaining: CHALLENGE_TOTAL_TIME_BLOCKS});
        challenge.lastMoveBlockHeight = block.number;
        challenge.active = true;
        challenge.gameCommitmentHash = computeSegmentationCommitment(Segmentation(Slice(0, nSteps), sections));
        return challengeId;
    }

    function dissectChallenge(uint64 challengeId, FaultAssertion calldata assertion, bytes32[] calldata newSnapshots)
        public
        onlyCurrentPlayer(challengeId)
    {
        requireValidAssertion(challengeId, assertion);
        Slice memory nextSlice = extractNextSlice(assertion);
        require(nextSlice.length > 1, "trace length too short to dissect. Call prove instead.");
        require(newSnapshots.length == min(k, nextSlice.length) + 1, "invalid number of segments");
        require(
            assertion.prevSegmentation.snapshots[assertion.challengePosition] == newSnapshots[0],
            "players must agree on segment start"
        );
        require(
            assertion.prevSegmentation.snapshots[assertion.challengePosition + 1]
                != newSnapshots[newSnapshots.length - 1],
            "players must disagree on segment end"
        );

        // update the game commitment hash with the new segmentation
        challenges[challengeId].gameCommitmentHash =
            computeSegmentationCommitment(Segmentation(nextSlice, newSnapshots));
        switchPlayers(challengeId);
    }

    /// Once the challenge length has been narrowed down to 1 we can delegate to the
    /// executor to find the next snapshot and confirm if there is fraud or not
    function proveChallenge(uint64 challengeId, FaultAssertion calldata assertion)
        public
        onlyCurrentPlayer(challengeId)
    {
        requireValidAssertion(challengeId, assertion);
        Slice memory nextSection = extractNextSlice(assertion);
        require(nextSection.length == 1, "too many steps to prove. Must be exactly 1");

        // Actually do the on-chain execution of the next instruction with the executor
        bytes32 afterSnapshot = executor.step(assertion.prevSegmentation.snapshots[assertion.challengePosition]);

        require(
            afterSnapshot != assertion.prevSegmentation.snapshots[assertion.challengePosition + 1],
            "no fault in trace at the given position"
        );

        currentPlayerWins(challengeId);
        switchPlayers(challengeId);
    }

    /// @notice This function end a challenge if it cannot finish or if the current player does not respond 
    function timeoutChallenge(uint64 challengeId) public {
        require(challenges[challengeId].active, "Challenge does not exist or is not active");
        require(isTimedOut(challengeId), "Challenge has not timed out");
        nextPlayerWins(challengeId);
    }

    function currentResponder(uint64 challengeId) public view returns (address) {
        return challenges[challengeId].current.addr;
    }

    /// @dev This function causes the open field of the challenge to be set to false by deleting the challenge
    function nextPlayerWins(uint64 challengeId) private {
        Challenge storage challenge = challenges[challengeId];
        address next = challenge.next.addr;
        address current = challenge.current.addr;
        delete challenges[challengeId];
        challengeConcluded(challengeId, next, current);
    }

    /// @dev This sets the gameCommitmentHash to zero thus preventing any further moves and ensuring a timeout for the other player
    function currentPlayerWins(uint64 challengeId) private {
        Challenge storage challenge = challenges[challengeId];
        challenge.gameCommitmentHash = bytes32(0);
    }

    function isTimedOut(uint64 challengeId) public view returns (bool) {
        Challenge storage challenge = challenges[challengeId];
        uint256 blocksSinceLastMove = block.number - challenge.lastMoveBlockHeight;
        return blocksSinceLastMove > challenge.current.blocksRemaining;
    }

    // Extracts the section of the trace indicated by the challengePosition of the assertion from the previous Segmentation
    // returns a start and length
    function extractNextSlice(FaultAssertion calldata assertion) internal pure returns (Slice memory) {
        uint256 oldChallengeDegree = assertion.prevSegmentation.snapshots.length - 1;
        uint256 length = assertion.prevSegmentation.span.length / oldChallengeDegree;
        // Intentionally done before challengeLength is potentially added to for the final segment
        uint256 start = assertion.prevSegmentation.span.start + length * assertion.challengePosition;

        // this is a special case for the last segment. If the trace is not divisible by the
        // degree then add the remainder into the last segment.
        if (assertion.challengePosition == assertion.prevSegmentation.snapshots.length - 2) {
            length += assertion.prevSegmentation.span.length % oldChallengeDegree;
        }
        return Slice(start, length);
    }

    function computeSegmentationCommitment(Segmentation memory segmentation) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked(segmentation.span.start, segmentation.span.length, segmentation.snapshots));
    }

    function switchPlayers(uint64 challengeId) internal {
        Challenge storage challenge = challenges[challengeId];
        // don't switch roles if the game just terminated
        if (!challenge.active) {
            return; // the function body resulted in the challenge being closed
        }
        // switch roles and adjust the chess-clock
        Player memory current = challenge.current;
        current.blocksRemaining -= block.number - challenge.lastMoveBlockHeight;

        challenge.current = challenge.next;
        challenge.next = current;
        challenge.lastMoveBlockHeight = block.number;
    }

    /// Require that the assertion correctly provides a segmentation matching the prior commitment
    function requireValidAssertion(uint64 challengeId, FaultAssertion calldata assertion) internal view {
        require(
            challenges[challengeId].gameCommitmentHash == computeSegmentationCommitment(assertion.prevSegmentation),
            "fault assertion segmentation does not match commitment"
        );
        require(
            assertion.prevSegmentation.snapshots.length >= 2
                && assertion.challengePosition < assertion.prevSegmentation.snapshots.length - 1,
            "challenge position invalid"
        );
    }

    function min(uint256 a, uint256 b) internal pure returns (uint256) {
        if (a < b) {
            return a;
        }
        return b;
    }
}
