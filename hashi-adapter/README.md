# Zipline Hashi Adapter

This adapter allows Zipline to provide trusted execution block roots to the [Hashi](https://github.com/gnosis/hashi) EVM block oracle aggregator

## Overview

Since Hashi expects to be able to request *execution* block roots at any height and Zipline only provides *beacon* block roots at epoch boundaries some additional proofs must be submitted along with the requests to store.

The first proof is required to prove that the beacon block containing the root of the desired execution block is an ancestor of a given trusted epoch boundary block. Assuming the chain is finalizing every epoch this ancestor will be at most one epoch away. An SSZ proof is used to prove the ancestry.

The second part is proving that this ancestor contains a reference to the given execution block root and that it is at the given height. These are identical to the proofs used in the [Telepathy Adapter](https://github.com/gnosis/hashi/blob/main/packages/evm/contracts/adapters/Telepathy/TelepathyAdapter.sol).

### Future Work

Currently an instance of the Zipline contract only manages a single Chain ID. The adapter could be enhanced to support multiple instances for different chains.

Verifying large ancestry proofs could make it expensive to submit Zipline blocks to Hashi. This could be eliminated by slightly altering how the Zipline protocol operates so that these proofs could be done off-chain in the fraud proof.

## Dependencies

- Install [Foundry](https://getfoundry.sh/) 

## Developer Quickstart

```shell
forge install
```

## Testing

```shell
forge test
```
