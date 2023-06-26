import { Api } from "@lodestar/api";
import { ssz } from "@lodestar/types";
import { SignedBeaconBlock } from "@lodestar/types/allForks";
import { Attestation, BeaconBlockHeader, Checkpoint } from "@lodestar/types/phase0";
import { toHex, toHexString } from "@lodestar/utils";
import { ContainerType, ListBasicType, UintNumberType, ValueOf, byteArrayEquals } from "@chainsafe/ssz";
import {
  BeaconStateAllForks,
  blockToHeader,
  computeEpochAtSlot,
  computeStartSlotAtEpoch,
  createCachedBeaconState,
  getCurrentSlot,
  processSlots,
  PubkeyIndexMap,
  stateTransition,
  getRandaoMix,
} from "@lodestar/state-transition";
import { EPOCHS_PER_HISTORICAL_VECTOR, MIN_SEED_LOOKAHEAD, SLOTS_PER_EPOCH } from "@lodestar/params";
import { BeaconConfig } from "@lodestar/config";
import { getBlockHeader, getBlocks, getState } from "./api.js";
import { log } from "./logger.js";
import path from "node:path";
import { writeFile } from "node:fs/promises";
// Copied from https://github.com/ChainSafe/Zipline/blob/f37d2c5c7be81a5ae6c203c9c60e8dba0968e017/zipline-verifier/src/state_patch.rs#L15
const MAX_ACTIVATIONS = 256;
const MAX_EXITS = 256;

const U32 = new UintNumberType(4);

export const StatePatchType = new ContainerType(
  {
    epoch: ssz.Epoch,
    activations: new ListBasicType(U32, MAX_ACTIVATIONS),
    exits: new ListBasicType(U32, MAX_EXITS),
    nDepositsProcessed: U32,
    randaoNext: ssz.Root,
  },
  { typeName: "StatePatch", jsonCase: "eth2", cachePermanentRootStruct: true }
);

export type StatePatch = ValueOf<typeof StatePatchType>;

/**
 * Create a patch from state A to state B
 */
export function createStatePatch(stateA: BeaconStateAllForks, stateB: BeaconStateAllForks): StatePatch {
  const prevEpoch = computeEpochAtSlot(stateA.slot);
  const epoch = computeEpochAtSlot(stateB.slot);
  if (prevEpoch !== epoch - 1) {
    throw new Error("State B must be one epoch ahead of state A");
  }
  const activations = [];
  const exits = [];
  for (let i = 0; i < stateB.validators.length; i++) {
    const validatorAtB = stateB.validators.get(i);
    if (validatorAtB.activationEpoch === epoch) {
      activations.push(i);
    }
    if (validatorAtB.exitEpoch === epoch) {
      exits.push(i);
    }
  }
  const nDepositsProcessed = stateB.validators.length - stateA.validators.length;
  const randaoIndex = epoch + EPOCHS_PER_HISTORICAL_VECTOR - MIN_SEED_LOOKAHEAD;
  const randaoNext = getRandaoMix(stateB, randaoIndex);

  log.info(
    `Created state patch epoch ${epoch}, activations ${activations}, exits ${exits}, ndeposits ${nDepositsProcessed}, randaoNext ${randaoNext} }`
  );

  return {
    epoch,
    activations,
    exits,
    nDepositsProcessed,
    randaoNext,
  };
}

/**
 * Find the next finalized checkpoint, starting from a trusted checkpoint
 */
export async function getNextFinalizedData(
  directory: string,
  api: Api,
  config: BeaconConfig,
  trustedCheckpoint: Checkpoint,
  trustedState?: BeaconStateAllForks
): Promise<{
  checkpoint: Checkpoint;
  header: BeaconBlockHeader;
  state: BeaconStateAllForks;
  attestations: Attestation[];
  statePatches: StatePatch[];
  trustedState: BeaconStateAllForks;
  trustedHeader: BeaconBlockHeader;
}> {
  log.info("Create chain finality proof");
  // get state root at checkpoint
  if (!trustedState) {
    const header = await getBlockHeader(api, toHexString(trustedCheckpoint.root));
    const state = config
      .getForkTypes(header.slot)
      .BeaconState.deserialize(await getState(api, toHex(header.stateRoot), "ssz"));
    trustedState = config.getForkTypes(state.slot).BeaconState.toViewDU(state);
    log.info("writing to file");
    await writeFile(path.join(directory, `state${trustedCheckpoint.epoch}`), trustedState.serialize());
  }

  const start = Date.now();
  log.info("Start createCachedBeaconState");
  const trustedStateCached = createCachedBeaconState(trustedState, {
    config,
    pubkey2index: new PubkeyIndexMap(),
    index2pubkey: [],
  });
  const millis = Date.now() - start;

  log.info(`End createCachedBeaconState ${Math.floor(millis / 1000)}s`);

  // apply blocks to state, epoch by epoch, until either
  // - state.finalizedCheckpoint.epoch > trustedCheckpoint.epoch (chain has finalized)
  // - slot >= clockSlot (chain has not finalized)
  const clockSlot = getCurrentSlot(config, trustedState.genesisTime);
  let state = trustedStateCached;
  const allBlocks: SignedBeaconBlock[] = [];
  const possibleFinalizedStates = [];
  let finalized = false;
  let stateA = trustedStateCached;
  const statePatches = [];
  for (let slot = computeStartSlotAtEpoch(trustedCheckpoint.epoch) + 1; slot < clockSlot; slot += SLOTS_PER_EPOCH) {
    const nextEpochSlot = slot + SLOTS_PER_EPOCH;
    log.info(`Getting blocks for slot ${slot} to ${nextEpochSlot}`);
    const blocks = await getBlocks(api, slot, nextEpochSlot);
    log.info("Finished getting blocks");
    for (const block of blocks) {
      log.info("Run stateTransition", state.slot);
      state = stateTransition(state, block);
    }
    allBlocks.push(...blocks);

    let stateB = state;
    // dial forward an epoch if there's a skip slot on the epoch boundary
    if (computeEpochAtSlot(stateB.slot) !== computeEpochAtSlot(stateA.slot) + 1) {
      stateB = processSlots(stateB, nextEpochSlot);
    }

    possibleFinalizedStates.push(stateB.clone(true));

    const patch = createStatePatch(stateA, stateB);
    statePatches.push(patch);
    stateA = stateB;

    if (stateB.finalizedCheckpoint.epoch > trustedCheckpoint.epoch) {
      finalized = true;
      break;
    }
  }
  if (!finalized) {
    throw new Error("No new finalized state");
  }

  const finalizedBlock = allBlocks.find((block) =>
    byteArrayEquals(
      config.getForkTypes(block.message.slot).BeaconBlock.hashTreeRoot(block.message),
      state.finalizedCheckpoint.root
    )
  );
  if (!finalizedBlock) {
    throw new Error("No finalized block found");
  }

  const finalizedCheckpoint = state.finalizedCheckpoint;
  const finalizedHeader = blockToHeader(config, finalizedBlock.message);
  const finalizedState = possibleFinalizedStates.find((state) =>
    ssz.Root.equals(state.hashTreeRoot(), finalizedBlock.message.stateRoot)
  );

  if (!finalizedState) {
    throw new Error("No finalized state found");
  }
  finalizedState.commit();

  return {
    checkpoint: finalizedCheckpoint,
    header: finalizedHeader,
    state: finalizedState,
    attestations: allBlocks.flatMap((block) => block.message.body.attestations),
    statePatches,
    trustedState: trustedState,
    trustedHeader: await getBlockHeader(api, toHexString(trustedCheckpoint.root)),
  };
}
