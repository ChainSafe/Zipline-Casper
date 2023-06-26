import { ContainerType, ValueOf, UintNumberType, ListCompositeType } from "@chainsafe/ssz";
import type { Signature } from "@chainsafe/bls/types";
import bls from "@chainsafe/bls";

import { ssz } from "@lodestar/types";
import { Attestation, AttestationData } from "@lodestar/types/phase0";
import { computeSigningRoot } from "@lodestar/state-transition";
import { DOMAIN_BEACON_ATTESTER } from "@lodestar/params";

import { getPubkey } from "./getPubkeys.js";

// SSZ types

// Limit has been set as "big number", merklization not necessary, so specific isn't important
// 1_000_000 is arbitrarily and sufficiently big
const LARGE_LIMIT = 1_000_000;

export const Attestations = new ListCompositeType(ssz.phase0.Attestation, LARGE_LIMIT);

export const CheckpointIdType = new UintNumberType(4, { typeName: "CheckpointId" });

export const CheckpointType = new ContainerType(
  {
    epoch: ssz.Epoch,
    root: ssz.Root,
  },
  { typeName: "Checkpoint", jsonCase: "eth2", cachePermanentRootStruct: true }
);

export const CompressedAttestationDataType = new ContainerType(
  {
    slot: ssz.Slot,
    beaconBlockRoot: ssz.Root,
    source: CheckpointIdType,
    target: CheckpointIdType,
  },
  { typeName: "CompressedAttestationData", jsonCase: "eth2", cachePermanentRootStruct: true }
);

export const ParticipationType = new ContainerType(
  {
    bitlist: ssz.phase0.CommitteeBits,
    committeeIndex: ssz.CommitteeIndex,
  },
  { typeName: "ParticipationType", jsonCase: "eth2", cachePermanentRootStruct: true }
);

export const CompressedAttestationType = new ContainerType(
  {
    data: CompressedAttestationDataType,
    participation: new ListCompositeType(ParticipationType, LARGE_LIMIT),
  },
  { typeName: "CompressedAttestation", jsonCase: "eth2", cachePermanentRootStruct: true }
);

export const SuperAttestationType = new ContainerType(
  {
    attestations: new ListCompositeType(CompressedAttestationType, LARGE_LIMIT),
    checkpoints: new ListCompositeType(CheckpointType, LARGE_LIMIT),
    signature: ssz.BLSSignature,
  },
  { typeName: "SuperAttestation", jsonCase: "eth2", cachePermanentRootStruct: true }
);

// ts types

export type CheckpointId = ValueOf<typeof CheckpointIdType>;
export type Checkpoint = ValueOf<typeof CheckpointType>;
export type CompressedAttestationData = ValueOf<typeof CompressedAttestationDataType>;
export type Participation = ValueOf<typeof ParticipationType>;
export type CompressedAttestation = ValueOf<typeof CompressedAttestationType>;
export type SuperAttestation = ValueOf<typeof SuperAttestationType>;

// functionality

/**
 * Adds new checkpoint to checkpoints list if not already present. Returns id of checkpoint if already present or new id otherwise
 */
function ensureCheckpoint(checkpoints: Checkpoint[], checkpoint: Checkpoint): CheckpointId {
  let id = checkpoints.findIndex((c) => CheckpointType.equals(c, checkpoint));
  if (id === -1) {
    id = checkpoints.length;
    checkpoints.push(checkpoint);
  }
  return id;
}

function toZiplineAttestion(
  attestation: Attestation,
  checkpoints: Checkpoint[]
): {
  data: CompressedAttestationData;
  participation: Participation;
  signature: Signature;
} {
  const sourceId = ensureCheckpoint(checkpoints, attestation.data.source);
  const targetId = ensureCheckpoint(checkpoints, attestation.data.target);
  return {
    data: {
      slot: attestation.data.slot,
      beaconBlockRoot: attestation.data.beaconBlockRoot,
      source: sourceId,
      target: targetId,
    },
    participation: {
      committeeIndex: attestation.data.index,
      bitlist: attestation.aggregationBits,
    },
    signature: bls.Signature.fromBytes(attestation.signature),
  };
}

/**
 * Convert attestations (from blocks) to a form useful for zipline
 */
export function compressAttestations(blockAttestations: Attestation[]): SuperAttestation {
  const unaggregatedSuperAttestations: { attestation: CompressedAttestation; signatures: Signature[] }[] = [];
  const checkpoints: Checkpoint[] = [];
  for (const attestation of blockAttestations) {
    const { data, participation, signature } = toZiplineAttestion(attestation, checkpoints);
    const unaggregatedSuperAttestation = unaggregatedSuperAttestations.find((x) =>
      CompressedAttestationDataType.equals(x.attestation.data, data)
    );
    if (!unaggregatedSuperAttestation) {
      unaggregatedSuperAttestations.push({
        attestation: { data, participation: [participation] },
        signatures: [signature],
      });
    } else {
      unaggregatedSuperAttestation.attestation.participation.push(participation);
      unaggregatedSuperAttestation.signatures.push(signature);
    }
  }
  return {
    attestations: unaggregatedSuperAttestations.map((x) => x.attestation),
    checkpoints,
    signature: bls.Signature.aggregate(unaggregatedSuperAttestations.map((x) => x.signatures).flat()).toBytes(),
  };
}

export function verifySuperAttestation(superAttestation: SuperAttestation, getPubkey: getPubkey): boolean {
  const vPubkeys: Uint8Array[] = [];
  const vMessages: Uint8Array[] = [];
  for (const attestation of superAttestation.attestations) {
    for (const participation of attestation.participation) {
      const pubkeys = participation.bitlist
        .getTrueBitIndexes()
        .map((bitIndex) => getPubkey(attestation.data.slot, participation.committeeIndex, bitIndex));
      const pubkey = bls.PublicKey.aggregate(pubkeys).toBytes();
      vPubkeys.push(pubkey);
      const attestationData: AttestationData = {
        slot: attestation.data.slot,
        beaconBlockRoot: attestation.data.beaconBlockRoot,
        index: participation.committeeIndex,
        source: superAttestation.checkpoints[attestation.data.source],
        target: superAttestation.checkpoints[attestation.data.target],
      };
      const message = computeSigningRoot(ssz.phase0.AttestationData, attestationData, DOMAIN_BEACON_ATTESTER);
      vMessages.push(message);
    }
  }

  return bls.verifyMultiple(vPubkeys, vMessages, superAttestation.signature);
}

export function uniqueSlots(superAttestation: SuperAttestation): Set<number> {
  const uniqueSlots = new Set<number>();
  for (const attestation of superAttestation.attestations) {
    uniqueSlots.add(attestation.data.slot);
  }
  return uniqueSlots;
}
