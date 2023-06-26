// Copyright 2022 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

pragma solidity ^0.7.3;
pragma abicoder v2;

import "forge-std/Test.sol";

import {MockExecutor} from "./MockExecutor.sol";
import {CGOrchestrator} from "../src/CGOrchestrator.sol";

abstract contract MockPlayer {
    function addr() public view virtual returns (address);

    function getMaxDegree() internal virtual returns (uint256);

    function getSnapshot(uint256 index) internal view virtual returns (bytes32);

    function getNextChallenge(
        CGOrchestrator.FaultAssertion calldata priorSelection, // the data the previous actor used in their call
        bytes32[] calldata priorSegments, // same as above
        uint256 challengePosition // segment in prevSections this actor believes the fault is in
    ) external returns (CGOrchestrator.FaultAssertion memory newSelection, bytes32[] memory newSegments) {
        // the segmentation the prior actor was using
        (uint256 priorSegmentStart, uint256 priorSegmentLength) = extractChallengeSegment(priorSelection);

        // the challenge we want to make referencing the old one
        newSelection = CGOrchestrator.FaultAssertion(
            CGOrchestrator.Segmentation(CGOrchestrator.Slice(priorSegmentStart, priorSegmentLength), priorSegments),
            challengePosition
        );

        console.log("prior segment start: %s", priorSegmentStart);
        console.log("prior segment length: %s", priorSegmentLength);

        // new selection we are using to segment the trace
        (uint256 segmentStart, uint256 segmentLength) = extractChallengeSegment(newSelection);

        console.log("new segment start: %s", segmentStart);
        console.log("new segment length: %s", segmentLength);

        uint256 degree = segmentLength;
        if (degree > getMaxDegree()) {
            degree = getMaxDegree();
        }

        newSegments = new bytes32[](degree + 1);
        for (uint256 i = 0; i <= degree; i++) {
            uint256 oneSegmentLength = segmentLength / degree;

            uint256 snapshotIndex = segmentStart + i * oneSegmentLength;

            // this is a special case for the last segment. If the trace is not divisible by the
            // degree then add on the remainder
            if (i == degree) {
                snapshotIndex += segmentLength % degree;
            }

            newSegments[i] = getSnapshot(snapshotIndex);
        }
    }

    function extractChallengeSegment(CGOrchestrator.FaultAssertion memory assertion)
        internal
        pure
        returns (uint256 segmentStart, uint256 segmentLength)
    {
        uint256 oldChallengeDegree = assertion.prevSegmentation.snapshots.length - 1;
        segmentLength = assertion.prevSegmentation.span.length / oldChallengeDegree;
        // Intentionally done before challengeLength is potentially added to for the final segment
        segmentStart = assertion.prevSegmentation.span.start + segmentLength * assertion.challengePosition;

        // this is a special case for the last segment. If the trace is not divisible by the
        // degree then add the remainder into the last segment.
        if (assertion.challengePosition == assertion.prevSegmentation.snapshots.length - 2) {
            segmentLength += assertion.prevSegmentation.span.length % oldChallengeDegree;
        }
    }
}

contract HonestCountingPlayer is MockPlayer {
    uint256 degree;
    address add;

    constructor(address addr_, uint256 degree_) {
        add = addr_;
        degree = degree_;
    }

    function addr() public view override returns (address) {
        return add;
    }

    function getMaxDegree() internal view override returns (uint256) {
        return degree;
    }

    function getSnapshot(uint256 index) internal pure override returns (bytes32) {
        return bytes32(index); // just identity function in this case
    }
}

contract DishonestCountingPlayer is MockPlayer {
    address add;
    uint256 degree;
    uint256 stepIndex; // trace index after which participant will start lying

    constructor(address addr_, uint256 degree_, uint256 stepIndex_) {
        add = addr_;
        degree = degree_;
        stepIndex = stepIndex_;
    }

    function addr() public view override returns (address) {
        return add;
    }

    function getMaxDegree() internal view override returns (uint256) {
        return degree;
    }

    function getSnapshot(uint256 index) internal view override returns (bytes32) {
        if (index <= stepIndex) {
            return bytes32(index); // just identity function in this case
        } else {
            return bytes32(index + 10); // we lied and added 10 somewhere
        }
    }
}

contract MockPlayerTests is Test {
    function testGetNextChallengeBisection() public {
        // max degree 2
        MockPlayer p = new HonestCountingPlayer(vm.addr(1), 2);

        //////////////////////////////////////////////
        //       index: | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
        //  true trace: | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
        // segments       ^                       ^
        //////////////////////////////////////////////
        bytes32[] memory segments0 = new bytes32[](2);
        segments0[0] = 0;
        segments0[1] = bytes32(uint256(6));
        CGOrchestrator.FaultAssertion memory selection0 =
            CGOrchestrator.FaultAssertion(CGOrchestrator.Segmentation(CGOrchestrator.Slice(0, 6), segments0), 0);

        //////////////////////////////////////////////
        //       index: | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
        //  true trace: | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
        // segments       ^           ^           ^
        //////////////////////////////////////////////
        (CGOrchestrator.FaultAssertion memory selection1, bytes32[] memory segments1) =
            p.getNextChallenge(selection0, segments0, 0);
        assertEq(selection1.prevSegmentation.span.start, 0);
        assertEq(selection1.prevSegmentation.span.length, 6);
        assertEq(segments1.length, 3);
        assertEq(segments1[0], bytes32(uint256(0)));
        assertEq(segments1[1], bytes32(uint256(3)));
        assertEq(segments1[2], bytes32(uint256(6)));

        //////////////////////////////////////////////
        //       index: | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
        //  true trace: | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
        // segments                   ^   ^       ^
        //////////////////////////////////////////////
        (CGOrchestrator.FaultAssertion memory selection2, bytes32[] memory segments2) =
            p.getNextChallenge(selection1, segments1, 1);
        assertEq(selection2.prevSegmentation.span.start, 0);
        assertEq(selection2.prevSegmentation.span.length, 6);
        assertEq(segments2.length, 3);
        assertEq(segments2[0], bytes32(uint256(3)));
        assertEq(segments2[1], bytes32(uint256(4)));
        assertEq(segments2[2], bytes32(uint256(6)));

        //////////////////////////////////////////////
        //       index: | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
        //  true trace: | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
        // segments                       ^   ^   ^
        //////////////////////////////////////////////
        (CGOrchestrator.FaultAssertion memory selection3, bytes32[] memory segments3) =
            p.getNextChallenge(selection2, segments2, 1);
        assertEq(selection3.prevSegmentation.span.start, 3);
        assertEq(selection3.prevSegmentation.span.length, 3);
        assertEq(segments3.length, 3);
        assertEq(segments3[0], bytes32(uint256(4)));
        assertEq(segments3[1], bytes32(uint256(5)));
        assertEq(segments3[2], bytes32(uint256(6)));
    }

    function testGetNextChallengeTrisection() public {
        // max degree 3
        MockPlayer p = new HonestCountingPlayer(vm.addr(1), 3);

        //////////////////////////////////////////////
        //       index: | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
        //  true trace: | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
        // segments       ^                       ^
        //////////////////////////////////////////////
        bytes32[] memory segments0 = new bytes32[](2);
        segments0[0] = 0;
        segments0[1] = bytes32(uint256(6));
        CGOrchestrator.FaultAssertion memory selection0 =
            CGOrchestrator.FaultAssertion(CGOrchestrator.Segmentation(CGOrchestrator.Slice(0, 6), segments0), 0);

        //////////////////////////////////////////////
        //       index: | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
        //  true trace: | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
        // segments       ^       ^       ^       ^
        //////////////////////////////////////////////
        (CGOrchestrator.FaultAssertion memory selection1, bytes32[] memory segments1) =
            p.getNextChallenge(selection0, segments0, 0);
        assertEq(selection1.prevSegmentation.span.start, 0);
        assertEq(selection1.prevSegmentation.span.length, 6);
        assertEq(segments1.length, 4);
        assertEq(segments1[0], bytes32(uint256(0)));
        assertEq(segments1[1], bytes32(uint256(2)));
        assertEq(segments1[2], bytes32(uint256(4)));
        assertEq(segments1[3], bytes32(uint256(6)));

        //////////////////////////////////////////////
        //       index: | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
        //  true trace: | 0 | 1 | 2 | 3 | 4 | 5 | 6 |
        // segments                       ^   ^   ^
        //////////////////////////////////////////////
        (CGOrchestrator.FaultAssertion memory selection2, bytes32[] memory segments2) =
            p.getNextChallenge(selection1, segments1, 2);
        assertEq(selection2.prevSegmentation.span.start, 0);
        assertEq(selection2.prevSegmentation.span.length, 6);
        assertEq(segments2.length, 3); // degree changed cos we ran out of trace
        assertEq(segments2[0], bytes32(uint256(4)));
        assertEq(segments2[1], bytes32(uint256(5)));
        assertEq(segments2[2], bytes32(uint256(6)));
    }
}
