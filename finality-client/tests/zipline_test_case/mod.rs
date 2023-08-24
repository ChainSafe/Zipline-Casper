use crypto::hash::H256;
use ethereum_consensus::bellatrix::mainnet as spec;
use ethereum_consensus::bellatrix::mainnet::{BeaconBlock, BeaconState};
use ethereum_consensus::state_transition::{Context, Validation};
use ssz_rs::compute_proof;
use ssz_rs::prelude::*;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use test_utils::{load_snappy_ssz, load_yaml, Config};
use validator_shuffling::get_randao_index;
use zipline_finality_client::attestation::Checkpoint;
use zipline_finality_client::input::ZiplineInput;
use zipline_finality_client::state_patch::StatePatch;
use zipline_spec::Spec;

type Attestation =
    zipline_finality_client::attestation::Attestation<{ spec::MAX_VALIDATORS_PER_COMMITTEE }>;

#[derive(Default, SimpleSerialize)]
pub struct ZiplineTestCase {
    pub trusted: Checkpoint,
    pub candidate: Checkpoint,
    pub attestations: List<Attestation, 1000>,
    pub state: BeaconState,
    pub patches: List<StatePatch, 10>,
    pub state_proof: List<H256, 3>,
    pub expected_result: bool,
}

#[derive(serde::Deserialize)]
struct FinalityMeta {
    blocks_count: usize,
}

struct ChainPoint {
    state: BeaconState,
    block: BeaconBlock,
    checkpoint: Checkpoint,
}

impl ChainPoint {
    fn new(state: BeaconState, block: BeaconBlock, checkpoint: Checkpoint) -> Self {
        assert_eq!(
            checkpoint.root,
            block.clone().hash_tree_root().unwrap().as_ref(),
            "checkpoint must reference block"
        );
        assert_eq!(
            block.state_root,
            state.clone().hash_tree_root().unwrap().as_ref(),
            "block must reference state"
        );
        Self {
            state,
            block,
            checkpoint,
        }
    }
}

impl ZiplineTestCase {
    pub fn from_eth_spec_path<S: Spec>(test_case_path: &str) -> Vec<Self> {
        let path = test_case_path.to_string() + "/pre.ssz_snappy";
        let pre: spec::BeaconState = load_snappy_ssz(&path).unwrap();

        let path = test_case_path.to_string() + "/meta.yaml";
        let meta: FinalityMeta = load_yaml(&path);
        let blocks_count = meta.blocks_count;

        let mut blocks = vec![];
        for i in 0..blocks_count {
            let path = format!("{test_case_path}/blocks_{i}.ssz_snappy");
            let block: spec::SignedBeaconBlock = load_snappy_ssz(&path).unwrap();
            blocks.push(block);
        }

        let (_config, context) = if test_case_path.contains("minimal") {
            (Config::Minimal, Context::for_minimal())
        } else {
            (Config::Mainnet, Context::for_mainnet())
        };

        let mut test_cases = Vec::<Self>::new();
        let mut run_state = pre.clone();

        // Calculate the first checkpoint we can trust. This is the state containing the first block
        // and hence the state after applying the first block
        let mut s = pre.clone();
        spec::state_transition(
            &mut s,
            &mut blocks[0],
            spec::NoOpExecutionEngine,
            Validation::Disabled,
            &context,
        )
        .unwrap();

        let first_trusted_cp = Checkpoint {
            epoch: spec::compute_epoch_at_slot(pre.slot, &context),
            root: blocks[0]
                .message
                .hash_tree_root()
                .unwrap()
                .as_ref()
                .try_into()
                .unwrap(),
        };

        // variables to use throughout the loop
        let mut attestation_accumulator = Vec::<Attestation>::new();
        let mut patches = Vec::<StatePatch>::new();
        let mut current_epoch = spec::compute_epoch_at_slot(run_state.slot, &context);
        let mut current_trusted =
            ChainPoint::new(s.clone(), blocks[0].message.clone(), first_trusted_cp);
        let mut state_cache: HashMap<H256, BeaconState> = HashMap::new();

        let bls = blocks.clone();

        for block in blocks.iter_mut() {
            attestation_accumulator.extend(
                block
                    .message
                    .body
                    .attestations
                    .iter()
                    .cloned()
                    .map(to_zipline_attestation),
            );

            let before_state = run_state.clone();
            spec::state_transition(
                &mut run_state,
                block,
                spec::NoOpExecutionEngine,
                Validation::Disabled,
                &context,
            )
            .unwrap();

            state_cache.insert(
                run_state
                    .hash_tree_root()
                    .unwrap()
                    .as_ref()
                    .try_into()
                    .unwrap(),
                run_state.clone(),
            );

            println!("processing block {}", block.message.slot);

            let after_epoch = spec::compute_epoch_at_slot(run_state.slot, &context);
            if after_epoch == current_epoch + 1 {
                println!(
                    "Epoch transition detected {} -> {}",
                    current_epoch, after_epoch
                );
                // we just did an epoch transition.  Store the patch!
                patches.push(patch_from_states::<S>(&before_state, &run_state, &context));
                current_epoch = after_epoch;
            }

            if run_state.finalized_checkpoint.epoch > current_trusted.checkpoint.epoch {
                // a new checkpoint was just finalized
                let candidate = to_zipline_checkpoint(run_state.finalized_checkpoint.clone());
                test_cases.push(ZiplineTestCase {
                    trusted: current_trusted.checkpoint,
                    candidate,
                    attestations: attestation_accumulator.to_vec().try_into().unwrap(),
                    patches: patches
                        .iter()
                        .filter(|p| p.epoch >= candidate.epoch)
                        .cloned()
                        .collect::<Vec<_>>()
                        .try_into()
                        .unwrap(),
                    state: current_trusted.state,
                    state_proof: make_state_proof(&mut current_trusted.block)
                        .to_vec()
                        .try_into()
                        .unwrap(),
                    expected_result: true,
                });
                let cp = to_zipline_checkpoint(run_state.finalized_checkpoint.clone());
                let b = bls
                    .iter()
                    .find(|b| {
                        let block = b.clone();
                        let block_hash: Node = block.message.clone().hash_tree_root().unwrap();
                        block_hash == cp.root
                    })
                    .expect("there should be a block matching the hash")
                    .clone()
                    .message;
                current_trusted = ChainPoint::new(
                    state_cache
                        .get(b.state_root.as_ref())
                        .expect("must have a cached state")
                        .clone(), //find the state as of the finalized checkpoint...
                    b,
                    cp,
                );
            }
        }

        test_cases
    }

    pub fn serialize_to_file(&self, path: &str) {
        let mut f = File::create(path).unwrap();
        f.write_all(&serialize(self).unwrap()).unwrap();
    }

    pub fn deserialize_from_file(path: &str) -> Self {
        let f = File::open(path).unwrap();
        deserialize(&f.bytes().map(|b| b.unwrap()).collect::<Vec<u8>>()).unwrap()
    }

    pub fn to_input(&mut self) -> ZiplineInput<{ spec::MAX_VALIDATORS_PER_COMMITTEE }, 1000, 10> {
        ZiplineInput {
            state_root: self
                .state
                .hash_tree_root()
                .unwrap()
                .as_ref()
                .try_into()
                .unwrap(),
            trusted_cp: self.trusted,
            candidate_cp: self.candidate,
            patches: self.patches.clone(),
            attestations: self.attestations.clone(),
            state_proof: self.state_proof.clone(),
        }
    }

    pub fn from_input(input: ZiplineInput<{ spec::MAX_VALIDATORS_PER_COMMITTEE }, 1000, 10>, state: BeaconState, expected: bool) -> Self {
        assert_eq!(state.clone().hash_tree_root().unwrap(), input.state_root);
        Self {
            state,
            trusted: input.trusted_cp,
            candidate: input.candidate_cp,
            attestations: input.attestations,
            patches: input.patches,
            state_proof: input.state_proof,
            expected_result: expected,
        }
    }
}

fn to_zipline_checkpoint(cp: spec::Checkpoint) -> Checkpoint {
    Checkpoint {
        epoch: cp.epoch,
        root: cp.root.as_ref().try_into().unwrap(),
    }
}

pub fn to_zipline_attestation(a: spec::Attestation) -> Attestation {
    ssz_rs::deserialize(&ssz_rs::serialize(&a).unwrap()).unwrap()
}

pub fn make_state_proof(b: &mut spec::BeaconBlock) -> Vec<H256> {
    let tree = b.to_merkle_tree().unwrap();
    compute_proof(&b.hash_tree_root().unwrap(), 11, &tree).unwrap()
}

pub fn patch_from_states<S: Spec>(
    before: &spec::BeaconState,
    after: &spec::BeaconState,
    context: &Context,
) -> StatePatch {
    let randao: Vec<_> = after
        .randao_mixes
        .iter()
        .map(|e| TryInto::<[u8; 32]>::try_into(e.as_ref()).unwrap())
        .collect();

    let after_epoch = spec::compute_epoch_at_slot(after.slot, context);
    let (activations, exits) = after.validators.iter().enumerate().fold(
        (Vec::new(), Vec::new()),
        |(mut activations, mut exits), (i, validator)| {
            if validator.exit_epoch == after_epoch {
                exits.push(i as u32)
            }
            if validator.activation_epoch == after_epoch {
                activations.push(i as u32)
            }
            (activations, exits)
        },
    );

    StatePatch {
        epoch: after_epoch,
        randao_next: randao[get_randao_index::<S>(after_epoch + 1)], // TODO: actually retrieve the correct one for the epoch
        n_deposits_processed: (after.validators.len() - before.validators.len()) as u32,
        activations: activations.try_into().unwrap(),
        exits: exits.try_into().unwrap(),
    }
}
