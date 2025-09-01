#![feature(const_trait_impl)]
#![feature(hasher_prefixfree_extras)]
#![feature(pattern)]
#![feature(try_trait_v2)]

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
    PlayerBattleData,
    PlayerData,
    PlayerDex,
    PlayerOptions,
    PlayerType,
    PublicCoreBattle,
    Request,
    RequestType,
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
