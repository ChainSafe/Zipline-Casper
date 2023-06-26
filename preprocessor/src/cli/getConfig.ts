import { BeaconConfig, createBeaconConfig } from "@lodestar/config";
import { networksChainConfig, genesisData } from "@lodestar/config/networks";
import { fromHex } from "@lodestar/utils";
import { Network } from "./networks.js";

export function getConfig(network: Network): BeaconConfig {
  return createBeaconConfig(networksChainConfig[network], fromHex(genesisData[network].genesisValidatorsRoot));
}
