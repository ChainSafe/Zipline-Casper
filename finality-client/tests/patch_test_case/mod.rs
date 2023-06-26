use ethereum_consensus::bellatrix::mainnet as spec;
use ethereum_consensus::state_transition::{Context, Result};
use test_utils::{load_snappy_ssz, Config};
use validator_shuffling::get_randao_index;
use zipline_finality_client::state_patch::StatePatch;
use zipline_spec::Spec;

pub struct PatchTestCase {
    pre: spec::BeaconState,
    post: spec::BeaconState,
    patch: StatePatch,
    config: Config,
}

impl PatchTestCase {
    pub fn from(test_case_path: &str) -> Self {
        let path = test_case_path.to_string() + "/pre.ssz_snappy";
        let pre: spec::BeaconState = load_snappy_ssz(&path).unwrap();

        let path = test_case_path.to_string() + "/post.ssz_snappy";
        let post: spec::BeaconState = load_snappy_ssz(&path).unwrap();

        let (config, context) = if test_case_path.contains("minimal") {
            (Config::Minimal, Context::for_minimal())
        } else {
            (Config::Mainnet, Context::for_mainnet())
        };

        let patch = patch_from_states::<zipline_spec::MainnetSpec>(&pre, &post, &context);

        Self {
            pre,
            post,
            patch,
            config,
        }
    }

    pub fn execute<F>(&mut self, f: F)
    where
        F: FnOnce(
            &mut spec::BeaconState,
            &mut spec::BeaconState,
            &StatePatch,
            &Context,
        ) -> Result<()>,
    {
        let context = match self.config {
            Config::Minimal => Context::for_minimal(),
            Config::Mainnet => Context::for_mainnet(),
        };

        let result = f(&mut self.pre, &mut self.post, &self.patch, &context);
        assert!(result.is_ok())
    }
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
                exits.push(i as u32);
            }
            if validator.activation_epoch == after_epoch {
                activations.push(i as u32);
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
