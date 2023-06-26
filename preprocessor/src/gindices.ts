/* eslint-disable @typescript-eslint/no-unused-vars */
import { Gindex } from "@chainsafe/persistent-merkle-tree";
import { EPOCHS_PER_HISTORICAL_VECTOR } from "@lodestar/params";

// ssz.phase0.BeaconState.getPathInfo(['validators'])
const VALIDATORS_ROOT_GINDEX = 43n;
// ssz.phase0.BeaconState.fields.validators.depth
const VALIDATORS_TREE_DEPTH = 41n;
// BigInt(2 ** VALIDATORS_TREE_DEPTH) * VALIDATORS_ROOT_GINDEX;
// Index of Validators[0]
const VALIDATORS_0_GINDEX = 94557999988736n;
// ssz.phase0.ValidatorContainer.getPathInfo(['activationEpoch']).gindex
const ACTIVATION_EPOCH_GINDEX = 13n;
// ssz.phase0.ValidatorContainer.getPathInfo(['exitEpoch']).gindex
const EXIT_EPOCH_GINDEX = 14n;
// ssz.phase0.ValidatorContainer.getPathInfo(['pubkey']).gindex
const PUBKEY_GINDEX = 8n;
// ssz.phase0.ValidatorContainer.getPathInfo(['effectiveBalance']).gindex
const EFFECTIVE_BALANCE_GINDEX = 10n;

/**
 * Concatenate Generalized Indices
 * Given generalized indices i1 for A -> B, i2 for B -> C .... i_n for Y -> Z, returns
 * the generalized index for A -> Z.
 */
function concatGindices(gindices: Gindex[]): Gindex {
  return BigInt(gindices.reduce((acc, gindex) => acc + gindex.toString(2).slice(1), "0b1"));
}

export function getValidatorGindices(validatorIndex: number): {
  activationEpoch: Gindex;
  exitEpoch: Gindex;
  pubkey: Gindex;
  balance: Gindex;
} {
  const validatorRootGindex = VALIDATORS_0_GINDEX + BigInt(validatorIndex);
  return {
    activationEpoch: concatGindices([validatorRootGindex, ACTIVATION_EPOCH_GINDEX]),
    exitEpoch: concatGindices([validatorRootGindex, EXIT_EPOCH_GINDEX]),
    pubkey: concatGindices([validatorRootGindex, PUBKEY_GINDEX]),
    balance: concatGindices([validatorRootGindex, EFFECTIVE_BALANCE_GINDEX]),
  };
}

export const VALIDATORS_LENGTH_GINDEX = VALIDATORS_ROOT_GINDEX * 2n + 1n;

// ssz.phase0.BeaconState.getPathInfo(['justificationBits']).gindex
export const JUSTIFICATION_BITS_GINDEX = 49n;

// ssz.phase0.BeaconState.getPathInfo(['previousJustifiedCheckpoint']).gindex
export const PREVIOUS_JUSTIFIED_CHECKPOINT_GINDEX = 50n;

// ssz.phase0.BeaconState.getPathInfo(['currentJustifiedCheckpoint']).gindex
export const CURRENT_JUSTIFIED_CHECKPOINT_GINDEX = 51n;

// ssz.phase0.BeaconState.getPathInfo(['finalizedCheckpoint']).gindex
export const FINALIZED_CHECKPOINT_GINDEX = 52n;

export function getCheckpointGindices(rootGindex: Gindex): { root: Gindex; epoch: Gindex } {
  return {
    epoch: rootGindex * 2n,
    root: rootGindex * 2n + 1n,
  };
}

// ssz.phase0.BeaconState.getPathInfo(['randaoMixes']).gindex
const RANDAO_MIXES_ROOT_GINDEX = 45n;
// ssz.phase0.BeaconState.fields.randaoMixes.depth
const RANDAO_MIXES_DEPTH = 16;
// ssz.phase0.BeaconState.getPathInfo(['randaoMixes', 0]).gindex
// OR
// BigInt(2 ** RANDAO_MIXES_DEPTH) * RANDAO_MIXES_ROOT_GINDEX
const RANDAO_MIXES_0_GINDEX = 2949120n;

export function getEpochRandaoMixGindex(epoch: number): Gindex {
  return RANDAO_MIXES_0_GINDEX + BigInt(epoch % EPOCHS_PER_HISTORICAL_VECTOR);
}

// Gets the Gindex of something in the state rooted in a Checkpoint.
// Transforms (BeaconState -> thing)
// To (Checkpoint -> BeaconBlockHeader -> BeaconState -> thing)
export function getCheckpointRootedGindex(stateRootedGindex: Gindex): Gindex {
  return concatGindices([
    // ssz.phase0.Checkpoint.getPathInfo(['root']).gindex
    3n,
    // ssz.phase0.BeaconBlockHeader.getPathInfo(['stateRoot']).gindex
    11n,
    stateRootedGindex,
  ]);
}
