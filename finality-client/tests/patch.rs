use direct_state_reader::{DirectStateReader, PatchedDirectStateReader};
use ethereum_consensus::bellatrix::mainnet as spec;
use ethereum_consensus::state_transition::Context;
use patch_test_case::PatchTestCase;
use zipline_finality_client::state_patch::StatePatch;
use zipline_finality_client::state_reader::{PatchedStateReader, StateReader};
use zipline_spec::Spec;

mod direct_state_reader;
mod patch_test_case;

macro_rules! test_path {
    ($t:literal) => {
        concat!(
            "../consensus-spec-tests/tests/mainnet/bellatrix/epoch_processing/registry_updates/pyspec_tests/",
            $t
        )
    };
}

fn patched_matches_post<S: Spec>(
    pre: &spec::BeaconState,
    post: &spec::BeaconState,
    patch: &StatePatch,
    context: &Context,
) -> Result<(), ethereum_consensus::state_transition::Error> {
    println!("{:?}", patch);

    let state_reader = DirectStateReader::new(pre.clone());
    let post_state_reader = DirectStateReader::new(post.clone());
    let patched_state_reader =
        PatchedDirectStateReader::new(state_reader).with_patch(patch.clone());

    let epoch = spec::compute_epoch_at_slot(post.slot, context);

    assert_eq!(
        patched_state_reader
            .get_active_validator_indices(epoch)
            .unwrap(),
        post_state_reader
            .get_active_validator_indices(epoch)
            .unwrap()
    );

    assert_eq!(
        patched_state_reader
            .get_total_active_balance(epoch)
            .unwrap(),
        post_state_reader.get_total_active_balance(epoch).unwrap()
    );

    assert_eq!(
        patched_state_reader.get_randao::<S>(epoch).unwrap(),
        post_state_reader.get_randao::<S>(epoch).unwrap()
    );

    for (i, validator) in post.validators.iter().enumerate() {
        // patches only update the entry/exit epoch if they are relevant for the current epoch
        if validator.activation_epoch == epoch {
            assert_eq!(
                patched_state_reader
                    .get_validator_activation_and_exit_epochs(i)
                    .unwrap()
                    .0,
                post_state_reader
                    .get_validator_activation_and_exit_epochs(i)
                    .unwrap()
                    .0
            );
        }
        if validator.exit_epoch == epoch {
            assert_eq!(
                patched_state_reader
                    .get_validator_activation_and_exit_epochs(i)
                    .unwrap()
                    .1,
                post_state_reader
                    .get_validator_activation_and_exit_epochs(i)
                    .unwrap()
                    .1
            );
        }
    }

    assert_eq!(
        patched_state_reader.get_validator_count().unwrap(),
        post_state_reader.get_validator_count().unwrap()
    );

    Ok(())
}

#[test]
fn test_activation_queue_activation_and_ejection_1() {
    let mut test_case =
        PatchTestCase::from(test_path!("activation_queue_activation_and_ejection__1"));

    test_case.execute(|pre, post, patch, context| {
        patched_matches_post::<zipline_spec::MainnetSpec>(pre, post, patch, context)?;
        Ok(())
    });
}

#[test]
fn test_activation_queue_activation_and_ejection_churn_limit() {
    let mut test_case = PatchTestCase::from(test_path!(
        "activation_queue_activation_and_ejection__churn_limit"
    ));

    test_case.execute(|pre, post, patch, context| {
        patched_matches_post::<zipline_spec::MainnetSpec>(pre, post, patch, context)?;
        Ok(())
    });
}

#[test]
fn test_activation_queue_activation_and_ejection_exceed_churn_limit() {
    let mut test_case = PatchTestCase::from(test_path!(
        "activation_queue_activation_and_ejection__exceed_churn_limit"
    ));

    test_case.execute(|pre, post, patch, context| {
        patched_matches_post::<zipline_spec::MainnetSpec>(pre, post, patch, context)?;
        Ok(())
    });
}

#[test]
fn test_activation_queue_efficiency_min() {
    let mut test_case = PatchTestCase::from(test_path!("activation_queue_efficiency_min"));

    test_case.execute(|pre, post, patch, context| {
        patched_matches_post::<zipline_spec::MainnetSpec>(pre, post, patch, context)?;
        Ok(())
    });
}

#[test]
fn test_activation_queue_no_activation_no_finality() {
    let mut test_case =
        PatchTestCase::from(test_path!("activation_queue_no_activation_no_finality"));

    test_case.execute(|pre, post, patch, context| {
        patched_matches_post::<zipline_spec::MainnetSpec>(pre, post, patch, context)?;
        Ok(())
    });
}

#[test]
fn test_activation_queue_sorting() {
    let mut test_case = PatchTestCase::from(test_path!("activation_queue_sorting"));

    test_case.execute(|pre, post, patch, context| {
        patched_matches_post::<zipline_spec::MainnetSpec>(pre, post, patch, context)?;
        Ok(())
    });
}

#[test]
fn test_activation_queue_to_activated_if_finalized() {
    let mut test_case =
        PatchTestCase::from(test_path!("activation_queue_to_activated_if_finalized"));

    test_case.execute(|pre, post, patch, context| {
        patched_matches_post::<zipline_spec::MainnetSpec>(pre, post, patch, context)?;
        Ok(())
    });
}

#[test]
fn test_add_to_activation_queue() {
    let mut test_case = PatchTestCase::from(test_path!("add_to_activation_queue"));

    test_case.execute(|pre, post, patch, context| {
        patched_matches_post::<zipline_spec::MainnetSpec>(pre, post, patch, context)?;
        Ok(())
    });
}

#[test]
fn test_ejection() {
    let mut test_case = PatchTestCase::from(test_path!("ejection"));

    test_case.execute(|pre, post, patch, context| {
        patched_matches_post::<zipline_spec::MainnetSpec>(pre, post, patch, context)?;
        Ok(())
    });
}

#[test]
fn test_ejection_past_churn_limit_min() {
    let mut test_case = PatchTestCase::from(test_path!("ejection_past_churn_limit_min"));

    test_case.execute(|pre, post, patch, context| {
        patched_matches_post::<zipline_spec::MainnetSpec>(pre, post, patch, context)?;
        Ok(())
    });
}
