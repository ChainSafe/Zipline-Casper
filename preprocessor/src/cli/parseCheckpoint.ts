import { Checkpoint } from "@lodestar/types/phase0";
import { fromHex } from "@lodestar/utils";

const checkpointRegex = /^([0-9]+):((?:0x)?[a-fA-F0-9]{64})$/;

export function parseCheckpoint(checkpointStr: string): Checkpoint {
  const match = checkpointRegex.exec(checkpointStr);
  if (!match) {
    throw new Error("Invalid checkpoint string");
  }
  return {
    epoch: Number(match[1]),
    root: fromHex(match[2]),
  };
}
