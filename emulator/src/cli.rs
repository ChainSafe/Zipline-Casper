use std::path::PathBuf;
use structopt::StructOpt;

fn parse_input_hex(src: &str) -> Result<[u8; 32], hex::FromHexError> {
    let hex = hex::decode(src)?;
    let mut output = [0_u8; 32];
    output.copy_from_slice(&hex);
    Ok(output)
}

#[derive(Debug, StructOpt)]
#[structopt(
    name = "cannon_run",
    about = "A tool to run Cannon compatible binaries with inputs and an optional pre-image oracle"
)]
pub struct Cli {
    /// The input to the execution.
    /// A hex encoded hash (32 bytes, 64 chars) which will be placed in the designated input memory slots before starting execution
    #[structopt(long, parse(try_from_str = parse_input_hex), env = "CANNON_INPUT")]
    pub input: Option<[u8; 32]>,

    /// List of paths to files to be loadable by the pre-image oracle
    /// Files will be treated as binaries and hashed using SHA256
    #[structopt(long, parse(from_os_str))]
    pub preimage_files: Option<Vec<PathBuf>>,

    /// Load a file that contains many pre-images
    /// The file stores 32 bytes (hash) followed by 64 bytes (image)
    #[structopt(long, parse(from_os_str))]
    pub multi_preimage_file: Option<PathBuf>,

    /// If the CLI chould go into interactive mode after
    /// execution to allow querying trie nodes
    #[structopt(long, short)]
    pub interactive: bool,

    /// Path to the binary of the program to run.
    /// If not provided will attempt to read from std-in
    pub program_path: PathBuf,

    #[structopt(subcommand)]
    pub cmd: Command,
}

#[derive(Debug, StructOpt, PartialEq)]
pub enum Command {
    /// Run the program to the end as fast as possible
    /// without counting steps or keep track of memory writes
    Turbo,
    /// Compute the golden root (the Merkle root of the MIPS memory with the program inserted)
    GoldenSnapshot,
    /// insert input into the golden snapshot and give an interactive prompt to
    /// query trie nodes
    InitialSnapshot,
    /// Output the data needed to open a new challenge
    /// this is the start and end snapshots and the length of the traces
    NewChallenge,
    /// Output the data needed to take one turn in a challenge game
    /// by dissecting the trace between start and end into a number of sections.
    /// This will output the snapshots at the start and end of each section as well as their step index
    DissectExecution {
        start: u64,
        end: u64,
        sections: usize,
        fuckup_step: Option<u64>,
    },
    /// Output the data needed to prove a single instruction execution
    /// this includes all memory and register values, and any preimages needed to execute this step
    OneStepProof { step: u64 },
}
