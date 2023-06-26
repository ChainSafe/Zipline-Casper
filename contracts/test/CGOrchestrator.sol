// Copyright 2022 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

pragma solidity ^0.7.3;
pragma abicoder v2;

import "forge-std/Test.sol";
import {CGOrchestrator} from "../src/CGOrchestrator.sol";
import {MockExecutor} from "./MockExecutor.sol";
import {MockPlayer, HonestCountingPlayer, DishonestCountingPlayer} from "./MockPlayer.sol";

contract CGOrchestratorTest is Test, CGOrchestrator {
    address winner;
    address loser;

    constructor() CGOrchestrator(new MockExecutor(), 2) {}

    function setUp() public {
        winner = address(0x0);
        loser = address(0x0);
    }

    function testDefenderWinsChallenge() public {
        // This is the actor who must prove the initial start/end snapshots
        // bound a valid execution trace. They win if either:
        // - The challenger stops responding
        // - The protocol reduces to a single step execution which is correct
        MockPlayer defender = new HonestCountingPlayer(vm.addr(1), 2);
        // The actor trying to prove the initial execution trace contains a fault
        // They win if either
        // - The defender stops responding
        // - The protocol reduces to a single step execution which is incorrect
        MockPlayer challenger = new DishonestCountingPlayer(vm.addr(2), 2, 5);

        // Bounds of an execution trace
        // This is the add-one trace, it is non-fraudulent if
        // each snapshot increment the previous one

        //////////////////////////////////////////////
        //       index: | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
        // fraud trace: | 0 | 1 | 2 | 3 | 4 | 5 |*7*|
        //  true trace: | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
        // segments       ^                       ^
        //////////////////////////////////////////////

        bytes32[2] memory startAndEndSnapshots;
        startAndEndSnapshots[0] = bytes32(0);
        startAndEndSnapshots[1] = bytes32(uint256(6));

        uint64 id = newChallenge(
            startAndEndSnapshots,
            6, //nSteps in execution trace
            defender.addr(),
            challenger.addr()
        );

        assertEq(id, uint256(1));

        Challenge memory challenge = challenges[id];

        // it immedietly becomes the challegers turn
        // The creation of the challenge is essentially the defenders turn
        assertEq(challenge.current.addr, challenger.addr());
        assertEq(challenge.next.addr, defender.addr());
        assertEq(challenge.active, true);

        //////////////////////
        // Challengers Turn (turn 1)
        //////////////////////

        // challenger recovers data from the prior segmentation to pass back to dissect

        bytes32[] memory turn0Segments = new bytes32[](2);
        turn0Segments[0] = startAndEndSnapshots[0];
        turn0Segments[1] = startAndEndSnapshots[1];

        FaultAssertion memory selection1 = FaultAssertion(
            Segmentation(
                Slice(
                    0, // prior segments start (0 for first segmentation)
                    6
                ), // prior segments length in ops (nSteps for first segmentation)
                turn0Segments
            ),
            0 // challengePosition (which segment we believe the challenge is in). The first only has one segment so we have to use 0
        );

        //////////////////////////////////////////////
        //       index: | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
        // fraud trace: | 0 | 1 | 2 | 3 | 4 | 5 |*7*|
        //  true trace: | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
        // segments       ^           ^           ^
        //////////////////////////////////////////////
        vm.prank(challenger.addr(), challenger.addr());
        (, bytes32[] memory turn1Segments) = challenger.getNextChallenge(selection1, turn0Segments, 0);

        vm.prank(challenger.addr(), challenger.addr());
        this.dissectChallenge(id, selection1, turn1Segments);

        // //////////////////////
        // // Defenders Turn (turn 2)
        // //////////////////////

        //////////////////////////////////////////////
        //       index: | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
        // fraud trace: | 0 | 1 | 2 | 3 | 4 | 5 |*7*|
        //  true trace: | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
        // segments                   ^   ^       ^
        //////////////////////////////////////////////
        vm.prank(defender.addr(), defender.addr());
        (FaultAssertion memory selection2, bytes32[] memory turn2Segments) =
            defender.getNextChallenge(selection1, turn1Segments, 1);

        vm.prank(defender.addr(), defender.addr());
        this.dissectChallenge(id, selection2, turn2Segments);

        //////////////////////
        // Challengers Turn (turn 3)
        //////////////////////

        //////////////////////////////////////////////
        //       index: | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
        // fraud trace: | 0 | 1 | 2 | 3 | 4 | 5 |*7*|
        //  true trace: | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
        // segments                       ^   ^   ^
        //////////////////////////////////////////////

        vm.prank(challenger.addr(), challenger.addr());
        (FaultAssertion memory selection3, bytes32[] memory turn3Segments) =
            challenger.getNextChallenge(selection2, turn2Segments, 1);

        vm.prank(challenger.addr(), challenger.addr());
        this.dissectChallenge(id, selection3, turn3Segments);

        //////////////////////
        // Defenders Turn (final turn)
        //////////////////////

        vm.prank(defender.addr(), defender.addr());
        (FaultAssertion memory finalSelection,) = defender.getNextChallenge(selection3, turn3Segments, 1);

        vm.prank(defender.addr(), defender.addr());
        this.proveChallenge(id, finalSelection);

        //////////////////////
        // Conclusion
        //////////////////////

        // we should now have proven fraud on the part of the challenger
        // this has effectively locked the challenge so it must now timeout
        // check it is locked (e.g. the gameCommitmentHash is zeroed)
        assertEq(challenges[id].gameCommitmentHash, bytes32(0));

        // wait for the timeout and trigger the timeout call
        vm.roll(100);
        timeoutChallenge(id);

        // This should trigger the callback and set our winners and losers
        assertEq(winner, defender.addr());
        assertEq(loser, challenger.addr());
    }

    ///////////////////////////////////////////////////////////////////////////
    // Test Helpers
    ///////////////////////////////////////////////////////////////////////////

    // Callback for when a challenge is completed
    // Use to update local contract state for checking in tests
    function challengeConcluded(uint256, address winner_, address loser_) internal override {
        winner = winner_;
        loser = loser_;
    }

    function range(uint256 start, uint256 end) private pure returns (bytes32[] memory result) {
        result = new bytes32[](end - start);
        for (uint256 i = 0; i < result.length; i++) {
            result[i] = bytes32(start + i);
        }
    }
}
