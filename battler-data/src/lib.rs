#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

#[cfg(feature = "typescript")]
extern crate std;

mod abilities;
mod clauses;
mod common;
mod conditions;
mod datastore;
mod items;
mod mons;
mod moves;

#[cfg(test)]
pub mod test_util;

pub use abilities::*;
pub use clauses::*;
pub use common::*;
pub use conditions::*;
pub use datastore::*;
pub use items::*;
pub use mons::*;
pub use moves::*;

#[cfg(test)]
#[cfg(feature = "typescript")]
mod typescript_tests {
    use ts_rs::TS;

    use super::*;

    #[test]
    fn export_types() {
        AbilityData::export().unwrap();
        AbilityFlag::export().unwrap();
        ConditionData::export().unwrap();
        ConditionType::export().unwrap();
        FlingData::export().unwrap();
        NaturalGiftData::export().unwrap();
        JudgmentData::export().unwrap();
        TechnoBlastData::export().unwrap();
        MultiAttackData::export().unwrap();
        MegaEvolutionData::export().unwrap();
        ZCrystalSource::export().unwrap();
        UltraBurstData::export().unwrap();
        ZCrystalData::export().unwrap();
        SpecialItemData::export().unwrap();
        ItemTarget::export().unwrap();
        ItemInput::export().unwrap();
        ItemFlag::export().unwrap();
        ItemData::export().unwrap();
        Color::export().unwrap();
        LevelingRate::export().unwrap();
        SpeciesFlag::export().unwrap();
        EvolutionMethod::export().unwrap();
        EvolutionData::export().unwrap();
        ShinyChance::export().unwrap();
        EventData::export().unwrap();
        SpeciesData::export().unwrap();
        MoveCategory::export().unwrap();
        MoveFlag::export().unwrap();
        MonOverride::export().unwrap();
        MultihitType::export().unwrap();
        OhkoType::export().unwrap();
        SwitchType::export().unwrap();
        SelfDestructType::export().unwrap();
        Accuracy::export().unwrap();
        HitEffect::export().unwrap();
        SecondaryEffectData::export().unwrap();
        ZPower::export().unwrap();
        ZMoveData::export().unwrap();
        MaxMoveData::export().unwrap();
        RecoilBase::export().unwrap();
        RecoilData::export().unwrap();
        AdvancedTargeting::export().unwrap();
        MoveData::export().unwrap();
        Fraction::<u8>::export().unwrap();
    }
}
