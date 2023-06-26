import path from "node:path";
import fs from "node:fs/promises";
import { createWriteStream } from "node:fs";
import { Gindex, Tree, Node, convertGindexToBitstring, GindexBitstring } from "@chainsafe/persistent-merkle-tree";
import { ChainForkConfig } from "@lodestar/config";
import { Epoch, phase0, ssz } from "@lodestar/types";
import {
  CURRENT_JUSTIFIED_CHECKPOINT_GINDEX,
  FINALIZED_CHECKPOINT_GINDEX,
  getCheckpointGindices,
  getEpochRandaoMixGindex,
  getValidatorGindices,
  JUSTIFICATION_BITS_GINDEX,
  PREVIOUS_JUSTIFIED_CHECKPOINT_GINDEX,
  VALIDATORS_LENGTH_GINDEX,
} from "./gindices.js";
import { log } from "./logger.js";
import { BeaconStateAllForks } from "@lodestar/state-transition";
import { EPOCHS_PER_HISTORICAL_VECTOR, MIN_SEED_LOOKAHEAD } from "@lodestar/params";
// String is a hex string
export type PreimageMap = Map<string, Uint8Array>;

function addTreePreimages(rootNode: Node, gindices: Iterable<Gindex>, preimages?: PreimageMap): PreimageMap {
  preimages = preimages ?? new Map();
  const tree = new Tree(rootNode);
  preimages.set(
    Buffer.from(tree.rootNode.root).toString("hex"),
    Buffer.concat([tree.rootNode.left.root, tree.rootNode.right.root])
  );
  for (const gindex of gindices) {
    const node = tree.getNode(gindex);
    if (node.isLeaf()) {
      // ignore for now - the preimage of a leaf is the leaf itself
    } else {
      preimages.set(Buffer.from(node.root).toString("hex"), Buffer.concat([node.left.root, node.right.root]));
    }
  }
  return preimages;
}

function toGindex(bitstring: GindexBitstring): Gindex {
  return BigInt("0b" + bitstring);
}

function addPathGindices(gindex: Gindex, gindices?: Set<Gindex>): Set<Gindex> {
  gindices = gindices ?? new Set<Gindex>();
  let bitstring = convertGindexToBitstring(gindex);
  for (let i = bitstring.length; i >= 1; i--) {
    gindices.add(toGindex(bitstring));
    bitstring = bitstring.slice(0, i);
  }
  return gindices;
}

function addPathsGindices(gindices: Gindex[], pathGindices?: Set<Gindex>): Set<Gindex> {
  pathGindices = pathGindices ?? new Set<Gindex>();
  for (const gindex of gindices) {
    addPathGindices(gindex, pathGindices);
  }
  return pathGindices;
}

function getZiplineGindices(
  validatorLength: number,
  epoch: Epoch
): {
  checkpoint: Set<Gindex>;
  header: Set<Gindex>;
  state: Set<Gindex>;
} {
  const checkpoint = new Set<Gindex>();
  log.info("getting checkpoint gindices");
  addPathGindices(ssz.phase0.Checkpoint.getPathInfo(["root"]).gindex, checkpoint);

  const header = new Set<Gindex>();
  log.info("getting header gindices");
  addPathGindices(ssz.phase0.BeaconBlockHeader.getPathInfo(["stateRoot"]).gindex, header);

  const state = new Set<Gindex>();
  // - for each validator
  //   - pubkey
  //   - activationEpoch
  //   - exitEpoch
  // - randao
  log.info("getting validators gindices");
  for (let i = 0; i < validatorLength; i++) {
    const { pubkey, activationEpoch, exitEpoch, balance } = getValidatorGindices(i);

    addPathsGindices([pubkey, activationEpoch, exitEpoch, balance], state);
  }
  const randaoIndex = epoch + EPOCHS_PER_HISTORICAL_VECTOR - MIN_SEED_LOOKAHEAD;
  log.info("getting other gindices");
  addPathsGindices(
    [
      VALIDATORS_LENGTH_GINDEX,
      JUSTIFICATION_BITS_GINDEX,
      ...Object.values(getCheckpointGindices(PREVIOUS_JUSTIFIED_CHECKPOINT_GINDEX)),
      ...Object.values(getCheckpointGindices(CURRENT_JUSTIFIED_CHECKPOINT_GINDEX)),
      ...Object.values(getCheckpointGindices(FINALIZED_CHECKPOINT_GINDEX)),
      getEpochRandaoMixGindex(epoch),
      getEpochRandaoMixGindex(epoch + 1),
      getEpochRandaoMixGindex(randaoIndex),
    ],
    state
  );

  return {
    checkpoint,
    header,
    state,
  };
}

/**
 * Get all data required for shufflings and signature aggregation
 */
export function getZiplinePreimages(
  config: ChainForkConfig,
  checkpoint: phase0.Checkpoint,
  header: phase0.BeaconBlockHeader,
  state: BeaconStateAllForks
): PreimageMap {
  const checkpointRootNode = ssz.phase0.Checkpoint.toView(checkpoint).node;
  const headerRootNode = ssz.phase0.BeaconBlockHeader.toView(header).node;
  const stateRootNode = state.node;

  // assert that the checkpoint , header, and state match

  if (!ssz.Root.equals(checkpoint.root, headerRootNode.root)) {
    throw new Error("Checkpoint.root !== header root");
  }

  if (!ssz.Root.equals(header.stateRoot, stateRootNode.root)) {
    throw new Error("header.stateRoot !== state root");
  }
  const {
    checkpoint: checkpointGindices,
    header: headerGindices,
    state: stateGindices,
  } = getZiplineGindices(state.validators.length, checkpoint.epoch);

  const preimages: PreimageMap = new Map();
  log.info("get checkpoint preimages");
  addTreePreimages(checkpointRootNode, checkpointGindices, preimages);
  log.info("get header preimages");
  addTreePreimages(headerRootNode, headerGindices, preimages);
  log.info("get state preimages");
  addTreePreimages(stateRootNode, stateGindices, preimages);

  return preimages;
}

export async function writePreimages(preimages: PreimageMap, directory: string): Promise<void> {
  // ensure directory exists
  await fs.mkdir(directory, { recursive: true });
  const filePath = path.join(directory, "preimages.bin");
  const stream = createWriteStream(filePath, { encoding: "binary" });
  for (const [hash, preimage] of preimages.entries()) {
    // write key
    stream.write(Buffer.from(hash, "hex"));
    // write value
    stream.write(Buffer.from(preimage));
  }
  await new Promise((resolve) => stream.end(resolve));
}
