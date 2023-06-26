// Copyright 2022 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

pragma solidity ^0.8.17;

import "forge-std/Test.sol";
import {ZiplineAdapter, IZipline} from "../src/ZiplineAdapter.sol";

contract MockZipline is IZipline {
    uint32 public chainId;
    mapping(uint64 => bytes32) public trustedBlockRoots;

    constructor(uint32 _chainId) {
        chainId = _chainId;
    }

    function setTrustedBlockRoot(uint64 epoch, bytes32 root) external {
        trustedBlockRoots[epoch] = root;
    }
}

contract ZiplineAdapterTest is Test {
    ZiplineAdapter adapter;
    MockZipline zl;

    function setUp() public {
        zl = new MockZipline(1337);
        adapter = new ZiplineAdapter(address(zl));
    }

    function testIncorrectChainId() public {
        vm.expectRevert();
        adapter.storeBlockHeader(
            1338, // chain ID
            10, //epoch
            bytes32(0), // block header root
            new bytes32[](0), // ancestor proof
            0, // block number
            new bytes32[](0), // block number proof
            bytes32(0), // block hash
            new bytes32[](0) // block hash proof
        );
    }

    function testEpochBlockNotAvailable() public {
        vm.expectRevert();
        adapter.storeBlockHeader(
            1337, // chain ID
            10, //epoch
            bytes32(0), // block header root
            new bytes32[](0), // ancestor proof
            0, // block number
            new bytes32[](0), // block number proof
            bytes32(0), // block hash
            new bytes32[](0) // block hash proof
        );
    }

    function testCanStoreExecutionBlockFromEBB() public {
        zl.setTrustedBlockRoot(10, 0x40c2d2bb6a7c369dda94bd83eab4e65a7fc0285fec7ca005091888736a500440);

        // TODO: add the correct block number and block hash proofs
        // adapter.storeBlockHeader(
        //     1337, // chain ID
        //     10, //epoch
        //     bytes32(0x40c2d2bb6a7c369dda94bd83eab4e65a7fc0285fec7ca005091888736a500440), // block header root
        //     new bytes32[](0), // ancestor proof (not required)
        //     0, // block number
        //     new bytes32[](0), // block number proof
        //     bytes32(0), // block hash
        //     new bytes32[](0) // block hash proof
        // );
    }

    function testCanStoreExecutionBlockAncestor() public {
        zl.setTrustedBlockRoot(10, 0x40c2d2bb6a7c369dda94bd83eab4e65a7fc0285fec7ca005091888736a500440);

        // TODO: add the correct ancestor, block number and block hash proofs
        // adapter.storeBlockHeader(
        //     1337, // chain ID
        //     10, //epoch
        //     bytes32(0x40c2d2bb6a7c369dda94bd83eab4e65a7fc0285fec7ca005091888736a500440), // block header root
        //     new bytes32[](0), // ancestor proof (not required)
        //     0, // block number
        //     new bytes32[](0), // block number proof
        //     bytes32(0), // block hash
        //     new bytes32[](0) // block hash proof
        // );
    }

    function testAncestorProofSingleStep() public view {
        bytes32 childRoot = 0x40c2d2bb6a7c369dda94bd83eab4e65a7fc0285fec7ca005091888736a500440;
        bytes32 parentRoot = 0x66c0fcca790b43a7020d05f22d4a4a85d53d99726885917fd710193372411173;
        bytes32[] memory proof = new bytes32[](3);

        proof[0] = 0x86842cc3f8b3e17168acaa9f978812548a472103a62db5275813b92a0522c972;
        proof[1] = 0x08c3d47668c4f34da269152ac3327ffe0f9db3ff10789ffe7de2a7e649d81a7f;
        proof[2] = 0xfd977123a04523586bef3ee2f967a36fabb39f657cc9bb9aee01348ca8831a04;

        require(adapter.verifyAncestorProof(parentRoot, proof, childRoot) == true, "Failed to verify ancestor proof");
    }
}
