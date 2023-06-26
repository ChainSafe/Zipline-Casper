export const networks = ["mainnet", "goerli", "sepolia", "gnosis", "zhejiang", "chiado"] as const;
export type Network = (typeof networks)[number];
