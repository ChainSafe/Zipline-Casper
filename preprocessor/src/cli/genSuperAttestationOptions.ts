import { CliCommandOptions } from "./types.js";

export type GenSuperAttestationArgs = {
  epoch: number;
  epochEnd?: number;
  directory: string;
  network: string;
  api: string;
  regularAttestation: boolean;
};

export const genSuperAttestationOptions: CliCommandOptions<GenSuperAttestationArgs> = {
  epoch: {
    type: "number",
    demandOption: true,
    description: "Starts getting super attestation in this epoch from the first slot in this epoch",
  },
  regularAttestation: {
    type: "boolean",
    default: false,
    description: "Get regular attestations as well as super attestation",
  },
  epochEnd: {
    type: "number",
    demandOption: false,
    description: "End at the last slot of this epoch",
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
