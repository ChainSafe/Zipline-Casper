use cli::{Cli, Command};
use eth_trie::{MemoryDB, DB};
use log::{debug, error};
use sha2::Digest;
use std::collections::HashMap;
use std::fs;
use std::io::{self, BufRead, Read};
use std::sync::Arc;
use structopt::StructOpt;
use unicorn::TraceConfig;

mod cli;
mod oracle_provider;
mod ram;
mod unicorn;

fn main() {
    let args = Cli::from_args();
    env_logger::init();

    // load program from file
    let program = fs::read(args.program_path).unwrap();
    let mut oracle: HashMap<[u8; 32], Vec<u8>> = HashMap::new();

    // load preimage files into the oracle HashMap
    if let Some(files) = args.preimage_files {
        for input_file in files {
            let data = fs::read(input_file).unwrap();
            let hash = sha2::Sha256::digest(&data);
            let mut key = [0; 32];
            key.copy_from_slice(&hash);
            debug!("Loaded preimage file with hash: {}", hex::encode(key));
            oracle.insert(key, data);
        }
    }

    // load the bunch of preimages from a single file if present
    if let Some(input_file) = args.multi_preimage_file {
        let mut preimages_file = fs::File::open(input_file).unwrap();
        let mut pre = [0; 64];
        let mut im = [0; 32];
        while preimages_file.read_exact(&mut im).is_ok() {
            preimages_file.read_exact(&mut pre).unwrap();
            oracle.insert(im, pre.to_vec());
        }
    }

    let ram = ram::UnsyncRam::new();
    let trie_db = Arc::new(MemoryDB::new(true));

    let mut mu = match args.cmd {
        Command::Turbo => {
            let mut mu =
                unicorn::new_cannon_unicorn(ram, oracle, Some(trie_db), TraceConfig::Turbo);
            let _ = unicorn::write_program(&mut mu, &program);
            let _ = unicorn::write_input(&mut mu, &args.input.unwrap_or([0x00; 32]));
            let (final_snapshot, _steps, result) = unicorn::run(&mut mu, 0);
            debug!("final_shapshot: {:?}", hex::encode(final_snapshot));
            debug!("output: {:?}", result);
            mu
        }
        Command::GoldenSnapshot => {
            let mut mu =
                unicorn::new_cannon_unicorn(ram, oracle, Some(trie_db), TraceConfig::NewChallenge);
            let golden_snapshot = unicorn::write_program(&mut mu, &program);
            println!("{}", hex::encode(golden_snapshot));
            mu
        }
        Command::InitialSnapshot => {
            let mut mu =
                unicorn::new_cannon_unicorn(ram, oracle, Some(trie_db), TraceConfig::NewChallenge);
            let _golden_snapshot = unicorn::write_program(&mut mu, &program);
            let start_snapshot = unicorn::write_input(&mut mu, &args.input.unwrap());
            println!("{}", hex::encode(start_snapshot));
            mu
        }
        Command::NewChallenge => {
            let mut mu =
                unicorn::new_cannon_unicorn(ram, oracle, Some(trie_db), TraceConfig::NewChallenge);
            let golden_snapshot = unicorn::write_program(&mut mu, &program);
            let start_snapshot = unicorn::write_input(&mut mu, &args.input.unwrap_or([0x00; 32]));
            let (final_snapshot, steps, _result) = unicorn::run(&mut mu, 0);
            debug!("golden_snapshot: {:?}", hex::encode(golden_snapshot));

            debug!("start_snapshot: {:?}", hex::encode(start_snapshot));
            debug!("final_shapshot: {:?}", hex::encode(final_snapshot));
            debug!("n_steps: {:?}", steps);

            println!("{} {}", hex::encode(final_snapshot), steps);
            mu
        }
        Command::DissectExecution {
            start,
            end,
            sections,
            fuckup_step,
        } => {
            let mut mu = unicorn::new_cannon_unicorn(
                ram,
                oracle,
                Some(trie_db),
                TraceConfig::DissectExecution {
                    start,
                    end,
                    n_sections: sections,
                    fuckup_step,
                },
            );
            let _golden_snapshot = unicorn::write_program(&mut mu, &program);
            let _start_snapshot = unicorn::write_input(&mut mu, &args.input.unwrap_or([0x00; 32]));
            let (_final_snapshot, _steps, _result) = unicorn::run(&mut mu, 0);

            for snapshot in mu.get_data().snapshots.iter() {
                print!("{} ", hex::encode(snapshot.1))
            }
            mu
        }
        Command::OneStepProof { step } => {
            let mut mu = unicorn::new_cannon_unicorn(
                ram,
                oracle,
                Some(trie_db),
                TraceConfig::OneStepProof { step },
            );
            let _golden_snapshot = unicorn::write_program(&mut mu, &program);
            let _start_snapshot = unicorn::write_input(&mut mu, &args.input.unwrap_or([0x00; 32]));
            let (final_snapshot, _steps, _result) = unicorn::run(&mut mu, 0);

            println!("{}", hex::encode(final_snapshot));
            mu
        }
    };

    if args.interactive {
        let db = unicorn::get_trie_db(&mut mu);

        debug!("Entering Interactive Mode. Request snapshot trie nodes by their hex encoded hash");
        let lines = io::stdin().lock().lines();

        for line in lines {
            let input = line.unwrap();

            // stop reading
            if input.is_empty() {
                debug!("Program exit");
                break;
            }

            if let Some(node) = db
                .get(&hex::decode(input.clone()).unwrap_or_else(|_| {
                    panic!("Input not hex decodable: {}", input);
                }))
                .unwrap()
            {
                println!("{}", hex::encode(&node));
            } else {
                error!("Node not found");
            }
        }
    }
}
