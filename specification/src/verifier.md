# Zipline Verifier

The verifier wraps the finality client in a way that can be executed within Cannon. It also abstracts the reading of state data in a way that is compatible with the cannon pre-image oracle. 

## State Reader

The state reader for the Zipline verifier relies on the pre-image oracle and SSZ Merklization to make state access both memory efficient and cheap. Only the required 32 bytes of the state is loaded at any one time. The [SszStateReader](https://github.com/ChainSafe/Zipline/blob/main/finality-client/src/ssz_state_reader.rs) illustrates how this strategy is used to retrieve the required state chunks.

## Building for baremetal MIPS target

Unlike Optimism which uses Golang for its provable execution Zipline uses Rust. This requires quite a complex build system in order to support the Cannon environment. Custom implementations of syscalls are added to support the special features of the Cannon host environment such notifying the host of successful completion or requesting data via the preimage oracle. 

The build system also converts the output of `rustc` which is an elf executable into a linear MIPS memory array which includes the program as well as the zero initialized stack and heap. This binary file is loaded directly into the MIPS emulator as its initial memory. 

