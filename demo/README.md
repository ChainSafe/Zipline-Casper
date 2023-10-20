# Zipline Demo

This node script runs a demo that shows how the Zipline contract, off-chain emulation and challenge game can be used to revert an invalid checkpoint submitted by a fraudulent actor.

> Note that the emulation may take quite a while in the early stages (up to 5 minutes each step) as it has to emulate 15 trillion or so MIPS instructions. It gets much faster as the challenge game progresses and the segments become shorter. 

## Prerequisites

- Ensure `pkg-config` is installed (e.g. `brew install pkg-config` or `apt install pkg-config` or an alternative method)
- Install node (tested with v16 but any newer version should work)
- Install [Foundry](https://getfoundry.sh/)
- Install the contract dependencies
```shell
cd ../contracts
forge install
```
- Build the `zipline-state-transition-mips` binary for the spec test. Instructions here are for docker build. See [readme](../zipline-state-transition-mips/README.md) for alternatives
```shell
cd ../zipline-state-transition-mips
make docker_image # or make docker_image_apple_silicon if using a M1 or M2 processor
make build_spectest_spec
```
    - Alternatively you can download a pre-built binary from the repo (see the actions/build-artifacts) and copy it to `zipline-state-transition-mips/build/spec_test_out.bin`

- Build the emulator
```shell
cd ../emulator
cargo build --release
```
## Running the demo

1. In a separate terminal, Start an Ethereum testnet by running. Make sure to leave this running

```shell
anvil

```

2. Run the script from the `demo` directory

```shell
node demo.js
```

## Additional info

The script will look for a locally built copy of the Zipline MIPS binary in `zipline-state-transition-mips/build/spec_test_out.bin`. 

As building this can be challenging if it fails to detect a local build it will grab a recent one from the github artifacts page. The download size is ~500MB. 
