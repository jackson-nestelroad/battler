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
pub mod rng;
pub mod teams;

// Public exports.
//
// In theory, everything is public, but everything re-exported at the root level is intended
// for users.
pub use battle::{
    ActiveMonDetails,
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
    PlayerOptions,
    PlayerType,
    PublicCoreBattle,
    PublicMonDetails,
    Request,
    RequestType,
    SideData,
    SwitchRequest,
    TeamPreviewRequest,
    TurnRequest,
    WildPlayerOptions,
};
pub use battler_data::*;
pub use config::{
    FormatData,
    FormatOptions,
};
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
    Condition,
    Effect,
    LocalData,
    Program,
    ProgramWithPriority,
};
pub use error::{
    borrow_failed_error,
    general_error,
    integer_overflow_error,
    not_found_error,
    BorrowFailedError,
    ConvertError,
    GeneralError,
    IntegerOverflowError,
    NotFoundError,
    ValidationError,
    WrapError,
    WrapOptionError,
    WrapResultError,
};
pub use rng::{
    PseudoRandomNumberGenerator,
    RealPseudoRandomNumberGenerator,
};
pub use teams::{
    BagData,
    MonData,
    MonPersistentBattleData,
    TeamData,
};
