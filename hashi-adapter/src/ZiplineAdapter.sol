// Copyright 2022 ChainSafe Systems
// SPDX-License-Identifier: LGPL-3.0-only

pragma solidity ^0.8.17;

import {SSZ} from "hashi/adapters/Telepathy/libraries/SimpleSerialize.sol";
import {BlockHashOracleAdapter} from "hashi/adapters/BlockHashOracleAdapter.sol";

contract ZiplineAdapter is BlockHashOracleAdapter {
    error InvalidBlockNumberProof();
    error BlockHeaderNotAvailable(uint256 slot);
    error InvalidBlockHashProof();
    error InvalidAncestorProof();
    error InvalidChainId(uint32 chainId);

    IZipline public immutable zipline;

    constructor(address ziplineAddress) {
        zipline = IZipline(ziplineAddress);
    }

    /// @notice Stores the execution block hash for a given block
    /// if it can verify a proof rooted in a trusted Zipline beacon block root.
    /// Since Zipline only stores one beacon block root per epoch the SSZ proof must traverse backward
    /// throught the chain of beacon blocks to the one corresponding to the desired execution block
    /// @dev Due to very implementations of the underlying protocol this adapter is clostely based of TelepathyAdapter.sol
    /// @param _epoch The Zipline trusted beacon block to root the proof. Ideally should be the most recent after the desired block
    /// @param _blockHeaderRoot A block header root that is an ancestor of epoch boundry block (EBB) of the above epoch
    /// @param _ancestorProof Proof that the given block header root is a predessor of the trusted EBB
    /// @param _blockNumber The execution block number to store
    /// @param _blockNumberProof The SSZ proof of the block number rooted in the beacon block at the corresponding slot
    /// @param _blockHash The execution block hash to prove
    /// @param _blockHashProof The SSZ proof of the block hash rooted in the beacon block at the corresponding slot
    function storeBlockHeader(
        uint32 _chainId,
        uint64 _epoch,
        bytes32 _blockHeaderRoot,
        bytes32[] calldata _ancestorProof,
        uint256 _blockNumber,
        bytes32[] calldata _blockNumberProof,
        bytes32 _blockHash,
        bytes32[] calldata _blockHashProof
    ) external {
        if (zipline.chainId() != _chainId) {
            revert InvalidChainId(_chainId);
        }

        // retrieve the trusted epoch boundary block (EBB) root for the given epoch from Zipline
        bytes32 EBBHeaderRoot = zipline.trustedBlockRoots(_epoch);
        if (EBBHeaderRoot == bytes32(0)) {
            revert BlockHeaderNotAvailable(_epoch);
        }

        // if we are not proving the execution block for the EBB block then we also need to check the ancestor proof
        if (EBBHeaderRoot != _blockHeaderRoot) {
            if (!verifyAncestorProof(_blockHeaderRoot, _ancestorProof, EBBHeaderRoot)) {
                revert InvalidAncestorProof();
            }
        }

        // now the given beacon block header root has been proven to be in the chain, prove the execution block is included in it
        if (!SSZ.verifyBlockNumber(_blockNumber, _blockNumberProof, _blockHeaderRoot)) {
            revert InvalidBlockNumberProof();
        }

        if (!SSZ.verifyBlockHash(_blockHash, _blockHashProof, _blockHeaderRoot)) {
            revert InvalidBlockHashProof();
        }
        _storeHash(uint256(_chainId), _blockNumber, _blockHash);
    }

    /// @notice Verifies a SSZ proof that a given block header root is an ancestor of another
    /// @dev public so it can be tested
    /// @param _ancestorRoot The root of the ancestor block
    /// @param _ancestorProof The SSZ proof between the two beacon block roots
    /// @param _childRoot The root of the child block
    function verifyAncestorProof(bytes32 _ancestorRoot, bytes32[] calldata _ancestorProof, bytes32 _childRoot)
        public
        pure
        returns (bool)
    {
        uint256 proofLength = _ancestorProof.length;
        require(proofLength >= 3, "Proof to short even for a single chain step");
        require((proofLength + 1) % 4 == 0, "proof length not correct for proving between ancestors");
        uint256 steps = (proofLength + 1) / 4;

        // compute the gindex for an ancestor `steps` hops away in the chain
        uint256 gindex = 2 ** (steps * 3) + 2;

        return SSZ.isValidMerkleBranch(_ancestorRoot, gindex, _ancestorProof, _childRoot);
    }
}

interface IZipline {
    function trustedBlockRoots(uint64 epoch) external view returns (bytes32);
    function chainId() external view returns (uint32);
}
