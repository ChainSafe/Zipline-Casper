const { promisify } = require('util');
const exec = promisify( require('child_process').exec );

const DEPLOYER = "0xf39Fd6e51aad88F6F4ce6aB8827279cffFb92266"

const {
    get_preimage,
} = require('./emulator_calls.js');

function parseToMissingTrieNode (err) {
    let tokens = err.message.split(/\s+/);
    let i = tokens.indexOf("reverted:");
    return tokens[i+1].substr(0,tokens[i+1].length-1);
}

async function open_challenge(unicorn_process, caller, contracts, epoch, rival_snapshot, zipline_input, final_snapshot, n_steps) {
    let current_missing = null;
    let cmd = `ETH_FROM=${caller} cast send ${contracts.Zipline} "challenge(uint64,bytes32,bytes,bytes32,uint256)" ${epoch} ${rival_snapshot} 0x${zipline_input.toString('hex')} 0x${final_snapshot} ${n_steps} --unlocked`;
    while (true) {
        let stdout = null;
        let stderr = null;
        try {
            await exec(cmd);
        } catch (e) {
            current_missing = parseToMissingTrieNode(e);
            if (current_missing.length != 64) {
                return null 
            }
            let preimage = await get_preimage(unicorn_process, current_missing);

            stdout, stderr = await exec(`ETH_FROM=${caller} cast send ${contracts.MIPSMemory} "AddTrieNode(bytes)" 0x${preimage} --unlocked`)
            continue;
        }
        break;
    }
    return exec(cmd)
}

/// Make a call to the bisectExecution function of the Zipline contract
async function bisect_execution(caller, contracts, challenge_index, { old_start, old_length, old_segments, challenge_position }, new_segmentation) {
    let cmd = `ETH_FROM=${caller} cast send ${contracts.Zipline} "dissectChallenge(uint64,(((uint256,uint256),bytes32[]),uint256),bytes32[])" ${challenge_index} "(((${old_start},${old_length}),[${old_segments.toString()}]),${challenge_position})" "[${new_segmentation.toString()}]" --unlocked`;
    return exec(cmd);
}

/// Make a call to the bisectExecution function of the Zipline contract
async function one_step_prove_execution(caller, contracts, challenge_index, { old_start, old_length, old_segments, challenge_position }) {
    let cmd = `ETH_FROM=${caller} cast send ${contracts.Zipline} "proveChallenge(uint64,(((uint256,uint256),bytes32[]),uint256))" ${challenge_index} "(((${old_start},${old_length}),[${old_segments.toString()}]),${challenge_position})" --unlocked`;
    return exec(cmd).catch((e) => { console.log(e.stderr); return null; });
}

/// Make a call to the bisectExecution function of the Zipline contract
async function submit_checkpoint(caller, contracts, epoch, block_root) {
    let cmd = `ETH_FROM=${caller} cast send ${contracts.Zipline} "submit(uint64,bytes32)" ${epoch} ${block_root} --unlocked`;
    return exec(cmd);
}
async function timeout_challenge(caller, contracts, challenge_index) {
    let cmd = `ETH_FROM=${caller} cast send ${contracts.Zipline} "timeoutChallenge(uint64)" ${challenge_index} --unlocked`;
    return exec(cmd);
}
async function get_missing_and_add_to_trie(unicorn_process, contracts, snapshot_root) {
    let current_missing = snapshot_root;
    // Add root trie node
    
    while (true) {
        try {
            await exec(`cast call ${contracts.MIPSExecutor} "Step(bytes32)"  0x${snapshot_root.toString()}`)
        } catch (e) {
            current_missing = parseToMissingTrieNode(e);
            console.log("Missing Trie Node: ", current_missing);
            // Call interactive prompt
            let preimage = await get_preimage(unicorn_process, current_missing);
            console.log("Adding TrieNode: ", preimage);
            // Then addtrienode
            let {stdout, stderr} = await exec(`ETH_FROM=${DEPLOYER} cast send ${contracts.MIPSMemory} "AddTrieNode(bytes)" 0x${preimage} --unlocked`)
            continue;
        }
        break
    }
}

module.exports = {
    open_challenge,
    bisect_execution,
    one_step_prove_execution,
    submit_checkpoint,
    timeout_challenge,
    get_missing_and_add_to_trie
}
