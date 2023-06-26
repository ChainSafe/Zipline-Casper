# Containers

The containers described can be considered as an extension of the [Beacon chain specification](https://github.com/ethereum/consensus-specs/blob/dev/specs/phase0/beacon-chain.md). Types from the beacon chain spec are used here without redefinition.

## Zipline Input

This contains all the data required to prove the candidate checkpoint has been finalized given some trusted checkpoint. This is the input data type for the provable computation.

```python
class ZiplineInput(Container):
    state_root: Root,
    trusted_cp: Checkpoint,
    candidate_cp: Checkpoint,
    patches: List[StatePatch, MAX_PATCHES],
    attestations: List[Attestation, MAX_ATTESTATIONS],
    state_proof: List[Root, 3],
```

## State Patch

A state patch can be applied to a BeaconState to produce a new beacon state. The patch only contains the data relevant to computing the validator shuffling.

```python
class StatePatch(Container):
    epoch: Epoch, # epoch this patches up to. A single patch should only increment the epoch by 1
    activations: List[ValidatorIndex, MAX_ACTIVATIONS],
    exits: List[ValidatorIndex, MAX_EXITS],
    n_deposits_processed: uint32,
    randao_next: Bytes32, # randao value needed to compute the shuffling in the NEXT epoch
```