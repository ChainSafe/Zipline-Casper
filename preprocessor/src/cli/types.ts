import { Options } from "yargs";

export type CliCommandOptions<OwnArgs> = Required<{ [key in keyof OwnArgs]: Options }>;
