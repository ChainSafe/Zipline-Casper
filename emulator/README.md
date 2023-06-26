# Cannon Compatible MIPS Emulator

This crate implements a MIPS emulator with all the special features required by Cannon. It has an easy to use CLI and is much MUCH faster that the Optimism implementation as of July 2023.

## Building

Building cannon-emulator also builds unicorn. See [their build instructons](https://github.com/unicorn-engine/unicorn/blob/master/docs/COMPILE.md) for required dependencies. 

## Usage

```shell
USAGE:
    zipline_unicorn [FLAGS] [OPTIONS] <program-path> <SUBCOMMAND>

FLAGS:
    -h, --help           Prints help information
    -i, --interactive    If the CLI chould go into interactive mode after execution to allow querying trie nodes
    -V, --version        Prints version information

OPTIONS:
        --input <input>
            The input to the execution. A hex encoded hash (32 bytes, 64 chars) which will be placed in the designated
            input memory slots before starting execution [env: CANNON_INPUT=]
        --multi-preimage-file <multi-preimage-file>
            Load a file that contains many pre-images The file stores 32 bytes (hash) followed by 64 bytes (image)

        --preimage-files <preimage-files>...
            List of paths to files to be loadable by the pre-image oracle Files will be treated as binaries and hashed
            using SHA256

ARGS:
    <program-path>    Path to the binary of the program to run. If not provided will attempt to read from std-in

SUBCOMMANDS:
    dissect-execution    Output the data needed to take one turn in a challenge game by dissecting the trace between
                         start and end into a number of sections. This will output the snapshots at the start and
                         end of each section as well as their step index
    golden-snapshot      Compute the golden root (the Merkle root of the MIPS memory with the program inserted)
    help                 Prints this message or the help of the given subcommand(s)
    initial-snapshot     insert input into the golden snapshot and give an interactive prompt to query trie nodes
    new-challenge        Output the data needed to open a new challenge this is the start and end snapshots and the
                         length of the traces
    one-step-proof       Output the data needed to prove a single instruction execution this includes all memory and
                         register values, and any preimages needed to execute this step
    turbo                Run the program to the end as fast as possible without counting steps or keep track of
                         memory writes
```