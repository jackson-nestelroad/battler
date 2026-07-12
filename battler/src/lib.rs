#![cfg_attr(not(feature = "std"), no_std)]
#![feature(const_trait_impl)]
#![feature(pattern)]
#![feature(try_trait_v2)]

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

pub mod abilities;
pub mod battle;
pub mod common;
pub mod conditions;
pub mod config;
pub mod dex;
pub mod effect;
pub mod error;
pub mod items;
pub mod log;
pub mod mons;
pub mod moves;
pub mod teams;

// Public exports.
//
// In theory, everything is public, but everything re-exported at the root level is intended
// for users.
pub use battle::{
    BattleType,
    CoreBattleEngineOptions,
    CoreBattleEngineRandomizeBaseDamage,
    CoreBattleEngineSpeedSortTieResolution,
    CoreBattleOptions,
    ExperienceOptions,
    ExperienceShareOptions,
    FieldData,
    FieldEnvironment,
    LearnMoveRequest,
    MonBattleData,
    MonLearnMoveRequest,
    MonMoveRequest,
    MonMoveSlotData,
    MonPersistentMoveData,
    MonPositionDetails,
    MonSummaryData,
    OutsideEffect,
    OutsideEffectTarget,
    PlayerBattleData,
    PlayerData,
    PlayerDex,
    PlayerOptions,
    PlayerType,
    PublicCoreBattle,
    Request,
    RequestType,
    SelectPosition,
    SelectReason,
    SelectRequest,
    SideData,
    SwitchRequest,
    TeamPreviewRequest,
    TimeOfDay,
    TurnRequest,
    WildEncounterType,
    WildPlayerOptions,
};
pub use battler_data::*;
pub use config::FormatData;
pub use dex::{
    AbilityDex,
    ClauseDex,
    ConditionDex,
    Dex,
    ItemDex,
    MoveDex,
    ResourceDex,
    SingleValueDex,
    SpeciesDex,
};
pub use effect::fxlang::{
    Callback,
    CallbackInput,
    Callbacks,
    ConditionAttributes,
    Effect,
    EffectAttributes,
    LocalData,
    Program,
    ProgramWithPriority,
};
pub use error::{
    BorrowFailedError,
    ConvertError,
    GeneralError,
    IntegerOverflowError,
    NotFoundError,
    ValidationError,
    WrapError,
    WrapOptionError,
    WrapResultError,
    borrow_failed_error,
    general_error,
    integer_overflow_error,
    not_found_error,
};
pub use teams::{
    BagData,
    MonData,
    MonPersistentBattleData,
    TeamData,
};

#[cfg(all(test, feature = "typescript"))]
mod typescript_tests {
    use ts_rs::TS;

    use super::*;

    #[test]
    fn export_types() {
        CoreBattleOptions::export().unwrap();
        FieldData::export().unwrap();
        FieldEnvironment::export().unwrap();
        TimeOfDay::export().unwrap();
        SideData::export().unwrap();
        PlayerData::export().unwrap();
        PlayerType::export().unwrap();
        WildPlayerOptions::export().unwrap();
        WildEncounterType::export().unwrap();
        PlayerOptions::export().unwrap();
        ExperienceOptions::export().unwrap();
        ExperienceShareOptions::export().unwrap();
        PlayerDex::export().unwrap();
        FormatData::export().unwrap();
        BattleType::export().unwrap();
        TeamData::export().unwrap();
        BagData::export().unwrap();
        MonData::export().unwrap();
        MonPersistentBattleData::export().unwrap();
        Type::export().unwrap();
        Nature::export().unwrap();
        Gender::export().unwrap();
        StatTable::export().unwrap();
        Stat::export().unwrap();
        Boost::export().unwrap();
        BoostTable::export().unwrap();
        MoveTarget::export().unwrap();
        Request::export().unwrap();
        RequestType::export().unwrap();
        TeamPreviewRequest::export().unwrap();
        TurnRequest::export().unwrap();
        SwitchRequest::export().unwrap();
        LearnMoveRequest::export().unwrap();
        SelectReason::export().unwrap();
        SelectPosition::export().unwrap();
        SelectRequest::export().unwrap();
        MonMoveSlotData::export().unwrap();
        MonPersistentMoveData::export().unwrap();
        MonSummaryData::export().unwrap();
        MonBattleData::export().unwrap();
        MonMoveRequest::export().unwrap();
        MonLearnMoveRequest::export().unwrap();
        PlayerBattleData::export().unwrap();
        Id::export().unwrap();
    }
}
