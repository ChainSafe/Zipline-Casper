import { PublicKey } from "@chainsafe/bls/types";
import bls from "@chainsafe/bls";
import { Api, routes } from "@lodestar/api";
import { Epoch, CommitteeIndex, Slot, ValidatorIndex } from "@lodestar/types";
import { Attestation } from "@lodestar/types/phase0";
import { computeEpochAtSlot, computeStartSlotAtEpoch } from "@lodestar/state-transition";

export type getPubkey = (slot: Slot, committeeIndex: CommitteeIndex, bitIndex: number) => PublicKey;

async function getValidators(api: Api): Promise<routes.beacon.ValidatorResponse[]> {
  const resp = await api.beacon.getStateValidators("head");
  if (!resp.ok) {
    throw new Error(resp.error.message);
  }
  return resp.response.data;
}

async function getEpochCommittees(api: Api, epoch: Epoch): Promise<routes.beacon.EpochCommitteeResponse[]> {
  const startSlot = computeStartSlotAtEpoch(epoch);
  const resp = await api.beacon.getEpochCommittees(startSlot);
  if (!resp.ok) {
    throw new Error(resp.error.message);
  }
  return resp.response.data.map((committee) => ({ ...committee, slot: startSlot + committee.slot }));
}

async function getMultiEpochCommitees(
  api: Api,
  epochs: Epoch[]
): Promise<Map<Slot, Map<CommitteeIndex, ValidatorIndex[]>>> {
  const committees = new Map<Slot, Map<CommitteeIndex, ValidatorIndex[]>>();
  for (const committee of (await Promise.all(epochs.map((epoch) => getEpochCommittees(api, epoch)))).flat()) {
    let slotCommittees = committees.get(committee.slot);
    if (!slotCommittees) {
      slotCommittees = new Map<CommitteeIndex, ValidatorIndex[]>();
      committees.set(committee.slot, slotCommittees);
    }

    slotCommittees.set(committee.index, committee.validators);
  }

  return committees;
}

function uniqueSlots(attestations: Attestation[]): Set<Slot> {
  const slots = new Set<Slot>();
  for (const attestation of attestations) {
    slots.add(attestation.data.slot);
  }
  return slots;
}

function uniqueEpochs(slots: Iterable<Slot>): Set<Epoch> {
  const epochs = new Set<Epoch>();
  for (const slot of slots) {
    epochs.add(computeEpochAtSlot(slot));
  }
  return epochs;
}

export async function getGetPubkey(api: Api, attestations: Attestation[]): Promise<getPubkey> {
  // Global list of Validator information including their public keys
  const validators = await getValidators(api);
  // Array of global public keys indexed by their validator index
  const pubkeys = validators.map((v) => bls.PublicKey.fromBytes(v.validator.pubkey));
  // All the unique epochs that our attestations come from
  const epochs = Array.from(uniqueEpochs(uniqueSlots(attestations)));
  // Committees by slot
  const committees = await getMultiEpochCommitees(api, epochs);

  return (slot: Slot, committeeIndex: CommitteeIndex, bitIndex: number): PublicKey => {
    const _committees = committees.get(slot);
    if (!_committees) {
      throw new Error("No committee at slot");
    }
    const committee = _committees.get(committeeIndex);
    if (!committee) {
      throw new Error("No committee with index");
    }
    const validatorIndex = committee[bitIndex];
    if (validatorIndex == null) {
      throw new Error("No validator index found in committee");
    }
    const pubkey = pubkeys[validatorIndex];
    if (!pubkey) {
      throw new Error("No pubkey found with validator index");
    }
    return pubkey;
  };
}
