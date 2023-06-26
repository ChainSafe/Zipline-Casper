import path from "node:path";
import fs from "node:fs/promises";
import { writeFile } from "node:fs/promises";
import { BeaconConfig } from "@lodestar/config";
import { Api } from "@lodestar/api";
import { Epoch, Root, ssz } from "@lodestar/types";
import { BeaconBlockHeader, Checkpoint } from "@lodestar/types/phase0";
import { BeaconStateAllForks, computeEndSlotAtEpoch, computeStartSlotAtEpoch } from "@lodestar/state-transition";

import { StatePatchType, getNextFinalizedData } from "./chainFinality.js";
import { getZiplinePreimages, writePreimages } from "./preimage.js";
import { Attestations, compressAttestations, SuperAttestationType } from "./superAggregate.js";
import { getBlocks } from "./api.js";
import { ContainerType, ListCompositeType, ValueOf } from "@chainsafe/ssz";
import { MAX_ATTESTATIONS } from "@lodestar/params";
import { Gindex, Tree } from "@chainsafe/persistent-merkle-tree";
import { log } from "./logger.js";

export const ZiplineInputType = new ContainerType(
  {
    trustedCp: ssz.phase0.Checkpoint,
    candidateCp: ssz.phase0.Checkpoint,
    stateRoot: ssz.Root,
    patches: new ListCompositeType(StatePatchType, 256),
    attestations: new ListCompositeType(ssz.phase0.Attestation, MAX_ATTESTATIONS),
    stateProof: new ListCompositeType(ssz.Root, 3),
  },
  { typeName: "ZiplineInput", jsonCase: "eth2", cachePermanentRootStruct: true }
);

export type ZiplineInput = ValueOf<typeof ZiplineInputType>;

function getStateProof(header: BeaconBlockHeader): [Root, Root, Root] {
  const headerView = ssz.phase0.BeaconBlockHeader.toView(header);
  const gindex = ssz.phase0.BeaconBlockHeader.getPropertyGindex("stateRoot") as Gindex;
  const tree = new Tree(headerView.node);
  return tree.getSingleProof(gindex) as [Root, Root, Root];
}

export async function preprocessUpdate(
  api: Api,
  config: BeaconConfig,
  trustedCheckpoint: Checkpoint,
  directory: string
): Promise<{ checkpoint: Checkpoint; state: BeaconStateAllForks }> {
  let trustedViewDU = undefined;
  try {
    log.info("it exists we use");
    const b = await fs.readFile(path.join(directory, `state${trustedCheckpoint.epoch}`));
    const trusted = config.getForkTypes(computeStartSlotAtEpoch(trustedCheckpoint.epoch)).BeaconState.deserialize(b);
    trustedViewDU = config.getForkTypes(trusted.slot).BeaconState.toViewDU(trusted);
  } catch {
    log.info("didnt exist");
  }
  const { checkpoint, state, attestations, statePatches, trustedHeader, trustedState } = await getNextFinalizedData(
    directory,
    api,
    config,
    trustedCheckpoint,
    trustedViewDU
  );

  const superAttestation = compressAttestations(attestations);
  const preimages = getZiplinePreimages(config, trustedCheckpoint, trustedHeader, trustedState);

  const stateProof = getStateProof(trustedHeader);

  await writePreimages(preimages, directory);
  await writeFile(path.join(directory, "superAttestation.ssz"), SuperAttestationType.serialize(superAttestation));
  await writeFile(path.join(directory, "finalizedCheckpoint.ssz"), ssz.phase0.Checkpoint.serialize(checkpoint));
  const input: ZiplineInput = {
    stateRoot: trustedState.hashTreeRoot(),
    trustedCp: trustedCheckpoint,
    candidateCp: checkpoint,
    patches: statePatches,
    attestations,
    stateProof,
  };
  await writeFile(path.join(directory, "input.ssz"), ZiplineInputType.serialize(input));
  return {
    checkpoint,
    state,
  };
}

export async function getSuperAttestationAtEpoch(
  api: Api,
  directory: string,
  epoch: Epoch,
  regularAttestaion: boolean,
  epochEnd?: Epoch
): Promise<void> {
  const start = computeStartSlotAtEpoch(epoch);
  const end = epochEnd ? computeEndSlotAtEpoch(epochEnd) : computeEndSlotAtEpoch(epoch);
  const blocks = await getBlocks(api, start, end + 1);
  const superAttestation = compressAttestations(blocks.map((block) => block.message.body.attestations).flat());
  await fs.mkdir(directory, { recursive: true });
  await writeFile(
    path.join(directory, `${epoch}-${epochEnd ? epochEnd : epoch}-superAttestation.ssz`),
    SuperAttestationType.serialize(superAttestation)
  );
  if (regularAttestaion) {
    const attestations = blocks.map((block) => block.message.body.attestations).flat();
    const attestationsSSZ = Attestations.serialize(attestations);
    await writeFile(path.join(directory, `${epoch}-${epochEnd ? epochEnd : epoch}-attestations.ssz`), attestationsSSZ);
  }
}
