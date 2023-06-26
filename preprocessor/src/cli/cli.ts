import yargs, { CommandModule } from "yargs";
import { hideBin } from "yargs/helpers.js";
import { Api, getClient } from "@lodestar/api";
import { getSuperAttestationAtEpoch, preprocessUpdate } from "../preprocessor.js";
import { GenAllArgs, genAllOptions } from "./genAllOptions.js";
import { GenSuperAttestationArgs, genSuperAttestationOptions } from "./genSuperAttestationOptions.js";
import { parseCheckpoint } from "./parseCheckpoint.js";
import { getConfig } from "./getConfig.js";
import { Network } from "./networks.js";
import { PreprocessorDaemon } from "../daemon.js";
import { onGracefulShutdown } from "./onGracefulShutdown.js";
import { log } from "../logger.js";
import { Checkpoint } from "@lodestar/types/phase0";
import { BeaconConfig } from "@lodestar/config";

export const yarg = yargs((hideBin as (args: string[]) => string[])(process.argv));

function getParsedArgs(args: GenAllArgs): {
  trustedCheckpoint: Checkpoint;
  directory: string;
  api: Api;
  config: BeaconConfig;
} {
  const config = getConfig(args.network as Network);
  return {
    trustedCheckpoint: parseCheckpoint(args.trustedCheckpoint),
    directory: args.directory,
    api: getClient({ baseUrl: args.api, timeoutMs: 30_000 }, { config }),
    config,
  };
}

export function getZiplineCli(): yargs.Argv {
  return yarg
    .command({
      command: "getSuperAttestationAtEpoch",
      describe: "Get super attestation at epoch encoded as SSZ",
      builder: (yargs) => {
        return yargs.options(genSuperAttestationOptions);
      },
      handler: async (args: GenSuperAttestationArgs) => {
        const { epoch, epochEnd, directory, network, regularAttestation } = args;
        const config = getConfig(network as Network);
        const api = getClient({ baseUrl: args.api, timeoutMs: 30_000 }, { config });
        await getSuperAttestationAtEpoch(api, directory, epoch, regularAttestation, epochEnd);
      },
      // eslint-disable-next-line @typescript-eslint/ban-types
    } as CommandModule<{}, GenSuperAttestationArgs>)
    .command({
      command: "generate",
      describe: "Generate necessary inputs for Zipline verifier",
      builder: (yargs) => {
        return yargs.options(genAllOptions);
      },
      handler: async (args: GenAllArgs) => {
        const { trustedCheckpoint, directory, config, api } = getParsedArgs(args);
        await preprocessUpdate(api, config, trustedCheckpoint, directory);
      },
      // eslint-disable-next-line @typescript-eslint/ban-types
    } as CommandModule<{}, GenAllArgs>)
    .command({
      command: "daemon",
      describe: "Generate necessary inputs for Zipline verifier -- daemon",
      builder: (yargs) => {
        return yargs.options(genAllOptions);
      },
      handler: async (args: GenAllArgs) => {
        const daemon = await PreprocessorDaemon.init(getParsedArgs(args));

        onGracefulShutdown(async () => daemon.close(), log.error);
      },
      // eslint-disable-next-line @typescript-eslint/ban-types
    } as CommandModule<{}, GenAllArgs>)
    .demandCommand()
    .showHelpOnFail(false)
    .strict();
}
