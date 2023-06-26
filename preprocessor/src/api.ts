import { Api } from "@lodestar/api";
import { FinalityCheckpoints, StateId } from "@lodestar/api/lib/beacon/routes/beacon/state";
import { BlockId } from "@lodestar/api/lib/beacon/routes/beacon/block";
import { BeaconState, SignedBeaconBlock } from "@lodestar/types/allForks";
import { BeaconBlockHeader, Checkpoint, Genesis } from "@lodestar/types/phase0";
import { toHex } from "@lodestar/utils";
import { StateFormat } from "@lodestar/api/beacon/routes/debug";

export async function getState(api: Api, id: StateId, format: "ssz"): Promise<Uint8Array>;
export async function getState(api: Api, id: StateId, format: "json"): Promise<BeaconState>;
export async function getState(api: Api, id: StateId): Promise<BeaconState>;

export async function getState(api: Api, id: StateId, format?: StateFormat): Promise<Uint8Array | BeaconState> {
  if (format === undefined || format === "json") {
    const resp = await api.debug.getStateV2(id);
    if (!resp.ok) {
      throw new Error(resp.error.message);
    }
    return resp.response.data;
  } else if (format === "ssz") {
    const resp = await api.debug.getStateV2(id, "ssz");
    if (!resp.ok) {
      throw new Error(resp.error.message);
    }
    return resp.response;
  } else {
    throw new Error("Unsupported format must be ssz or undefined");
  }
}

export async function getCheckpointState(api: Api, checkpoint: Checkpoint): Promise<BeaconState> {
  const header = await getBlockHeader(api, toHex(checkpoint.root));
  return (await getState(api, toHex(header.stateRoot))) as BeaconState;
}

export async function getBlock(api: Api, id: BlockId): Promise<SignedBeaconBlock> {
  const resp = await api.beacon.getBlockV2(id);
  if (!resp.ok) {
    throw new Error(resp.error.message);
  }
  return resp.response.data;
}

export async function getBlockHeader(api: Api, id: BlockId): Promise<BeaconBlockHeader> {
  const resp = await api.beacon.getBlockHeader(id);
  if (!resp.ok) {
    throw new Error(resp.error.message);
  }
  return resp.response.data.header.message;
}

export async function getBlocks(api: Api, from: number, to: number): Promise<SignedBeaconBlock[]> {
  const allPromises = await Promise.allSettled(
    Array.from({ length: to - from }, (_, i) => i + from).map((slot) => getBlock(api, slot))
  );
  return allPromises
    .map((res) => (res.status === "fulfilled" ? res.value : undefined))
    .filter((res) => res) as SignedBeaconBlock[];
}

export async function getGenesis(api: Api): Promise<Genesis> {
  const resp = await api.beacon.getGenesis();
  if (!resp.ok) {
    throw new Error(resp.error.message);
  }
  return resp.response.data;
}

export async function getFinalityCheckpoints(api: Api, id: StateId): Promise<FinalityCheckpoints> {
  const resp = await api.beacon.getStateFinalityCheckpoints(id);
  if (!resp.ok) {
    throw new Error(resp.error.message);
  }
  return resp.response.data;
}
