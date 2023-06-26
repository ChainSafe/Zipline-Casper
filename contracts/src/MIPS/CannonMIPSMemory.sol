// SPDX-License-Identifier: MIT
pragma solidity ^0.7.3;

import "./lib/Lib_MerkleTrie.sol";
import {Lib_BytesUtils} from "./lib/Lib_BytesUtils.sol";

contract CannonMIPSMemory {
    function AddTrieNode(bytes calldata anything) public {
        Lib_MerkleTrie.GetTrie()[keccak256(anything)] = anything;
    }

    struct Preimage {
        uint64 length;
        mapping(uint256 => uint64) data;
    }

    mapping(bytes32 => Preimage) public preimage;

    function MissingPreimageRevert(bytes32 outhash, uint256 offset) internal pure {
        Lib_BytesUtils.revertWithHex(abi.encodePacked(outhash, offset));
    }

    function GetPreimageLength(bytes32 outhash) public view returns (uint32) {
        uint64 data = preimage[outhash].length;
        if (data == 0) {
            MissingPreimageRevert(outhash, 0);
        }
        return uint32(data);
    }

    function GetPreimage(bytes32 outhash, uint256 offset) public view returns (uint32) {
        uint64 data = preimage[outhash].data[offset];
        if (data == 0) {
            MissingPreimageRevert(outhash, offset);
        }
        return uint32(data);
    }

    /// @dev Important to note this has been changed from the original Cannon implementation
    /// to use SHA256 hashing instead of Keccak256. This is because SSZ Merklization uses
    /// Sha256 and we must be consistent with that to be able to traverse the SSZ tree nodes
    function AddPreimage(bytes calldata anything, uint256 offset) public {
        require(offset & 3 == 0, "offset must be 32-bit aligned");
        uint256 len = anything.length;
        require(offset < len, "offset can't be longer than input");
        Preimage storage p = preimage[sha256(anything)];
        require(p.length == 0 || uint32(p.length) == len, "length is somehow wrong");
        p.length = (1 << 32) | uint64(uint32(len));
        p.data[offset] = (1 << 32) | ((len <= (offset + 0) ? 0 : uint32(uint8(anything[offset + 0]))) << 24)
            | ((len <= (offset + 1) ? 0 : uint32(uint8(anything[offset + 1]))) << 16)
            | ((len <= (offset + 2) ? 0 : uint32(uint8(anything[offset + 2]))) << 8)
            | ((len <= (offset + 3) ? 0 : uint32(uint8(anything[offset + 3]))) << 0);
    }

    function tb(uint32 dat) internal pure returns (bytes memory) {
        bytes memory ret = new bytes(4);
        ret[0] = bytes1(uint8(dat >> 24));
        ret[1] = bytes1(uint8(dat >> 16));
        ret[2] = bytes1(uint8(dat >> 8));
        ret[3] = bytes1(uint8(dat >> 0));
        return ret;
    }

    function fb(bytes memory dat) internal pure returns (uint32) {
        require(dat.length == 4, "wrong length value");
        uint32 ret = uint32(uint8(dat[0])) << 24 | uint32(uint8(dat[1])) << 16 | uint32(uint8(dat[2])) << 8
            | uint32(uint8(dat[3]));
        return ret;
    }

    function fbo(bytes memory dat, uint256 offset) internal pure returns (uint32) {
        uint32 ret = uint32(uint8(dat[offset + 0])) << 24 | uint32(uint8(dat[offset + 1])) << 16
            | uint32(uint8(dat[offset + 2])) << 8 | uint32(uint8(dat[offset + 3]));
        return ret;
    }

    function WriteMemory(bytes32 stateHash, uint32 addr, uint32 value) public returns (bytes32) {
        require(addr & 3 == 0, "write memory must be 32-bit aligned");
        return Lib_MerkleTrie.update(tb(addr >> 2), tb(value), stateHash);
    }

    function WriteBytes32(bytes32 stateHash, uint32 addr, bytes32 val) public returns (bytes32) {
        for (uint32 i = 0; i < 32; i += 4) {
            uint256 tv = uint256(val >> (224 - (i * 8)));
            stateHash = WriteMemory(stateHash, addr + i, uint32(tv));
        }
        return stateHash;
    }

    // TODO: refactor writeMemory function to not need these
    event DidStep(bytes32 stateHash);

    function WriteMemoryWithReceipt(bytes32 stateHash, uint32 addr, uint32 value) public {
        bytes32 newStateHash = WriteMemory(stateHash, addr, value);
        emit DidStep(newStateHash);
    }

    function WriteBytes32WithReceipt(bytes32 stateHash, uint32 addr, bytes32 value) public {
        bytes32 newStateHash = WriteBytes32(stateHash, addr, value);
        emit DidStep(newStateHash);
    }

    // needed for preimage oracle
    function ReadBytes32(bytes32 stateHash, uint32 addr) public view returns (bytes32) {
        uint256 ret = 0;
        for (uint32 i = 0; i < 32; i += 4) {
            ret <<= 32;
            ret |= uint256(ReadMemory(stateHash, addr + i));
        }
        return bytes32(ret);
    }

    function ReadMemory(bytes32 stateHash, uint32 addr) public view returns (uint32) {
        require(addr & 3 == 0, "read memory must be 32-bit aligned");

        // zero register is always 0
        if (addr == 0xc0000000) {
            return 0;
        }

        // MMIO preimage oracle
        if (addr >= 0x31000000 && addr < 0x32000000) {
            bytes32 pihash = ReadBytes32(stateHash, 0x30001000);
            if (pihash == sha256("")) {
                // both the length and any data are 0
                return 0;
            }
            if (addr == 0x31000000) {
                return uint32(GetPreimageLength(pihash));
            }
            return GetPreimage(pihash, addr - 0x31000004);
        }

        bool exists;
        bytes memory value;
        (exists, value) = Lib_MerkleTrie.get(tb(addr >> 2), stateHash);

        if (!exists) {
            // this is uninitialized memory
            return 0;
        } else {
            return fb(value);
        }
    }
}
