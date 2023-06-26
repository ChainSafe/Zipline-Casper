const { spawn } = require('child_process');
const { promisify } = require('util');
const exec = promisify( require('child_process').exec );
const fs = require('fs');

const binary = "../zipline-state-transition-mips/build/spec_test_out.bin"

// Send commands to the child process
const sendCommand = (child, command) => {
    const isCommandSent = child.stdin.write(`${command}\n`);
    if (!isCommandSent) {
        child.stdin.once('drain', () => {
            // console.log(`1Command sent: ${command}`);
        });
    } else {
        // console.log(`Failed Command sent: ${command}`);
    }
};


// Get preimage of hash from unicorn
async function get_preimage(unicorn_process, hash) {
    return new Promise((resolve, reject) => {
        // Add event listner
        unicorn_process.stdout.once('data', (d) => {
            let data = d.toString().trim();
            // remove this listener
            resolve(data);
        });
        // Call to stdin
        sendCommand(unicorn_process, `${hash}`);
        
    });
}

async function unicorn_get_final_snapshot_root_one_step_proof(input_hash, step, interactive = false) {
    let args = ['run', '--release', binary, `--preimage-files`, `../demo/demo_data/input.ssz.bin`, `--multi-preimage-file`, `../demo/demo_data/beacon_state_preimages.bin`, `--input`, `${input_hash}`];
    if (interactive) {
        args.push('-i');
    }
    args.push('one-step-proof');
    args.push(`${step}`);
    return new Promise((resolve, reject) => {
        const unicorn_process = spawn('cargo', args, { stdio: ['pipe', 'pipe', 'pipe'], cwd: '../emulator' });
        // Listen for when snapshot is outputted implying the trace is done 
        // Now unicorn will be in interactive mode
        unicorn_process.stdout.on('data', (d) => {
            let one_step_final_snapshot = d.toString().trim();
            unicorn_process.stdout.removeAllListeners('data');
            console.log("Final snapshot: ", one_step_final_snapshot)
            if (!interactive) {
                unicorn_process.kill();
                resolve({one_step_final_snapshot})
            }
            resolve ({unicorn_process, one_step_final_snapshot});
        });
    })
}

async function start_unicorn_new_challenge(input_hash, interactive = false) {
    let args = ['run', '--release', binary, `--preimage-files`, `../demo/demo_data/input.ssz.bin`, `--multi-preimage-file`, `../demo/demo_data/beacon_state_preimages.bin`, `--input`, `${input_hash}`];
    if (interactive) {
        args.push('-i');
    }
    args.push('new-challenge')
    return new Promise ((resolve, reject) => {
        const new_challenge_unicorn_process = spawn('cargo', args, { stdio: ['pipe', 'pipe', 'pipe'], cwd: '../emulator' });
        // Listen for when snapshot is outputted implying the trace is done 
        // Now unicorn will be in interactive mode
        new_challenge_unicorn_process.stdout.on('data', (d) => {
            let final_snapshot_and_steps = d.toString().trim().split(/\s+/);
            let final_snapshot = final_snapshot_and_steps[0];
            let steps = final_snapshot_and_steps[1];
            new_challenge_unicorn_process.stdout.removeAllListeners('data');
            console.log("final_snapshot: ", final_snapshot)
            if (!interactive) {
                new_challenge_unicorn_process.kill();
                resolve({final_snapshot, steps})
            }
            resolve ({new_challenge_unicorn_process, final_snapshot, steps});
        });
    })
}

async function dissect_trace(input_hash, start, end, sections, fuckup_step) {
    console.log("Dissecting trace start")
    let cmd = `cargo run --release ${binary} --preimage-files=../demo/demo_data/input.ssz.bin --multi-preimage-file=../demo/demo_data/beacon_state_preimages.bin --input=${input_hash} dissect-execution ${start} ${end} ${sections} ${fuckup_step ? fuckup_step : ""}`
    const { stdout, stderr } = await exec(cmd);
    
    let snapshots = stdout.toString().trim().split(/\s+/);
    return (snapshots);
}

async function deploy_zipline_contracts(golden_root, trusted_epoch, trusted_block_root) {
    console.log("Deploying Zipline contracts via forge script");
    const { stdout, stderr } = await exec(`GOLDEN_SNAPSHOT=${golden_root} TRUSTED_EPOCH=${trusted_epoch} TRUSTED_BLOCK_ROOT=${trusted_block_root} forge script ./script/DeployWithMIPS.s.sol:DeployWithMIPS --fork-url http://localhost:8545 --broadcast`, {cwd: '../contracts'});
    // Read json file and get address of all 3 contracts
    const data = fs.readFileSync('../contracts/broadcast/DeployWithMIPS.s.sol/31337/run-latest.json', 'utf8');
    const json = JSON.parse(data);
    const txns = json["transactions"]

    return txns.reduce((deployedContracts, txn) => {
        if (txn["transactionType"] === "CREATE") {
            deployedContracts[txn["contractName"]] = txn["contractAddress"]
        }
        return deployedContracts
    }, {})
}

async function get_golden_snapshot_root(interactive = false) {
    let args = ['run', '--release' , binary];
    if (interactive) {
        args.push('-i');
    }
    args.push('golden-snapshot')
    return new Promise((resolve, reject) => {
        const golden_snapshot_unicorn_process = spawn('cargo', args, { stdio: ['pipe', 'pipe', 'pipe'], cwd: '../emulator' });

        golden_snapshot_unicorn_process.stdout.on('data', (d) => {
            let golden_snapshot = d.toString().trim();
            golden_snapshot_unicorn_process.stdout.removeAllListeners('data');
            console.log("Golden snapshot: ", golden_snapshot)
            if (!interactive) {
                golden_snapshot_unicorn_process.kill();
                resolve(golden_snapshot)
            }
            resolve ({golden_snapshot_unicorn_process, golden_snapshot});
        });

        let error_log = "";
        golden_snapshot_unicorn_process.stderr.on('data', (d) => {
            error_log += d.toString().trim();
        });

        golden_snapshot_unicorn_process.on('close', (code) => {
            if (code != 0) {
                console.error("Emulator failed with:\n", error_log);
            }
        })
    })
}

async function get_initial_snapshot_root(input_hash, interactive = false) {
    return new Promise((resolve, reject) => {
        const initial_unicorn_process = spawn('cargo', ['run', '--release' , binary, '--input', input_hash, 'initial-snapshot'], { stdio: ['pipe', 'pipe', 'pipe'], cwd: '../emulator' });
        // Listen for when snapshot is outputted implying the trace is done 
        // Now unicorn will be in interactive mode
        initial_unicorn_process.stdout.on('data', (d) => {
            let initial_snapshot = d.toString().trim();
            initial_unicorn_process.stdout.removeAllListeners('data');
            if (!interactive) {
                initial_unicorn_process.kill();
                resolve(initial_snapshot)
            }
            resolve ({initial_unicorn_process, initial_snapshot});
        });
        
    })
} 

module.exports = {
    get_preimage,
    get_initial_snapshot_root,
    get_golden_snapshot_root,
    unicorn_get_final_snapshot_root_one_step_proof,
    deploy_zipline_contracts,
    dissect_trace,
    start_unicorn_new_challenge,
}
