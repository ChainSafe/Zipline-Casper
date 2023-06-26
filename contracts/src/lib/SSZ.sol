// Copyright 2022 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

pragma solidity ^0.7.3;

library SSZ {
    function uint64ToLittleEndian(uint64 n) internal pure returns (bytes8) {
        n = ((n & 0xFF00FF00FF00FF00) >> 8) | ((n & 0x00FF00FF00FF00FF) << 8);
        n = ((n & 0xFFFF0000FFFF0000) >> 16) | ((n & 0x0000FFFF0000FFFF) << 16);
        n = (n >> 32) | (n << 32);
        return bytes8(n);
    }
}
