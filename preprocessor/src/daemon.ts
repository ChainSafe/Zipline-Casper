import fs from "node:fs/promises";
import path from "node:path";
import { Api } from "@lodestar/api";
import { routes } from "@lodestar/api/beacon";
import { BeaconConfig } from "@lodestar/config";
import { BeaconStateAllForks, computeEpochAtSlot } from "@lodestar/state-transition";
import { Checkpoint } from "@lodestar/types/phase0";
import { getCheckpointState } from "./api.js";
import { preprocessUpdate } from "./preprocessor.js";
import { toHex } from "@lodestar/utils";

const { EventType } = routes.events;
type BeaconEvent = routes.events.BeaconEvent;
type EventData = routes.events.EventData;

export type PreprocessorDaemonInit = {
  config: BeaconConfig;
  api: Api;
  directory: string;
  trustedCheckpoint: Checkpoint;
};

type PreprocessorDaemonConstructorArgs = PreprocessorDaemonInit & {
  trustedState: BeaconStateAllForks;
};

export class PreprocessorDaemon {
  private readonly config: BeaconConfig;
  private readonly api: Api;
  private readonly directory: string;
  private readonly controller: AbortController;
  private trustedCheckpoint: Checkpoint;
  private trustedState: BeaconStateAllForks;
  private isWorking: boolean;

  private constructor(init: PreprocessorDaemonConstructorArgs) {
    this.config = init.config;
    this.api = init.api;
    this.directory = init.directory;
    this.trustedCheckpoint = init.trustedCheckpoint;
    this.trustedState = init.trustedState;
    this.isWorking = false;
    this.controller = new AbortController();

    this.api.events.eventstream([EventType.head], this.controller.signal, this.onHead as (evt: BeaconEvent) => void);
  }

  static async init(init: PreprocessorDaemonInit): Promise<PreprocessorDaemon> {
    const state = await getCheckpointState(init.api, init.trustedCheckpoint);
    const trustedState = init.config.getForkTypes(state.slot).BeaconState.toViewDU(state);
    return new PreprocessorDaemon({ ...init, trustedState });
  }

  close(): void {
    this.controller.abort();
  }

  // poor man's clock
  private onHead = (event: { type: typeof EventType.head; message: EventData[typeof EventType.head] }): void => {
    const { slot, epochTransition } = event.message;
    if (!epochTransition) {
      return;
    }

    if (computeEpochAtSlot(slot) - this.trustedCheckpoint.epoch >= 2) {
      void this.doWork();
    }
  };

  private async doWork(): Promise<void> {
    if (this.isWorking) return;
    this.isWorking = true;
    const directory = path.join(
      this.directory,
      `${this.trustedCheckpoint.epoch}:${toHex(this.trustedCheckpoint.root)}`
    );
    try {
      const { checkpoint, state } = await preprocessUpdate(
        this.api,
        this.config,
        this.trustedCheckpoint,
        // TODO: 1. waht is the subdirectory named
        //       2. what to do on failure / only partial success
        directory
      );

      this.trustedCheckpoint = checkpoint;
      this.trustedState = state;
    } catch (e) {
      await fs.rm(directory, { recursive: true });
    } finally {
      this.isWorking = false;
    }
  }
}
