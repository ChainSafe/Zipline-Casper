import { CliCommandOptions } from "./types.js";

export type GenAllArgs = {
  trustedCheckpoint: string;
  directory: string;
  network: string;
  api: string;
};

export const genAllOptions: CliCommandOptions<GenAllArgs> = {
  trustedCheckpoint: {
    type: "string",
    demandOption: true,
    description: "Trusted checkpoint in form of <epoch>:<block hash>",
  },
  directory: {
    type: "string",
    demandOption: true,
    description: "Directory to write inputs",
  },
  network: {
    type: "string",
    demandOption: true,
    description: "Network to use for configuration",
    choices: ["mainnet", "goerli", "sepolia", "gnosis", "zhejiang", "chiado"],
    default: "mainnet",
  },
  api: {
    type: "string",
    demandOption: true,
    description: "REST Beacon API to fetch chain data",
    default: "https://lodestar-mainnet.chainsafe.io",
  },
};
