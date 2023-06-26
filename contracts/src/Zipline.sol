// Copyright 2022 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

pragma solidity ^0.7.3;
pragma abicoder v2;

import {IExecutor} from "./interfaces/IExecutor.sol";
import {IExecutorIO} from "./interfaces/IExecutorIO.sol";
import {CGOrchestrator} from "./CGOrchestrator.sol";

import {FINALIZATION_TIME_BLOCKS, K} from "./Constants.sol";
import {SSZ} from "./lib/SSZ.sol";

/// @title Zipline Block Oracle
/// @author ChainSafe Systems
/// @notice This is the primary contract by which permissionless participants interact with the Zipline protocol
/// Each instance of this contract represents a single instance of an oracle from an origin chain.
/// Block roots that the protocol deems trustworth can be ready from the `trustedBlockRoots` map by their epoch number to be used by
/// cross chain applications.
/// @dev This is currently a proof-of-concept and is not yet ready for production use
contract Zipline is CGOrchestrator {
    struct PendingSubmission {
        // account that submitted the Submission
        address creator;
        // candidate epoch
        uint64 epoch;
        // candidate block hash
        bytes32 blockRoot;
        // block height at which this will finalize if unchallenged
        uint256 finalizeAtBlock;
        // number of open challenges
        uint64 nOpenChallenges;
    }

    struct OpenChallenge {
        uint64 epoch;
        address defender;
        address challenger;
    }

    // maps epoch number to settled finalized block roots
    // block roots in this map can be trusted
    // maintaining this map is the primary purpose of the protocol
    mapping(uint256 => bytes32) public trustedBlockRoots;

    // maps epoch number to pending submission
    mapping(uint256 => PendingSubmission) public pendingSubmissions;

    // maps challengeId to open challenges
    mapping(uint256 => OpenChallenge) public openChallenges;

    // The highest trusted checkpoint epoch
    // this can be used to index into trustedBlockRoots
    uint256 public highestTrusted;

    // The highest seen checkpoint epoch
    // this is the epoch for the most recent pending submission
    uint256 public highestSeen;

    // Contract that manages memory for the executor
    IExecutorIO public immutable io;
    // Untouched state transition code snapshot commitment
    // this essentially commits to the code this contract is fraud proving
    bytes32 public immutable goldenSnapshot;
    // Chain ID this instance of Zipline is tracking
    uint32 public immutable chainId;

    /// @notice Contract constructor which commits to the fraud proving code and how it is executed.
    /// It also requires an initial trusted checkpoint (which can be the genesis block)
    /// @param exec_ The contract that will execute the fraud proving code
    /// @param io_ The contract that manages memory for the executor
    /// @param goldenSnapshot_ A commitment to the fraud proving code in the form of a snapshot root
    /// @param trustedEpoch The epoch of the initial trusted checkpoint
    /// @param trustedBlockRoot The block root of the initial trusted checkpoint
    constructor(
        IExecutor exec_,
        IExecutorIO io_,
        bytes32 goldenSnapshot_,
        uint32 chainId_,
        uint64 trustedEpoch,
        bytes32 trustedBlockRoot
    ) CGOrchestrator(exec_, K) {
        io = io_;
        goldenSnapshot = goldenSnapshot_;
        chainId = chainId_;

        highestTrusted = trustedEpoch;
        highestSeen = trustedEpoch;
        trustedBlockRoots[trustedEpoch] = trustedBlockRoot;
    }

    /// @notice A permissionless function any account can call to submit a new checkpoint
    /// they claim was finalized by the origin chain.
    /// If unchallenged this Submission will finalize
    /// @dev In future this will require staking a bond
    /// @param epoch The epoch of the checkpoint being submitted
    /// @param blockRoot The block root of the checkpoint being submitted
    function submit(uint64 epoch, bytes32 blockRoot) external {
        require(epoch == highestSeen + 1, "Must be submitting a successor to the last submission");

        // derive a new unique identifier for this Submission
        uint256 finalizesAtBlock = block.number + FINALIZATION_TIME_BLOCKS;

        // check if there is already a submission at this epoch
        PendingSubmission storage p = pendingSubmissions[epoch];
        if (p.creator == address(0)) {
            // if no existing submission then create one
            p.creator = tx.origin;
            p.epoch = epoch;
            p.blockRoot = blockRoot;
            p.finalizeAtBlock = finalizesAtBlock;

            highestSeen = epoch;
        } else {
            revert(
                "Tried to submit at height with an existing submission. Challenge the existing one first then resubmit"
            );
        }
    }

    /// @notice Allows another account to challenge a pending checkpoint
    /// They challenge this by proving another checkpoint the claim was finalized by the origin chain
    /// at the same height as the pending checkpoint. Since only one can be correct the two submitters are entered into a challenge game
    /// @param epoch The epoch of the checkpoint being challenged
    /// @param rivalBlockRoot The alternative block root the challenger claims was finalized instead
    /// @param proofData This is an SSZ serialized Zipline input container
    /// @param finalSnapshot The final snapshot of the execution trace proving the rival block root has been finalized
    /// @param nSteps The number of instructions in the execution trace
    function challenge(
        uint64 epoch,
        bytes32 rivalBlockRoot,
        bytes calldata proofData,
        bytes32 finalSnapshot,
        uint256 nSteps
    ) external {
        PendingSubmission storage challengedSubmission = pendingSubmissions[epoch];
        require(
            block.number < challengedSubmission.finalizeAtBlock,
            "Submission has already reached finality. Call finalize"
        );

        // check that this final snapshot has terminated and validates to true
        require(
            io.readOutput(finalSnapshot) == bytes32(0x00), "Challengers final output must signal a correct verification"
        );

        // the rival checkpoint must be incompatible with the checkpoint it is challenging
        // i.e. it must be impossible for both to have been finalized by Casper FFG
        // In simple terms this just means they have the same epoch number but different block roots
        require(
            rivalBlockRoot != challengedSubmission.blockRoot,
            "Provided rival checkpoint does not conflict with the submision it is challenging"
        );

        // build the zipline input struct, hash it, and insert it the hash into the golden snapshot to get the start snapshot
        // need to enforce that:
        // - the trusted checkpoint is checkpoint at the previous epoch (either already trusted or optimistically trusted)
        // - the candidate checkpoint is the rival checkpoint passed to this function
        bytes32 trustedblockRoot = trustedBlockRoots[epoch - 1] | pendingSubmissions[epoch - 1].blockRoot;

        bytes32 inputHash = computeInputHash(epoch - 1, trustedblockRoot, epoch, rivalBlockRoot, proofData);
        bytes32 startSnapshot = io.writeInput(goldenSnapshot, inputHash);

        bytes32[2] memory startAndEndSnapshots;
        startAndEndSnapshots[0] = startSnapshot;
        startAndEndSnapshots[1] = finalSnapshot;

        uint64 id = newChallenge(
            startAndEndSnapshots,
            nSteps,
            tx.origin, // asserter is the one challenging the submission. They are asserting an alternative checkpoint at the same epoch
            challengedSubmission.creator // challenger is the author of the original Submission
        );
        // store in the open challenges mapping
        OpenChallenge storage c = openChallenges[id];
        c.challenger = tx.origin;
        c.defender = challengedSubmission.creator;
        c.epoch = epoch;
    }

    /// @notice Finalize a pending checkpoint
    /// @dev In future this will free up any bond associated with the submission
    /// @param epoch The epoch of the checkpoint being finalized
    function finalize(uint64 epoch) external {
        require(canFinalize(epoch), "Submission is not eligible to finalize");
        PendingSubmission storage p = pendingSubmissions[epoch];

        // write the pending state root to settled
        trustedBlockRoots[p.epoch] = p.blockRoot;
        highestTrusted = p.epoch;

        delete pendingSubmissions[epoch];
    }

    /// @notice Callback for when a challenge has been completed
    function challengeConcluded(uint256 challengeId, address winner, address loser) internal override {
        // find the Submission this is the challenge for
        OpenChallenge storage c = openChallenges[challengeId];
        if (c.challenger == winner && c.defender == loser) {
            SubmissionChallengeSuccess(c.epoch, challengeId);
        } else if (c.defender == winner && c.challenger == loser) {
            SubmissionChallengeFailed(c.epoch, challengeId);
        } else {
            revert("Winner and loser addresses from callback do not match open challenge!");
        }
    }

    ////////////////////////////////////////////////
    ///                 Helpers
    ////////////////////////////////////////////////

    /// @notice Take a proofData blob and check that is contains the given checkpoints (trusted+candidate)
    /// before returning its sha256 hash
    function computeInputHash(
        uint64 trustedEpoch,
        bytes32 trustedblockRoot,
        uint64 candidateEpoch,
        bytes32 candidateblockRoot,
        bytes calldata proofData
    ) public pure returns (bytes32) {
        // Check that the trused and candidate checkpoints passed as arguments
        // are present in the proofData in the correct location.
        // Do this by comparing the hash for efficiency
        bytes32 argDigest = keccak256(
            abi.encodePacked(
                SSZ.uint64ToLittleEndian(trustedEpoch),
                trustedblockRoot,
                SSZ.uint64ToLittleEndian(candidateEpoch),
                candidateblockRoot
            )
        );
        bytes32 proofDataDigest = keccak256(abi.encodePacked(proofData[:80]));
        require(
            argDigest == proofDataDigest, "proofData is not correctly formed for the checkpoints it is proving between"
        );

        return sha256(proofData);
    }

    // Called when a Submission was successfully proven to be fraudulent
    function SubmissionChallengeSuccess(uint64 epoch, uint256 challengeId) private {
        delete openChallenges[challengeId];
        // delete the submission and
        // also delete any submissions that build off this one
        for (uint256 i = 0; epoch + i <= highestSeen; i++) {
            delete pendingSubmissions[epoch + i];
        }
        highestSeen = epoch - 1; // wind it back to allow for a new submission
            // TODO: Slash defenders stake
    }

    /**
     *
     */
    function SubmissionChallengeFailed(uint64 epoch, uint256 challengeId) private {
        pendingSubmissions[epoch].nOpenChallenges -= 1;
        delete openChallenges[challengeId];
        // TODO: Slash challengers stake
    }

    /**
     * An Submission can finalize iff
     * - the current block >= to it's finalization block
     * - It does not have any open challenges
     */
    function canFinalize(uint64 epoch) private view returns (bool) {
        PendingSubmission storage p = pendingSubmissions[epoch];
        return p.finalizeAtBlock >= block.number && p.nOpenChallenges == 0;
    }
}
