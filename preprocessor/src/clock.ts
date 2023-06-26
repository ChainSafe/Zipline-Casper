import EventEmitter from "node:events";
import { BeaconConfig } from "@lodestar/config";
import { computeEpochAtSlot, getCurrentSlot } from "@lodestar/state-transition";
import { Slot } from "@lodestar/types";

export enum ClockEvent {
  slot = "slot",
  epoch = "epoch",
}

export class BeaconClock extends EventEmitter {
  private readonly config: BeaconConfig;
  private readonly genesisTime: number;
  private _currentSlot: Slot;
  private timeoutId: NodeJS.Timeout;

  constructor(config: BeaconConfig, genesisTime: number) {
    super();
    this.config = config;
    this.genesisTime = genesisTime;
    this._currentSlot = getCurrentSlot(config, genesisTime);
    this.timeoutId = setTimeout(this.onNextSlot, this.msUntilNextSlot());
  }

  private onNextSlot = (slot?: Slot): void => {
    const clockSlot = slot ?? getCurrentSlot(this.config, this.genesisTime);
    // process multiple clock slots in the case the main thread has been saturated for > SECONDS_PER_SLOT
    while (this._currentSlot < clockSlot) {
      const previousSlot = this._currentSlot;
      this._currentSlot++;

      this.emit(ClockEvent.slot, this._currentSlot);

      const previousEpoch = computeEpochAtSlot(previousSlot);
      const currentEpoch = computeEpochAtSlot(this._currentSlot);

      if (previousEpoch < currentEpoch) {
        this.emit(ClockEvent.epoch, currentEpoch);
      }
    }
    //recursively invoke onNextSlot
    this.timeoutId = setTimeout(this.onNextSlot, this.msUntilNextSlot());
  };

  private msUntilNextSlot(): number {
    const milliSecondsPerSlot = this.config.SECONDS_PER_SLOT * 1000;
    const diffInMilliSeconds = Date.now() - this.genesisTime * 1000;
    return milliSecondsPerSlot - (diffInMilliSeconds % milliSecondsPerSlot);
  }
}
