// Copyright 2022 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

pragma solidity ^0.7.3;
pragma abicoder v2;

import "forge-std/Test.sol";
import {Zipline} from "../src/Zipline.sol";
import {MockExecutor, MockExecutorIO} from "./MockExecutor.sol";

contract ZiplineTest is Test {
    Zipline zl;

    function setUp() public {
        zl = new Zipline(
        	new MockExecutor(),
            new MockExecutorIO(),
            0x0,
            0,
            0,
            0x0
        );
    }

    function testComputeInputHash() public view {
        zl.computeInputHash(
            3,
            bytes32(0x844ab2b1635933451fa5a665d0ae5f52cbac8d79ec2fdf2853d22cd7ec18f5f4),
            4,
            bytes32(0x9d01f76f5069fafe06d274db116c1667d0b8cecccd2ec4ce8ad35cedfb976946),
            hex"0300000000000000844ab2b1635933451fa5a665d0ae5f52cbac8d79ec2fdf2853d22cd7ec18f5f404000000000000009d01f76f5069fafe06d274db116c1667d0b8cecccd2ec4ce8ad35cedfb9769463516966ec07887e730509344c59c92e8"
        );
    }
}
