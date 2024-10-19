#![feature(const_trait_impl)]
#![feature(hasher_prefixfree_extras)]
#![feature(pattern)]
#![feature(trait_upcasting)]
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
// In theory, everything is public, but everything re-exported at the root level is intended for
// users.
pub use abilities::{
    AbilityData,
    AbilityFlags,
};
pub use battle::{
    ActiveMonDetails,
    BattleBuilder,
    BattleBuilderFlags,
    BattleBuilderOptions,
    BattleBuilderPlayerData,
    BattleBuilderSideData,
    BattleType,
    Boost,
    BoostOrderIterator,
    BoostTable,
    BoostTableEntries,
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
pub use common::{
    FastHashMap,
    FastHashSet,
    Fraction,
    FractionInteger,
    Id,
};
pub use conditions::{
    ConditionData,
    ConditionType,
};
pub use config::{
    ClauseData,
    ClauseValueType,
    FormatData,
    FormatOptions,
    Rule,
    SerializedRuleSet,
};
pub use dex::{
    AbilityDex,
    ClauseDex,
    ConditionDex,
    DataStore,
    Dex,
    ItemDex,
    LocalDataStore,
    MoveDex,
    ResourceDex,
    SerializedAliases,
    SerializedDataTable,
    SingleValueDex,
    SpeciesDex,
};
pub use error::{
    borrow_failed_error,
    general_error,
    integer_overflow_error,
    not_found_error,
    BorrowFailedError,
    ConvertError,
    Error,
    GeneralError,
    IntegerOverflowError,
    NotFoundError,
    TeamValidationError,
    WrapError,
    WrapOptionError,
    WrapResultError,
};
pub use items::{
    ItemData,
    ItemFlags,
    ItemInput,
    ItemTarget,
};
pub use mons::{
    Color,
    EventData,
    EvolutionData,
    EvolutionMethod,
    Gender,
    LearnSet,
    LevelingRate,
    MoveSource,
    Nature,
    PartialStatTable,
    ShinyChance,
    SpeciesData,
    SpeciesFlags,
    Stat,
    StatMap,
    StatTableEntries,
    Type,
    TypeChart,
    TypeEffectiveness,
    TypeTable,
};
pub use moves::{
    Accuracy,
    HitEffect,
    MonOverride,
    MoveCategory,
    MoveData,
    MoveFlags,
    MoveTarget,
    MultihitType,
    OhkoType,
    SecondaryEffect,
    SelfDestructType,
    SwitchType,
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
