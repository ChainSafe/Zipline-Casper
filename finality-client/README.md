# Zipline Finality Client

This crate implements a prototype stateless client capable of following the Casper-FFG finality protocol.

It is no-std compatible and is intended to run in resource (although not IO and memory) constrained environments such as provable execution and Substrate runtimes.

It's main function is `verify` accepts

- A trusted checkpoint
- A candidate checkpoint which can be trusted if the function returns true
- A StateReader able to give us read access into a trusted BeaconState
- A number of StatePatches which can patch this trusted state for future epochs
- A collection of attestations which should prove finality of the candidate

and is able to determine if the candidate checkpoint has been finalized given an already trusted checkpoint.

## Testing

Running the full test suite requires the [ethereum spec tests](https://github.com/ethereum/consensus-spec-tests).

To download them run the following from the root of the repo
```shell
cargo install just
just download-integration-tests
```

The tests also require some cached local files. Build these using

```shell
just cache-zipline-tests
```

## Implementation Details
### State Reader

Although the finality client is stateless it requires access to parts of the most recent beacon state when verifying finality. This is abstracted through the [`StateReader`](./src/state_reader.rs) trait. A concrete implementation of a finality client must provide some way to read these particular entries in the state.

The [`SszStateReader`](./src/ssz_state_reader.rs) is one implementation that is able to traverse an SSZ Merklized beacon state from the root down in order to retrieve the required entries. To do so it requires an oracle (hash addressed lookup table) for all leaves and intermediate nodes in the SSZ tree. This is particularly useful as the lookup table can be implemented as a pre-image oracle for provable computation or lookup tables in a SNARK.

### State Patches

The finality client introduces the idea of state patches for a beacon state. These are the fields that change between adjacent epochs required for checking attestations. These fields are:
- randao commitment
- entering validators
- exiting validators
- number of deposits processed

Using state patches allows starting at the state as of a trusted checkpoint and projecting forward in time to verify attestations in near future epochs.
## Testing

### Integration Tests

Tests are derived from the [ethereum consensus spec tests](https://github.com/ethereum/consensus-spec-tests/). There are a number of steps required to get the integration tests running:

1. Download the spec test: `just download-integration-tests`
2. Derive and cache the zipline tests from the spec tests: `just cache-zipline-tests`

Now it is possible to run the integration tests with

`cargo test -p zipline-finality-client`

It is highly reccomended to enable logging by setting the correct env vars and cargo test flags

`RUST_LOG=info cargo test test_finality_rule_3 -- --nocapture`
