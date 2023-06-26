#![no_std]

extern crate alloc;

use core::fmt::Debug;
use typenum::{
    Exp, Prod, Shleft, Sum, Unsigned, U1, U10, U128, U13, U14, U16, U2, U24, U3, U32, U4, U41, U43,
    U45, U49, U5, U50, U51, U52, U64, U65536, U8, U9, U90,
};

mod fork_data;
use fork_data::ForkData;
use hex_literal::hex;
use ssz_rs::prelude::*;

pub trait Spec: 'static + Default + Debug {
    type SlotsPerEpoch: Unsigned + Clone + Sync + Send + Debug + PartialEq;

    // these are usually defined in a ChainSpec but since Zipline can't be dynamically
    // configured it makes more sense to define everything at compile time
    type MaxCommitteesPerSlot: Unsigned + Clone + Sync + Send + Debug + PartialEq;
    type TargetCommitteeSize: Unsigned + Clone + Sync + Send + Debug + PartialEq;
    type ShuffleRoundCount: Unsigned + Clone + Sync + Send + Debug + PartialEq;

    // signing domain types
    type DomainBeaconAttester: Unsigned + Clone + Sync + Send + Debug + PartialEq;
    type ForkVersion: Unsigned + Clone + Sync + Send + Debug + PartialEq;

    type MinSeedLookahead: Unsigned + Clone + Sync + Send + Debug + PartialEq;
    type EpochsPerHistoricalVector: Unsigned + Clone + Sync + Send + Debug + PartialEq;

    type MinPerEpochChurnLimit: Unsigned + Clone + Sync + Send + Debug + PartialEq;
    type ChurnLimitQuotient: Unsigned + Clone + Sync + Send + Debug + PartialEq;
    type MaxDeposits: Unsigned + Clone + Sync + Send + Debug + PartialEq;
    // --- Gindex Constants ---

    // ssz.phase0.BeaconState.getPathInfo(['validators'])
    type ValidatorsRootGindex: Unsigned + Clone + Sync + Send + Debug + PartialEq;
    // ssz.phase0.BeaconState.fields.validators.depth
    type ValidatorsTreeDepth: Unsigned + Clone + Sync + Send + Debug + PartialEq;
    // BigInt(2 ** VALIDATORS_TREE_DEPTH) * VALIDATORS_ROOT_GINDEX;
    // Index of Validators[0]
    type Validators0Gindex: Unsigned + Clone + Sync + Send + Debug + PartialEq;
    // ssz.phase0.ValidatorContainer.getPathInfo(['activationEpoch']).gindex
    type ActivationEpochGindex: Unsigned + Clone + Sync + Send + Debug + PartialEq;
    // ssz.phase0.ValidatorContainer.getPathInfo(['exitEpoch']).gindex
    type ExitEpochGindex: Unsigned + Clone + Sync + Send + Debug + PartialEq;
    // ssz.phase0.ValidatorContainer.getPathInfo(['pubkey']).gindex
    type PubkeyGindex: Unsigned + Clone + Sync + Send + Debug + PartialEq;
    // ssz.phase0.ValidatorContainer.getPathInfo(['effectiveBalance']).gindex
    type EffectiveBalanceGindex: Unsigned + Clone + Sync + Send + Debug + PartialEq;

    type ValidatorsLengthGindex: Unsigned + Clone + Sync + Send + Debug + PartialEq; //= VALIDATORS_ROOT_GINDEX * 2n + 1n;

    // ssz.phase0.BeaconState.getPathInfo(['justificationBits']).gindex
    type JustificationBitsGindex: Unsigned + Clone + Sync + Send + Debug + PartialEq;

    // ssz.phase0.BeaconState.getPathInfo(['previousJustifiedCheckpoint']).gindex
    type PreviousJustifiedCheckpointGindex: Unsigned + Clone + Sync + Send + Debug + PartialEq;

    // ssz.phase0.BeaconState.getPathInfo(['currentJustifiedCheckpoint']).gindex
    type CurrentJustifiedCheckpointGindex: Unsigned + Clone + Sync + Send + Debug + PartialEq;

    // ssz.phase0.BeaconState.getPathInfo(['finalizedCheckpoint']).gindex
    type FinalizedCheckpointGindex: Unsigned + Clone + Sync + Send + Debug + PartialEq;

    // ssz.phase0.BeaconState.getPathInfo(['randaoMixes']).gindex
    type RandaoMixesRootGindex: Unsigned + Clone + Sync + Send + Debug + PartialEq;
    // ssz.phase0.BeaconState.fields.randaoMixes.depth
    type RandaoMixesDepth: Unsigned + Clone + Sync + Send + Debug + PartialEq;
    // ssz.phase0.BeaconState.getPathInfo(['randaoMixes', 0]).gindex
    // OR
    // BigInt(2 ** RANDAO_MIXES_DEPTH) * RANDAO_MIXES_ROOT_GINDEX
    type RandaoMixes0Gindex: Unsigned + Clone + Sync + Send + Debug + PartialEq;

    fn slots_per_epoch() -> usize {
        Self::SlotsPerEpoch::to_usize()
    }

    fn max_committees_per_slot() -> usize {
        Self::MaxCommitteesPerSlot::to_usize()
    }

    fn target_committee_size() -> usize {
        Self::TargetCommitteeSize::to_usize()
    }

    fn shuffle_count_count() -> u8 {
        Self::ShuffleRoundCount::to_u8()
    }

    fn fork_version() -> [u8; 4] {
        // [3, 0, 0, 0] //capella
        Self::ForkVersion::to_u32().to_be_bytes()
    }

    fn genesis_validators_root() -> ssz_rs::Node;

    fn fork_data_root(genesis_validators_root: ssz_rs::Node) -> ssz_rs::Node {
        ForkData {
            current_version: Self::fork_version(),
            genesis_validators_root,
        }
        .hash_tree_root()
        .unwrap()
    }

    fn beacon_attester_signing_domain() -> [u8; 32] {
        let domain_type = Self::DomainBeaconAttester::to_u32().to_le_bytes();
        let fork_data_root = Self::fork_data_root(Self::genesis_validators_root());
        let mut domain = [0_u8; 32];
        domain[..4].copy_from_slice(&domain_type);
        domain[4..].copy_from_slice(&fork_data_root.as_ref()[..28]);
        domain
    }

    fn min_seed_lookahead() -> usize {
        Self::MinSeedLookahead::to_usize()
    }

    fn epochs_per_historical_vector() -> usize {
        Self::EpochsPerHistoricalVector::to_usize()
    }

    fn start_slot(epoch: usize) -> usize {
        epoch * Self::slots_per_epoch()
    }

    fn epoch(slot: usize) -> usize {
        slot / Self::slots_per_epoch()
    }

    fn get_committee_count_per_slot(active_validator_count: usize) -> usize {
        Self::get_committee_count_per_slot_with(
            active_validator_count,
            Self::max_committees_per_slot(),
            Self::target_committee_size(),
        )
    }

    fn get_committee_count_per_slot_with(
        active_validator_count: usize,
        max_committees_per_slot: usize,
        target_committee_size: usize,
    ) -> usize {
        let slots_per_epoch = Self::SlotsPerEpoch::to_usize();

        core::cmp::max(
            1,
            core::cmp::min(
                max_committees_per_slot,
                (active_validator_count / slots_per_epoch) / target_committee_size,
            ),
        )
    }

    fn domain_beacon_attester() -> u32 {
        Self::DomainBeaconAttester::to_u32()
    }

    fn min_per_epoch_churn_limit() -> u32 {
        Self::MinPerEpochChurnLimit::to_u32()
    }

    fn churn_limit_quotient() -> u32 {
        Self::ChurnLimitQuotient::to_u32()
    }

    fn max_deposits() -> u32 {
        Self::MaxDeposits::to_u32()
    }
}

/// Ethereum Foundation specifications.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct MainnetSpec;

impl Spec for MainnetSpec {
    type SlotsPerEpoch = U32;
    type MaxCommitteesPerSlot = U64;
    type TargetCommitteeSize = U128;
    type ShuffleRoundCount = U90;

    type DomainBeaconAttester = U1;
    type ForkVersion = Shleft<U3, U24>; // capella big endian [3, 0, 0, 0]

    type MinSeedLookahead = U1;
    type EpochsPerHistoricalVector = U65536;

    type MinPerEpochChurnLimit = U4;
    type ChurnLimitQuotient = U65536;
    type MaxDeposits = U16;
    // --- Gindex Constants ---
    // 43
    type ValidatorsRootGindex = U43;
    // 41
    type ValidatorsTreeDepth = U41;
    // 94557999988736
    type Validators0Gindex = Prod<Exp<U2, U41>, U43>;
    // 13
    type ActivationEpochGindex = U13;
    // 14
    type ExitEpochGindex = U14;
    // 8
    type PubkeyGindex = U8;
    // 10
    type EffectiveBalanceGindex = U10;
    // 87
    type ValidatorsLengthGindex = Sum<Prod<Self::ValidatorsRootGindex, U2>, U1>;
    // 49
    type JustificationBitsGindex = U49;
    // 50
    type PreviousJustifiedCheckpointGindex = U50;
    // 51
    type CurrentJustifiedCheckpointGindex = U51;
    // 52
    type FinalizedCheckpointGindex = U52;
    // 45
    type RandaoMixesRootGindex = U45;
    // 16
    type RandaoMixesDepth = U16;
    // 2949120
    type RandaoMixes0Gindex = Prod<Prod<Exp<U2, U16>, U9>, U5>;

    fn genesis_validators_root() -> ssz_rs::Node {
        Node::try_from(
            hex!("4b363db94e286120d76eb905340fdd4e54bfe9f06bf33ff6cf5ad27f511bfe95").as_slice(),
        )
        .unwrap()
    }
}

/// Ethereum Foundation specifications.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct SpecTestSpec;

impl Spec for SpecTestSpec {
    type SlotsPerEpoch = U32;
    type MaxCommitteesPerSlot = U64;
    type TargetCommitteeSize = U128;
    type ShuffleRoundCount = U90;

    type DomainBeaconAttester = U1;
    type ForkVersion = Shleft<U2, U24>;

    type MinSeedLookahead = U1;
    type EpochsPerHistoricalVector = U65536;

    type MinPerEpochChurnLimit = U4;
    type ChurnLimitQuotient = U65536;
    type MaxDeposits = U16;
    // --- Gindex Constants ---
    // 43
    type ValidatorsRootGindex = U43;
    // 41
    type ValidatorsTreeDepth = U41;
    // 94557999988736
    type Validators0Gindex = Prod<Exp<U2, U41>, U43>;
    // 13
    type ActivationEpochGindex = U13;
    // 14
    type ExitEpochGindex = U14;
    // 8
    type PubkeyGindex = U8;
    // 10
    type EffectiveBalanceGindex = U10;
    // 87
    type ValidatorsLengthGindex = Sum<Prod<Self::ValidatorsRootGindex, U2>, U1>;
    // 49
    type JustificationBitsGindex = U49;
    // 50
    type PreviousJustifiedCheckpointGindex = U50;
    // 51
    type CurrentJustifiedCheckpointGindex = U51;
    // 52
    type FinalizedCheckpointGindex = U52;
    // 45
    type RandaoMixesRootGindex = U45;
    // 16
    type RandaoMixesDepth = U16;
    // 2949120
    type RandaoMixes0Gindex = Prod<Prod<Exp<U2, U16>, U9>, U5>;

    fn genesis_validators_root() -> ssz_rs::Node {
        // find this in a state object from the chain
        // TODO: This is from the spec tests. Unsure if it is the correct one for mainnet
        // TODO: This wont work for spec tests.
        Node::try_from(
            hex!("ef82b97f46b3decc813a5c37fe2cb679d084de70ae42e1eaef3a6a90da2b361a").as_slice(),
        )
        .unwrap()
    }
}

/// Ethereum Foundation minimal spec, as defined in the eth2.0-specs repo.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct MinimalSpec;

impl Spec for MinimalSpec {
    type SlotsPerEpoch = U32;
    type MaxCommitteesPerSlot = U4;
    type TargetCommitteeSize = U4;
    type ShuffleRoundCount = U10;

    type DomainBeaconAttester = U1;
    type ForkVersion = Sum<Shleft<U2, U24>, U1>; // bellatrix minimal big endian [2, 0, 0, 1]

    type MinSeedLookahead = U1;
    type EpochsPerHistoricalVector = U64;

    type MinPerEpochChurnLimit = U4;
    type ChurnLimitQuotient = U32;
    type MaxDeposits = U16;
    // --- Gindex Constants ---
    // 43
    type ValidatorsRootGindex = U43;
    // 41
    type ValidatorsTreeDepth = U41;
    // 94557999988736
    type Validators0Gindex = Prod<Exp<U2, U41>, U43>;
    // 13
    type ActivationEpochGindex = U13;
    // 14
    type ExitEpochGindex = U14;
    // 8
    type PubkeyGindex = U8;
    // 10
    type EffectiveBalanceGindex = U10;
    // 87
    type ValidatorsLengthGindex = Sum<Prod<Self::ValidatorsRootGindex, U2>, U1>;
    // 49
    type JustificationBitsGindex = U49;
    // 50
    type PreviousJustifiedCheckpointGindex = U50;
    // 51
    type CurrentJustifiedCheckpointGindex = U51;
    // 52
    type FinalizedCheckpointGindex = U52;
    // 45
    type RandaoMixesRootGindex = U45;
    // 16
    type RandaoMixesDepth = U16;
    // 2949120
    type RandaoMixes0Gindex = Prod<Prod<Exp<U2, U16>, U9>, U5>;
    // TODO: Add other fields we might need

    fn genesis_validators_root() -> ssz_rs::Node {
        Node::try_from(
            hex!("4b363db94e286120d76eb905340fdd4e54bfe9f06bf33ff6cf5ad27f511bfe95").as_slice(),
        )
        .unwrap()
    }
}
