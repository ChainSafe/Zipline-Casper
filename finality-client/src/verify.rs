use crypto::hash::H256;
use log::{trace, warn};
use ssz_rs::{is_valid_merkle_branch, Node};
use validator_shuffling::{committee_shuffle_seed_from_randao, CommitteeCache, ShuffleData};
use zipline_spec::Spec;

use crate::attestation::{Attestation, CasperLink};
use crate::input::ZiplineInput;
use crate::signing::verify_signed_attestation;
use crate::state_patch::StatePatch;
use crate::state_reader::{StateReadError, StateReader};

use alloc::collections::btree_map::BTreeMap as Map;
use alloc::collections::btree_set::BTreeSet as Set;

use crate::state_reader::PatchedStateReader;

use alloc::vec::Vec;

pub fn verify<
    S: Spec,
    PSR: PatchedStateReader,
    const MAX_COMMITTEE_SIZE: usize,
    const MAX_ATTESTATIONS: usize,
    const MAX_PATCHES: usize,
>(
    state_reader: PSR::SR,
    mut input: ZiplineInput<MAX_COMMITTEE_SIZE, MAX_ATTESTATIONS, MAX_PATCHES>,
) -> Result<bool, Error> {
    let trusted_cp = input.trusted_cp;
    let candidate_cp = input.candidate_cp;
    log::info!("Verify Start!");
    log::info!(
        "We have {} attestations and {} patches",
        input.attestations.len(),
        input.patches.len()
    );
    log::debug!("0. Checking pre-conditions");
    /////////// 0. pre-conditions  //////////////
    assert_eq!(
        candidate_cp.epoch,
        trusted_cp.epoch + 1,
        "Candidate must be direct successor of trusted checkpoint"
    );
    assert!(
        !input.patches.is_empty(),
        "Must be at least one patch to finalize candidate"
    );
    assert_eq!(
        input.patches[0].epoch, candidate_cp.epoch,
        "first patch must produce the state as of the candidate"
    );
    assert!(
        contiguous_patches(&input.patches),
        "Patches must be of contiguous, incrementing epochs"
    );
    assert!(
        is_valid_merkle_branch(
            &node_from_hash(&state_reader.root()?),
            input
                .state_proof
                .iter()
                .map(node_from_hash)
                .collect::<Vec<_>>()
                .iter(),
            3,  // depth of BeaconBlockHeader merklizations
            11, // gindex of state_root in beacon block header
            &node_from_hash(&trusted_cp.root),
        ),
        "Given state root must correspond to the trusted_cp as shown by proof"
    );

    /////////// 1. Attestation processing  //////////////
    log::debug!("1. Attestation processing start");
    let mut state_reader = PSR::new(state_reader);

    // these are the epochs we have states (or can patch state) for
    let epoch_range = trusted_cp.epoch..=trusted_cp.epoch + (input.patches.len() as u64);
    let patches = [None] // no need to patch first epoch
        .into_iter()
        .chain(input.patches.iter().map(Some));

    // how much attested balance we have for each link found so far
    // in attestations with valid signatures
    let mut attested_balance_by_link = Map::<CasperLink, u64>::new();
    for (epoch, patch) in epoch_range.zip(patches) {
        log::info!("Loop epoch: {}", epoch);
        // patch the state reader if required
        if let Some(patch) = patch {
            trace!("Checking patch validity");
            if !patch.is_valid::<S>(state_reader.get_active_validator_indices(epoch)?.len() as u32)
            {
                return Ok(false);
            }
            trace!("Applying patch to state");
            trace!(
                "Patch has:\n\t{} activations\n\t{} exits\n\t{} deposits processed",
                patch.activations.len(),
                patch.exits.len(),
                patch.n_deposits_processed
            );
            state_reader = state_reader.with_patch(patch.clone());
        }
        // using the state at 'epoch' we can verify attestations at 'attestations_epoch'
        let attestations_epoch = epoch + 1;

        let committee_cache = get_shufflings_for_epoch::<S, _>(&state_reader, attestations_epoch)?;

        let epoch_attestations = input
            .attestations
            .iter_mut()
            .filter(|a| S::epoch(a.data.slot as usize) as u64 == attestations_epoch);
        for a in epoch_attestations {
            trace!(
                "Checking attestation for slot: {} committee: {}",
                a.data.slot,
                a.data.index
            );
            let committee = committee_cache
                .get_beacon_committee::<S>(a.data.slot as usize, a.data.index as usize)
                .unwrap();
            let participants = get_attesting_indices(committee, a);
            trace!(
                "Attestations has {}/{} participants",
                participants.len(),
                committee.len()
            );

            let (pubkeys, attesting_balance) = state_reader
                .aggregate_validator_keys_and_balance(&participants)
                .unwrap();
            log::trace!("Verifying signed attestations");

            match verify_signed_attestation::<S, MAX_COMMITTEE_SIZE>(a, &pubkeys) {
                Ok(_) => {
                    trace!("Signature ok!");
                    if let Some(val) = attested_balance_by_link.get_mut(&a.data.link()) {
                        *val += attesting_balance;
                    } else {
                        attested_balance_by_link.insert(a.data.link(), attesting_balance);
                    }
                }
                Err(e) => {
                    warn!("Invalid attestation signature found: {:?}", e);
                    warn!("Attesting indices: {:?}", participants);
                }
            }
        }

        trace!(
            "Current total balance per link: {:?}",
            attested_balance_by_link
        );
    }

    /////////// 2. Finality calculation  //////////////
    log::debug!("2. Finality calculation start");
    // ok now we have verified all the attestations signatures we can and aggregated the attesting balance
    // for each link across all epoch. The resulting map between links and attesting balance can be used to
    // calculate the supermajority links that we have

    let sm_links = get_supermajority_links::<PSR, MAX_COMMITTEE_SIZE>(
        &state_reader,
        trusted_cp.epoch,
        &attested_balance_by_link,
    )?;

    // Because by definition the trusted CP is finalized we know that:
    // - All checkpoints prior to trusted_cp are finalized
    // - The candidate is justified (since to finalize requires a link forward to the tip of a chain of justified CPs which must include the direct successor)
    // so all we really need to look at is finalizing the candidate or (in the case of delayed finality) one of its successors.
    // we will ignore the delayed finality case for now

    // by definition the trusted and candidate checkpoints are justified
    let mut hightest_justified_epoch = candidate_cp.epoch;
    trace!("Highest justified epoch: {}", hightest_justified_epoch);
    for epoch in candidate_cp.epoch + 1.. {
        // if we can justify the checkpoint at that epoch then do it or else abort
        if sm_links
            .iter()
            .any(|link| link.source.epoch <= hightest_justified_epoch && link.target.epoch == epoch)
        {
            hightest_justified_epoch = epoch;
            trace!("Successfully justified epoch: {}", epoch);
        } else {
            // no way to finalize if we have a gap in the sequence of justified checkpoints
            warn!(
                "Non-contiguous sequence of finalized checkpoints prohibits finalizing candidate"
            );
            return Ok(false);
        }
        // see if we can now finalize the candidate by linking to the end of a sequence of justified checkpoints
        if sm_links.iter().any(|link| {
            link.source == candidate_cp && link.target.epoch <= hightest_justified_epoch
        }) {
            log::info!("Successfully finalized candidate");
            return Ok(true);
        }
    }

    log::error!("Unsuccessfully finalized candidate. This should never happen.");
    // unable to finalize candidate with given inputs
    // this should actually be unreachable
    Ok(false)
}

// process attestations to produce supermajority links. A supermajority link is defined as a
// (source, target) pair with:
// - valid signatures by enough validators to comprise 2/3 of the total active balance in the validator set
// - a justified source
fn get_supermajority_links<SR: StateReader, const MAX_COMMITTEE_SIZE: usize>(
    state_reader: &SR,
    epoch: u64,
    links: &Map<CasperLink, u64>,
) -> Result<Set<CasperLink>, Error> {
    let total_active_balance = state_reader.get_total_active_balance(epoch)?;
    let sm_links = links
        .iter()
        .filter(|(_, attesting_balance)| *attesting_balance * 3 >= &total_active_balance * 2) // check enough participation
        .map(|(link, _)| *link)
        .collect();
    Ok(sm_links)
}

// this can compute validators for up to
// 1 epoch ahead of the epoch the state_reader can read from
pub fn get_shufflings_for_epoch<S: Spec, SR: StateReader>(
    state_reader: &SR,
    epoch: u64,
) -> Result<CommitteeCache, Error> {
    log::trace!("Getting shufflings for epoch: {}", epoch);
    // first up lets compute and cache the committee shufflings for this epoch
    let len_total_validators: usize = state_reader.get_validator_count()?;
    log::trace!("Valdator count: {}", len_total_validators);

    let active_validator_indices = state_reader.get_active_validator_indices(epoch)?;

    // Use the randao to compute the seed
    let mix = state_reader.get_randao::<S>(epoch)?;
    log::trace!("Randao mix: {:?}", mix);
    let seed = committee_shuffle_seed_from_randao::<S>(mix, epoch as usize);

    CommitteeCache::initialized::<S>(
        ShuffleData {
            seed,
            active_validator_indices,
            len_total_validators,
        },
        epoch as usize,
    )
    .map_err(|_| Error::CommitteeCache)
}

pub fn get_attesting_indices<const MAX_COMMITTEE_SIZE: usize>(
    committee: &[usize],
    attestation: &Attestation<MAX_COMMITTEE_SIZE>,
) -> Vec<usize> {
    committee
        .iter()
        .enumerate()
        .filter(|(i, _)| attestation.aggregation_bits[*i])
        .map(|(_, validator_index)| *validator_index)
        .collect()
}

fn contiguous_patches(patches: &[StatePatch]) -> bool {
    patches.windows(2).all(|w| w[0].epoch + 1 == w[1].epoch)
}

#[allow(dead_code)] // TODO: Remove this when cleaning up
fn sorted_attestations<const MAX_COMMITTEE_SIZE: usize>(
    attestations: &[Attestation<MAX_COMMITTEE_SIZE>],
) -> bool {
    attestations
        .windows(2)
        .all(|w| w[1].data.slot >= w[0].data.slot)
}

fn node_from_hash(h: &H256) -> Node {
    Node::try_from(h.as_ref()).expect("is right size")
}

#[derive(Debug)]
pub enum Error {
    StateRead(StateReadError),
    CommitteeCache,
}

impl From<StateReadError> for Error {
    fn from(value: StateReadError) -> Self {
        Self::StateRead(value)
    }
}

impl From<validator_shuffling::Error> for Error {
    fn from(_value: validator_shuffling::Error) -> Self {
        Self::CommitteeCache
    }
}
