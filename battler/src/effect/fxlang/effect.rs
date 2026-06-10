use alloc::{
    string::{
        String,
        ToString,
    },
    vec::Vec,
};

use anyhow::Error;
use hashbrown::HashMap;
use serde::{
    Deserialize,
    Serialize,
};
use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

use crate::{
    WrapResultError,
    battle::SpeedOrderable,
    effect::fxlang::{
        LocalData,
        ValueType,
    },
};

/// Flags used to indicate the input and output of a [`Callback`].
#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
pub mod CallbackFlag {
    pub const TakesGeneralMon: u32 = 1 << 1;
    pub const TakesTargetMon: u32 = 1 << 2;
    pub const TakesSourceMon: u32 = 1 << 3;
    pub const TakesEffect: u32 = 1 << 4;
    pub const TakesActiveMove: u32 = 1 << 5;
    pub const TakesUserMon: u32 = 1 << 6;
    pub const TakesSourceTargetMon: u32 = 1 << 7;
    pub const TakesSourceEffect: u32 = 1 << 8;
    pub const TakesSide: u32 = 1 << 9;
    pub const TakesOptionalEffect: u32 = 1 << 10;
    pub const TakesPlayer: u32 = 1 << 11;

    pub const ReturnsActiveMove: u32 = 1 << 18;
    pub const ReturnsType: u32 = 1 << 19;
    pub const ReturnsStatTable: u32 = 1 << 20;
    pub const ReturnsMoveTarget: u32 = 1 << 21;
    pub const ReturnsStrings: u32 = 1 << 22;
    pub const ReturnsSecondaryEffects: u32 = 1 << 23;
    pub const ReturnsTypes: u32 = 1 << 24;
    pub const ReturnsMon: u32 = 1 << 25;
    pub const ReturnsBoosts: u32 = 1 << 26;
    pub const ReturnsString: u32 = 1 << 27;
    pub const ReturnsEventResult: u32 = 1 << 28;
    pub const ReturnsNumber: u32 = 1 << 29;
    pub const ReturnsBoolean: u32 = 1 << 30;
    pub const ReturnsVoid: u32 = 1 << 31;
}

/// Common types of [`Callback`]s, defined for convenience.
///
/// - `ApplyingEffect` - An effect being applied to a target Mon, potentially from a source Mon. The
///   focus is on the applying effect itself.
/// - `Effect` - Same as `ApplyingEffect`, but the applying effect is considered to be the "source
///   effect."
/// - `SourceMove` - An active move being used by a Mon, potentially with a target.
/// - `Move` - An active move being used by a Mon against a target.
/// - `Mon` - A callback on the Mon itself, with no associated effect.
/// - `Side` - A callback on the side itself, with no associated effect, potentially with a source
///   Mon.
/// - `MoveSide` - An active move being used by a Mon against a side.
/// - `MoveField` - An active move being used by a Mon against the field.
#[repr(u32)]
enum CommonCallbackType {
    ApplyingEffectModifier = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesEffect
        | CallbackFlag::ReturnsNumber
        | CallbackFlag::ReturnsVoid,
    ApplyingEffectBoolean = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesEffect
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsVoid,
    ApplyingEffectResult = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesEffect
        | CallbackFlag::ReturnsEventResult
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsString
        | CallbackFlag::ReturnsVoid,
    ApplyingEffectVoid = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesEffect
        | CallbackFlag::ReturnsVoid,
    ApplyingEffectBoostModifier = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesEffect
        | CallbackFlag::ReturnsBoosts
        | CallbackFlag::ReturnsVoid,

    MaybeApplyingEffectVoid = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesOptionalEffect
        | CallbackFlag::ReturnsVoid,
    MaybeApplyingEffectModifier = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesOptionalEffect
        | CallbackFlag::ReturnsNumber
        | CallbackFlag::ReturnsVoid,
    MaybeApplyingEffectBoostModifier = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesOptionalEffect
        | CallbackFlag::ReturnsBoosts
        | CallbackFlag::ReturnsVoid,

    EffectBoolean = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesSourceEffect
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsVoid,
    EffectResult = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesSourceEffect
        | CallbackFlag::ReturnsEventResult
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsString
        | CallbackFlag::ReturnsVoid,
    EffectVoid = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesSourceEffect
        | CallbackFlag::ReturnsVoid,

    NoContextBoolean = CallbackFlag::ReturnsBoolean | CallbackFlag::ReturnsVoid,
    NoContextVoid = CallbackFlag::ReturnsVoid,

    SourceMoveModifier = CallbackFlag::TakesUserMon
        | CallbackFlag::TakesSourceTargetMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsNumber
        | CallbackFlag::ReturnsVoid,
    SourceMoveResult = CallbackFlag::TakesUserMon
        | CallbackFlag::TakesSourceTargetMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsEventResult
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsString
        | CallbackFlag::ReturnsVoid,
    SourceMoveVoid = CallbackFlag::TakesUserMon
        | CallbackFlag::TakesSourceTargetMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsVoid,
    SourceMoveMonModifier = CallbackFlag::TakesUserMon
        | CallbackFlag::TakesSourceTargetMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsMon
        | CallbackFlag::ReturnsVoid,
    SourceMoveActiveMove = CallbackFlag::TakesUserMon
        | CallbackFlag::TakesSourceTargetMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsActiveMove
        | CallbackFlag::ReturnsVoid,

    SourceEffectType = CallbackFlag::TakesUserMon
        | CallbackFlag::TakesSourceTargetMon
        | CallbackFlag::TakesEffect
        | CallbackFlag::ReturnsType
        | CallbackFlag::ReturnsString
        | CallbackFlag::ReturnsVoid,

    MoveModifier = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsNumber
        | CallbackFlag::ReturnsVoid,
    MoveBoolean = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsVoid,
    MoveVoid = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsVoid,
    MoveHitOutcomeResult = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsNumber
        | CallbackFlag::ReturnsEventResult
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsString
        | CallbackFlag::ReturnsVoid,
    MoveResult = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsEventResult
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsString
        | CallbackFlag::ReturnsVoid,
    MoveSecondaryEffectModifier = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsSecondaryEffects
        | CallbackFlag::ReturnsVoid,

    MonModifier =
        CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsNumber | CallbackFlag::ReturnsVoid,
    MonBoolean =
        CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsBoolean | CallbackFlag::ReturnsVoid,
    MonResult = CallbackFlag::TakesGeneralMon
        | CallbackFlag::ReturnsEventResult
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsString
        | CallbackFlag::ReturnsVoid,
    MonVoid = CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsVoid,
    MonInfo =
        CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsString | CallbackFlag::ReturnsVoid,
    MonType = CallbackFlag::TakesGeneralMon
        | CallbackFlag::ReturnsType
        | CallbackFlag::ReturnsString
        | CallbackFlag::ReturnsVoid,
    MonTypes =
        CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsTypes | CallbackFlag::ReturnsVoid,
    MonBoostModifier =
        CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsBoosts | CallbackFlag::ReturnsVoid,
    MonValidator = CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsStrings,
    MonMoveTarget =
        CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsMoveTarget | CallbackFlag::ReturnsVoid,
    MonStatTableModifier =
        CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsStatTable | CallbackFlag::ReturnsVoid,

    PlayerValidator = CallbackFlag::TakesPlayer | CallbackFlag::ReturnsStrings,

    PlayerEffectVoid = CallbackFlag::TakesPlayer
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesEffect
        | CallbackFlag::ReturnsVoid,

    SideVoid = CallbackFlag::TakesSide
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesSourceEffect
        | CallbackFlag::ReturnsVoid,
    SideResult = CallbackFlag::TakesSide
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesSourceEffect
        | CallbackFlag::ReturnsEventResult
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsString
        | CallbackFlag::ReturnsVoid,

    SideEffectVoid = CallbackFlag::TakesSide
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesEffect
        | CallbackFlag::ReturnsVoid,
    SideEffectModifier = CallbackFlag::TakesSide
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesEffect
        | CallbackFlag::ReturnsNumber
        | CallbackFlag::ReturnsVoid,

    MoveSideResult = CallbackFlag::TakesSide
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsEventResult
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsString
        | CallbackFlag::ReturnsVoid,

    MoveFieldResult = CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsEventResult
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsString
        | CallbackFlag::ReturnsVoid,

    FieldVoid =
        CallbackFlag::TakesSourceMon | CallbackFlag::TakesSourceEffect | CallbackFlag::ReturnsVoid,
    FieldResult = CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesSourceEffect
        | CallbackFlag::ReturnsEventResult
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsString
        | CallbackFlag::ReturnsVoid,

    FieldEffectResult = CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesEffect
        | CallbackFlag::ReturnsEventResult
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsString
        | CallbackFlag::ReturnsVoid,
    FieldEffectVoid =
        CallbackFlag::TakesSourceMon | CallbackFlag::TakesEffect | CallbackFlag::ReturnsVoid,
    FieldEffectModifier = CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesEffect
        | CallbackFlag::ReturnsNumber
        | CallbackFlag::ReturnsVoid,
}

/// A modifier on a [`BattleEvent`].
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    SerializeLabeledStringEnum,
    DeserializeLabeledStringEnum,
)]
pub enum BattleEventModifier {
    /// The default event.
    #[default]
    #[string = ""]
    None,
    /// Runs for an ally of the target Mon.
    #[string = "ally"]
    Ally,
    /// Runs for any Mon.
    #[string = "any"]
    Any,
    /// Runs on the field.
    #[string = "field"]
    Field,
    /// Runs for a foe of the target Mon.
    #[string = "foe"]
    Foe,
    /// Runs for the side of the target Mon.
    #[string = "side"]
    Side,
    /// Runs for the source of the effect.
    #[string = "source"]
    Source,
}

/// A battle event that can trigger a [`Callback`].
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    SerializeLabeledStringEnum,
    DeserializeLabeledStringEnum,
)]
pub enum BattleEvent {
    /// Runs when the accuracy check of a move against a target fails.
    ///
    /// Runs in the context of a move target.
    #[string = "AccuracyCheckFailed"]
    AccuracyCheckFailed,
    /// Runs when the accuracy of a move against a target is being determined.
    ///
    /// Runs in the context of a move target.
    #[string = "AccuracyExempt"]
    AccuracyExempt,
    /// Runs when an effect activates.
    ///
    /// Runs when activated by a battle effect. Used for shared logic between multiple event
    /// callbacks.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "Activate"]
    Activate,
    /// Runs when an effect activates.
    ///
    /// Runs when activated by a battle effect. Used for shared logic between multiple event
    /// callbacks.
    ///
    /// Runs in the context of an applying effect on the field.
    #[string = "ActivateField"]
    ActivateField,
    /// Runs when an effect activates.
    ///
    /// Runs when activated by a battle effect. Used for shared logic between multiple event
    /// callbacks.
    ///
    /// Runs in the context of an applying effect on a player.
    #[string = "ActivatePlayer"]
    ActivatePlayer,
    /// Runs when an effect activates.
    ///
    /// Runs when activated by a battle effect. Used for shared logic between multiple event
    /// callbacks.
    ///
    /// Runs in the context of an applying effect on a side.
    #[string = "ActivateSide"]
    ActivateSide,
    /// Runs when a pseudo-weather is being added to the field.
    ///
    /// Runs before the pseudo-weather effect is applied. Can be used to fail the pseudo-weather.
    ///
    /// Runs in the context of an applying effect on the field.
    #[string = "AddPseudoWeather"]
    AddPseudoWeather,
    /// Runs when a type is being added to a Mon.
    ///
    /// Runs before the type addition is applied. Can be used to fail the type addition.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "AddType"]
    AddType,
    /// Runs after a volatile effect is added to a Mon.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "AddVolatile"]
    AddVolatile,
    /// Runs after a new pseudo-weather is added to the field.
    ///
    /// Only runs if the pseudo-weather has been added successfully. This event will not undo the
    /// pseudo-weather.
    ///
    /// Runs in the context of an applying effect on the field.
    #[string = "AfterAddPseudoWeather"]
    AfterAddPseudoWeather,
    /// Runs after a Mon receives a new volatile effect.
    ///
    /// Only runs if the volatile has been added successfully. This event will not undo the
    /// volatile.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "AfterAddVolatile"]
    AfterAddVolatile,
    /// Runs after stat boosts are applied.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "AfterBoost"]
    AfterBoost,
    /// Runs after a Mon's current status is cured.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "AfterCureStatus"]
    AfterCureStatus,
    /// Runs after a Mon takes damage.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "AfterDamage"]
    AfterDamage,
    /// Runs after an individual stat boost is applied.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "AfterEachBoost"]
    AfterEachBoost,
    /// Runs after a Mon causes one or more Mons to faint.
    ///
    /// Runs in the context of a Mon.
    #[string = "AfterFainted"]
    AfterFainted,
    /// Runs after a Mon heals.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "AfterHeal"]
    AfterHeal,
    /// Runs after a Mon hits another Mon with a move.
    ///
    /// Runs on the active move.
    #[string = "AfterHit"]
    AfterHit,
    /// Runs after a Mon Mega Evolves.
    ///
    /// Runs in the context of a Mon.
    #[string = "AfterMegaEvolution"]
    AfterMegaEvolution,
    /// Runs after a Mon finishes using a move.
    ///
    /// Runs on the active move and in the context of a move user.
    #[string = "AfterMove"]
    AfterMove,
    /// Runs after a move's secondary effects have been applied, for all targets the move was
    /// successful against.
    ///
    /// Should be viewed as the last effect the move needs to apply on the target.
    ///
    /// Runs on the active move and in the context of a move target.
    #[string = "AfterMoveSecondaryEffects"]
    AfterMoveSecondaryEffects,
    /// Runs after a move's secondary effects have been applied, for all targets affected by damage.
    ///
    /// Should be viewed as the last effect the move needs to apply on the target. Minimal
    /// difference with `AfterMove`; the key difference is that Sheer Force prevents this event from
    /// running.
    ///
    /// Runs on the active move and in the context of a move target.
    #[string = "AfterMoveSecondaryEffectsDamage"]
    AfterMoveSecondaryEffectsDamage,
    /// Runs after a move's secondary effects have been applied.
    ///
    /// Should be viewed as the last effect the move needs to apply on the user. Minimal difference
    /// with `AfterMove`; the key difference is that Sheer Force prevents this event from running.
    ///
    /// Runs on the active move and in the context of a move user.
    #[string = "AfterMoveSecondaryEffectsUser"]
    AfterMoveSecondaryEffectsUser,
    /// Runs after a Mon has its ability set.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "AfterSetAbility"]
    AfterSetAbility,
    /// Runs after a Mon has its item set.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "AfterSetItem"]
    AfterSetItem,
    /// Runs after a Mon's status effect is changed.
    ///
    /// Only runs if the status has been set successfully. This event will not undo a status
    /// change.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "AfterSetStatus"]
    AfterSetStatus,
    /// Runs after damage is applied to a substitute.
    ///
    /// Hitting a substitute does not trigger ordinary effects that run when a target is hit. Thus,
    /// this event is used to cover for scenarios where hitting a substitute should still trigger
    /// some callback.
    ///
    /// Runs on the active move and in the context of a move target.
    #[string = "AfterSubstituteDamage"]
    AfterSubstituteDamage,
    /// Runs after a Mon has its item taken.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "AfterTakeItem"]
    AfterTakeItem,
    /// Runs after a Mon Terastallizes.
    ///
    /// Runs in the context of a Mon.
    #[string = "AfterTerastallization"]
    AfterTerastallization,
    /// Runs after a Mon uses its item.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "AfterUseItem"]
    AfterUseItem,
    /// Runs when a move's base power is being calculated for a target.
    ///
    /// Runs on the active move and in the context of a move target.
    #[string = "BasePower"]
    BasePower,
    /// Runs before a turn of a battle ends.
    ///
    /// Runs in the context of the battle.
    #[string = "BattleEndTurn"]
    BattleEndTurn,
    /// Runs when a Mon is using a charge move, on the charging turn.
    ///
    /// Runs in the context of a move user.
    #[string = "BeforeChargeMove"]
    BeforeChargeMove,
    /// Runs before a Mon Dynamaxes.
    ///
    /// Runs in the context of a Mon.
    #[string = "BeforeDynamax"]
    BeforeDynamax,
    /// Runs before a Mon uses a move.
    ///
    /// Can prevent the move from being used.
    ///
    /// Runs on the active move and in the context of a move user.
    #[string = "BeforeMove"]
    BeforeMove,
    /// Runs before an effect starts.
    ///
    /// Used to set up state prior to the Start event.
    ///
    /// Runs on the effect.
    #[string = "BeforeStart"]
    BeforeStart,
    /// Runs when a Mon switches in, prior to `SwitchIn`.
    ///
    /// Not really prior to switching in.
    ///
    /// Runs in the context of a Mon.
    #[string = "BeforeSwitchIn"]
    BeforeSwitchIn,
    /// Runs before a Mon switches out.
    ///
    /// Runs in the context of a Mon.
    #[string = "BeforeSwitchOut"]
    BeforeSwitchOut,
    /// Runs before a Mon Terastallizes.
    ///
    /// Runs in the context of a Mon.
    #[string = "BeforeTerastallization"]
    BeforeTerastallization,
    /// Runs before a turn of a battle.
    ///
    /// Runs on the move and in the context of a move user.
    #[string = "BeforeTurn"]
    BeforeTurn,
    /// Runs when determining the health at which the Mon should eat berries.
    ///
    /// Runs in the context of a Mon.
    #[string = "BerryEatingHealth"]
    BerryEatingHealth,
    /// Runs when determining if a Mon can Dynamax.
    ///
    /// Runs in the context of a Mon.
    #[string = "CanDynamax"]
    CanDynamax,
    /// Runs when a Mon is attempting to escape from battle.
    ///
    /// Runs in the context of a Mon.
    #[string = "CanEscape"]
    CanEscape,
    /// Runs when determining if a Mon can heal.
    ///
    /// Runs in the context of a Mon.
    #[string = "CanHeal"]
    CanHeal,
    /// Runs when a Mon is caught.
    ///
    /// Runs on the item (used to catch the Mon) and in the context of a Mon.
    #[string = "Catch"]
    Catch,
    /// Runs when a Mon fails to be caught.
    ///
    /// Runs on the item (used to catch the Mon) and in the context of a Mon.
    #[string = "CatchFailed"]
    CatchFailed,
    /// Runs when a group of stat boosts is being applied to a Mon.
    ///
    /// Runs in the context of a Mon.
    #[string = "ChangeBoosts"]
    ChangeBoosts,
    /// Runs when a Mon's stat is being calculated.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "CalculateStat"]
    CalculateStat,
    /// Runs when a Mon is using a charge move, on the charging turn.
    ///
    /// Runs on the active move and in the context of a move user.
    #[string = "ChargeMove"]
    ChargeMove,
    /// Runs when the field's terrain is being cleared.
    ///
    /// Runs before the terrain effect is cleared. Can be used to fail the clear.
    ///
    /// Runs in the context of an applying effect on the field.
    #[string = "ClearTerrain"]
    ClearTerrain,
    /// Runs when the field's weather is being cleared.
    ///
    /// Runs before the weather effect is cleared. Can be used to fail the clear.
    ///
    /// Runs in the context of an applying effect on the field.
    #[string = "ClearWeather"]
    ClearWeather,
    /// Runs when copying a volatile effect to the target Mon.
    ///
    /// Runs on the effect.
    #[string = "CopyVolatile"]
    CopyVolatile,
    /// Runs when a move critical hits a target.
    ///
    /// Runs in the context of a move target.
    #[string = "CriticalHit"]
    CriticalHit,
    /// Runs when a Mon's current status is cured.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "CureStatus"]
    CureStatus,
    /// Runs when a Mon's damage is being calculated for a target.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "Damage"]
    Damage,
    /// Runs after a Mon hits another Mon with a move, causing a nonzero amount of damage.
    ///
    /// Runs once per hit (i.e., multi-hit moves execute one event per hit).
    ///
    /// Runs in the context of a move target.
    #[string = "DamagingHit"]
    DamagingHit,
    /// Runs after a move is used that should have PP deducted.
    ///
    /// Runs in the context of a move user.
    #[string = "DeductPp"]
    DeductPp,
    /// Runs when determining which moves are disabled.
    ///
    /// Runs in the context of a Mon and on the move.
    #[string = "DisableMove"]
    DisableMove,
    /// Runs before a Mon is dragged out of battle.
    ///
    /// Can cancel the force switch.
    ///
    /// Runs in the context of a Mon.
    #[string = "DragOut"]
    DragOut,
    /// Runs when determining the duration of an effect.
    ///
    /// Runs on the effect.
    #[string = "Duration"]
    Duration,
    /// Runs when an item is eaten.
    ///
    /// Runs on the item.
    #[string = "Eat"]
    Eat,
    /// Runs when a Mon eats its item.
    ///
    /// Runs in the context of a Mon.
    #[string = "EatItem"]
    EatItem,
    /// Runs when determining the type effectiveness of a move.
    ///
    /// Runs on the effect and in the context of an applying effect on a Mon.
    #[string = "Effectiveness"]
    Effectiveness,
    /// Runs when an effect ends.
    ///
    /// Runs on the effect.
    #[string = "End"]
    End,
    /// Runs when a Mon is active when the battle has ended.
    ///
    /// Runs in the context of a Mon.
    #[string = "EndBattle"]
    EndBattle,
    /// Runs before a turn of a battle ends.
    ///
    /// Runs in the context of a Mon.
    #[string = "EndTurn"]
    EndTurn,
    /// Runs when a Mon exits the battle (is no longer active).
    ///
    /// Runs in the context of a Mon.
    #[string = "Exit"]
    Exit,
    /// Runs when a Mon faints.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "Faint"]
    Faint,
    /// Runs when a field condition ends.
    ///
    /// Runs on the field condition.
    #[string = "FieldEnd"]
    FieldEnd,
    /// Runs at the end of every turn to apply residual effects on the field.
    ///
    /// Runs on the field condition.
    #[string = "FieldResidual"]
    FieldResidual,
    /// Runs when a field condition restarts.
    ///
    /// Runs on the field condition.
    #[string = "FieldRestart"]
    FieldRestart,
    /// Runs when a field condition starts.
    ///
    /// Runs on the field condition.
    #[string = "FieldStart"]
    FieldStart,
    /// Runs when a Mon flinches.
    ///
    /// Runs in the context of the target Mon.
    #[string = "Flinch"]
    Flinch,
    /// Runs when determining the type effectiveness of an effect, to prevent normal type
    /// effectiveness from being used.
    ///
    /// Runs on the effect and in the context of an applying effect on a Mon.
    #[string = "ForceEffectiveness"]
    ForceEffectiveness,
    /// Runs when a Mon is attempting to escape from battle, prior to any speed check.
    ///
    /// Runs in the context of a Mon.
    #[string = "ForceEscape"]
    ForceEscape,
    /// Runs when determining if a Mon can terastallize.
    ///
    /// Runs in the context of a Mon.
    #[string = "ForceTeraType"]
    ForceTeraType,
    /// Runs when determining the types of a Mon, to force types early.
    ///
    /// Runs in the context of a Mon.
    #[string = "ForceTypes"]
    ForceTypes,
    /// Runs when a Mon is hit by a move.
    ///
    /// Can fail, but will only fail the move if everything else failed. Can be viewed as part of
    /// the applying hit effect.
    ///
    /// Runs on the active move and in the context of a move target.
    #[string = "Hit"]
    Hit,
    /// Runs when the field is hit by a move.
    ///
    /// Can fail, but will only fail the move if everything else failed. Can be viewed as part of
    /// the applying hit effect.
    ///
    /// Runs on the active move.
    #[string = "HitField"]
    HitField,
    /// Runs when a side is hit by a move.
    ///
    /// Can fail, but will only fail the move if everything else failed. Can be viewed as part of
    /// the applying hit effect.
    ///
    /// Runs on the active move and in the context of an applying effect on a side.
    #[string = "HitSide"]
    HitSide,
    /// Runs when a Mon uses a move that defines a user hit effect.
    ///
    /// Can fail, but will only fail the move if everything else failed. Can be viewed as part of
    /// the applying hit effect.
    ///
    /// Runs on the active move.
    #[string = "HitUser"]
    HitUser,
    /// Runs when determining if a move should ignore type immunity.
    ///
    /// Runs on the active move and in the context of a move target.
    #[string = "IgnoreImmunity"]
    IgnoreImmunity,
    /// Runs when determining if a Mon is immune to some status.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "Immunity"]
    Immunity,
    /// Runs when determining if a Mon is invulnerable to targeting moves.
    ///
    /// Runs as the very first step in a move.
    ///
    /// Runs in the context of a move target.
    #[string = "Invulnerability"]
    Invulnerability,
    /// Runs when determining if a Mon is asleep.
    ///
    /// Runs in the context of a Mon.
    #[string = "IsAsleep"]
    IsAsleep,
    /// Runs when determining if a Mon is away from the field (e.g., immobilized by Sky Drop).
    ///
    /// Runs in the context of a Mon.
    #[string = "IsAwayFromField"]
    IsAwayFromField,
    /// Runs when determining if a Mon is behind a substitute.
    ///
    /// Runs in the context of a Mon.
    #[string = "IsBehindSubstitute"]
    IsBehindSubstitute,
    /// Runs when determining if a Mon is locked into its previous choice.
    ///
    /// Runs in the context of a Mon.
    #[string = "IsChoiceLocked"]
    IsChoiceLocked,
    /// Runs when determining if a Mon is protected from making contact with other Mons.
    ///
    /// Runs in the context of a Mon.
    #[string = "IsContactProof"]
    IsContactProof,
    /// Runs when determining if a Mon is grounded.
    ///
    /// Runs in the context of a Mon.
    #[string = "IsGrounded"]
    IsGrounded,
    /// Runs when determining if a Mon is immune to entry hazards.
    ///
    /// Runs in the context of a Mon.
    #[string = "IsImmuneToEntryHazards"]
    IsImmuneToEntryHazards,
    /// Runs when determining if a weather includes raining.
    ///
    /// Runs in the context of a Mon.
    #[string = "IsRaining"]
    IsRaining,
    /// Runs when determining if a Mon is in a semi-invulnerable state.
    ///
    /// Runs in the context of a Mon.
    #[string = "IsSemiInvulnerable"]
    IsSemiInvulnerable,
    /// Runs when determining if a weather includes snowing.
    ///
    /// Runs in the context of a Mon.
    #[string = "IsSnowing"]
    IsSnowing,
    /// Runs when determining if a Mon is soundproof.
    ///
    /// Runs in the context of a Mon.
    #[string = "IsSoundproof"]
    IsSoundproof,
    /// Runs when determining if a weather includes sunny weather.
    ///
    /// Runs in the context of a Mon.
    #[string = "IsSunny"]
    IsSunny,
    /// Runs when determining if a Mon is locked into a move.
    ///
    /// Runs in the context of a Mon.
    #[string = "LockMove"]
    LockMove,
    /// Runs when calculating the accuracy of a move.
    ///
    /// Runs in the context of a move target.
    #[string = "ModifyAccuracy"]
    ModifyAccuracy,
    /// Runs when calculating the speed of an action.
    ///
    /// Runs in the context of a Mon.
    #[string = "ModifyActionSpeed"]
    ModifyActionSpeed,
    /// Runs when calculating a Mon's Atk stat.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "ModifyAtk"]
    ModifyAtk,
    /// Runs when modifying a Mon's stat boosts used for stat calculations.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "ModifyBoosts"]
    ModifyBoosts,
    /// Runs when calculating the modified catch rate of a Mon.
    ///
    /// Runs in the context of the item and in the context of an applying effect on a Mon.
    #[string = "ModifyCatchRate"]
    ModifyCatchRate,
    /// Runs when calculating a move's critical hit chance.
    ///
    /// Runs in the context of a move user.
    #[string = "ModifyCritChance"]
    ModifyCritChance,
    /// Runs when calculating a move's critical hit ratio.
    ///
    /// Runs in the context of a move user.
    #[string = "ModifyCritRatio"]
    ModifyCritRatio,
    /// Runs when calculating the damage applied to a Mon.
    ///
    /// Runs as the very last step in the regular damage calculation formula.
    ///
    /// Runs in the context of a move user.
    #[string = "ModifyDamage"]
    ModifyDamage,
    /// Runs when calculating a Mon's Def stat.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "ModifyDef"]
    ModifyDef,
    /// Runs when calculating the duration of a condition applying to a Mon.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "ModifyDuration"]
    ModifyDuration,
    /// Runs when determining the type effectiveness of a move.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "ModifyEffectiveness"]
    ModifyEffectiveness,
    /// Runs when calculating the EV yield gained by a Mon.
    ///
    /// Runs in the context of a Mon.
    #[string = "ModifyEvYield"]
    ModifyEvYield,
    /// Runs when calculating the amount of experience gained by a Mon.
    ///
    /// Runs in the context of a Mon.
    #[string = "ModifyExperience"]
    ModifyExperience,
    /// Runs when calculating the duration of a condition applying to the field.
    ///
    /// Runs in the context of an applying effect on the field.
    #[string = "ModifyFieldDuration"]
    ModifyFieldDuration,
    /// Runs when calculating the amount of friendship gained by a Mon.
    ///
    /// Runs in the context of a Mon.
    #[string = "ModifyFriendshipIncrease"]
    ModifyFriendshipIncrease,
    /// Runs when modifying the type of a move.
    ///
    /// Runs on the move and in the context of a move user.
    #[string = "ModifyMoveType"]
    ModifyMoveType,
    /// Runs when determining the priority of a move.
    ///
    /// Runs in the context of a move user.
    #[string = "ModifyPriority"]
    ModifyPriority,
    /// Runs before applying secondary move effects.
    ///
    /// Runs in the context of a move target.
    #[string = "ModifySecondaryEffects"]
    ModifySecondaryEffects,
    /// Runs when calculating the duration of a condition applying to a side.
    ///
    /// Runs in the context of an applying effect on a side.
    #[string = "ModifySideDuration"]
    ModifySideDuration,
    /// Runs when calculating the duration of a condition applying to a slot.
    ///
    /// Runs in the context of an applying effect on a side.
    #[string = "ModifySlotDuration"]
    ModifySlotDuration,
    /// Runs when calculating a Mon's SpA stat.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "ModifySpA"]
    ModifySpA,
    /// Runs when calculating a Mon's SpD stat.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "ModifySpD"]
    ModifySpD,
    /// Runs when calculating a Mon's Spe stat.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "ModifySpe"]
    ModifySpe,
    /// Runs when calculating the base species catch rate of a Mon.
    ///
    /// Runs in the context of the item and in the context of an applying effect on a Mon.
    #[string = "ModifySpeciesCatchRate"]
    ModifySpeciesCatchRate,
    /// Runs when calculating a move's STAB multiplier.
    ///
    /// Runs in the context of a move user.
    #[string = "ModifyStab"]
    ModifyStab,
    /// Runs before a move is used, to modify the target Mon.
    ///
    /// Runs on the active move and in the context of a move user.
    #[string = "ModifyTarget"]
    ModifyTarget,
    /// Runs when calculating a Mon's weight.
    ///
    /// Runs in the context of a Mon.
    #[string = "ModifyWeight"]
    ModifyWeight,
    /// Runs when a move is aborted due to failing the BeforeMove event.
    ///
    /// Runs on the active move and in the context of a move user.
    #[string = "MoveAborted"]
    MoveAborted,
    /// Runs when a move's base power is being calculated for a target.
    ///
    /// Runs on the active move.
    #[string = "MoveBasePower"]
    MoveBasePower,
    /// Runs when a move's damage is being calculated for a target.
    ///
    /// Runs on the active move.
    #[string = "MoveDamage"]
    MoveDamage,
    /// Runs when a move fails, only on the move itself.
    ///
    /// A move fails when it is successfully used by the user, but it does not hit or apply its
    /// primary effect to any targets.
    ///
    /// Runs on the active move.
    #[string = "MoveFailed"]
    MoveFailed,
    /// Runs when a move's target type is determined for a Mon selecting a move.
    ///
    /// Runs on the move.
    #[string = "MoveTargetOverride"]
    MoveTargetOverride,
    /// Runs when determining if a Mon's immunity against a single type should be negated.
    ///
    /// Runs in the context of a Mon.
    #[string = "NegateImmunity"]
    NegateImmunity,
    /// Runs when a Mon uses a move, to override the chosen move.
    ///
    /// Runs in the context of a Mon.
    #[string = "OverrideMove"]
    OverrideMove,
    /// Runs when determining if a move should be overwritten.
    ///
    /// Runs in the context of a Mon.
    #[string = "OverwriteMove"]
    OverwriteMove,
    /// Runs when determining the effective weather for a Mon. Overrides the weather without looking
    /// at the actual field weather or weather suppression effects.
    ///
    /// Runs in the context of a Mon.
    #[string = "OverrideWeather"]
    OverrideWeather,
    /// Runs when a player tries to choose to use an item.
    ///
    /// Runs on the item.
    #[string = "PlayerTryUseItem"]
    PlayerTryUseItem,
    /// Runs when an item is used on a Mon by a player.
    ///
    /// Runs on the item.
    #[string = "PlayerUse"]
    PlayerUse,
    /// Runs when applying any pre-move effect.
    ///
    /// Very similar to `UseMove`, except it runs after the move is announced.
    ///
    /// Runs in the context of a move user.
    #[string = "PreMoveEffect"]
    PreMoveEffect,
    /// Runs when a Mon is preparing to hit all of its targets with a move.
    ///
    /// Can fail the move.
    ///
    /// Runs on the active move and in the context of a move user.
    #[string = "PrepareHit"]
    PrepareHit,
    /// Runs when determining if a Mon can have items used on it.
    ///
    /// Runs in the context of a Mon.
    #[string = "PreventUsedItems"]
    PreventUsedItems,
    /// Runs before at the start of the turn, when a move is charging for the turn.
    ///
    /// Runs in the context of a move user.
    #[string = "PriorityChargeMove"]
    PriorityChargeMove,
    /// Runs when a move is going to target one Mon but can be redirected towards a different
    /// target.
    ///
    /// Runs in the context of a move user.
    #[string = "RedirectTarget"]
    RedirectTarget,
    /// Runs at the end of every turn to apply residual effects.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "Residual"]
    Residual,
    /// Runs when a volatile effect is applied to a Mon that already has the volatile effect.
    ///
    /// Runs on the effect.
    #[string = "Restart"]
    Restart,
    /// Runs when restoring PP to a move.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "RestorePp"]
    RestorePp,
    /// Runs when a Mon is selected for a Mon's active position.
    ///
    /// Runs in the context of a Mon.
    #[string = "Select"]
    Select,
    /// Runs when a Mon's ability is being set.
    ///
    /// Runs before the ability is changed. Can be used to fail the ability change.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "SetAbility"]
    SetAbility,
    /// Runs when an item is being given to a Mon.
    ///
    /// Can prevent the item from being set.
    ///
    /// Runs on the item and in the context of an applying effect on a Mon.
    #[string = "SetItem"]
    SetItem,
    /// Runs when the Mon's last move selected is being set.
    ///
    /// Runs in the context of a Mon.
    #[string = "SetLastMove"]
    SetLastMove,
    /// Runs when a Mon's status effect is being set.
    ///
    /// Runs before the status effect is applied. Can be used to fail the status change.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "SetStatus"]
    SetStatus,
    /// Runs when the field's terrain is being set.
    ///
    /// Runs before the terrain effect is applied. Can be used to fail the terrain.
    ///
    /// Runs in the context of an applying effect on the field.
    #[string = "SetTerrain"]
    SetTerrain,
    /// Runs when a Mon's types are being changed.
    ///
    /// Runs before the types are applied. Can be used to fail the type change.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "SetTypes"]
    SetTypes,
    /// Runs when the field's weather is being set.
    ///
    /// Runs before the weather effect is applied. Can be used to fail the weather.
    ///
    /// Runs in the context of an applying effect on the field.
    #[string = "SetWeather"]
    SetWeather,
    /// Runs when a side condition starts successfully.
    ///
    /// Runs in the context of an applying effect on a side.
    #[string = "SideConditionStart"]
    SideConditionStart,
    /// Runs when a side condition ends.
    ///
    /// Runs in the context of the side condition.
    #[string = "SideEnd"]
    SideEnd,
    /// Runs at the end of every turn to apply residual effects on the side.
    ///
    /// Runs in the context of the side condition.
    #[string = "SideResidual"]
    SideResidual,
    /// Runs when a side condition restarts.
    ///
    /// Runs in the context of the side condition.
    #[string = "SideRestart"]
    SideRestart,
    /// Runs when a side condition starts.
    ///
    /// Runs in the context of the side condition.
    #[string = "SideStart"]
    SideStart,
    /// Runs when a slot condition ends.
    ///
    /// Runs in the context of the slot condition.
    #[string = "SlotEnd"]
    SlotEnd,
    /// Runs when a slot condition restarts.
    ///
    /// Runs in the context of the slot condition.
    #[string = "SlotRestart"]
    SlotRestart,
    /// Runs when a slot condition starts.
    ///
    /// Runs in the context of the slot condition.
    #[string = "SlotStart"]
    SlotStart,
    /// Runs when a Mon attempts a stalling move (e.g., Protect).
    ///
    /// Can fail the stalling move (assuming the stalling move integrates with the event properly).
    ///
    /// Runs in the context of a Mon.
    #[string = "StallMove"]
    StallMove,
    /// Runs when an effect starts.
    ///
    /// Used to set up state.
    ///
    /// Runs on the effect.
    #[string = "Start"]
    Start,
    /// Runs when the battle starts.
    ///
    /// Runs in the context of the battle.
    #[string = "StartBattle"]
    StartBattle,
    /// Runs when Mon starts using a move.
    ///
    /// Runs in the context of a Mon.
    #[string = "StartUsingMove"]
    StartUsingMove,
    /// Runs when Mon stops using a move.
    ///
    /// Runs in the context of a Mon.
    #[string = "StopUsingMove"]
    StopUsingMove,
    /// Runs when determining the sub-priority of a move.
    ///
    /// Runs in the context of a move user.
    #[string = "SubPriority"]
    SubPriority,
    /// Runs when determining if terrain on the field is suppressed, for some other active effect.
    ///
    /// Runs on the effect.
    #[string = "SuppressFieldTerrain"]
    SuppressFieldTerrain,
    /// Runs when determining if weather on the field is suppressed, for some other active effect.
    ///
    /// Runs on the effect.
    #[string = "SuppressFieldWeather"]
    SuppressFieldWeather,
    /// Runs when determining if a Mon's ability is suppressed, for some other active effect.
    ///
    /// Runs on the effect.
    #[string = "SuppressMonAbility"]
    SuppressMonAbility,
    /// Runs when determining if the item on the Mon is suppressed, for some other active effect.
    ///
    /// Runs on the effect.
    #[string = "SuppressMonItem"]
    SuppressMonItem,
    /// Runs when determining if terrain on the Mon is suppressed, for some other active effect.
    ///
    /// Runs on the effect.
    #[string = "SuppressMonTerrain"]
    SuppressMonTerrain,
    /// Runs when determining if weather on the Mon is suppressed, for some other active effect.
    ///
    /// Runs on the effect.
    #[string = "SuppressMonWeather"]
    SuppressMonWeather,
    /// Runs when a Mon switches in.
    ///
    /// Runs in the context of a Mon.
    #[string = "SwitchIn"]
    SwitchIn,
    /// Runs when a Mon is switching in, prior to `SwitchIn`.
    ///
    /// Runs in the context of a Mon.
    #[string = "SwitchingIn"]
    SwitchingIn,
    /// Runs when a Mon is switching out.
    ///
    /// Runs in the context of a Mon.
    #[string = "SwitchOut"]
    SwitchOut,
    /// Runs when an item is being taken from a Mon.
    ///
    /// Can prevent the item from being taken.
    ///
    /// Runs on the item and in the context of an applying effect on a Mon.
    #[string = "TakeItem"]
    TakeItem,
    /// Runs when the terrain over a Mon changes.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "TerrainChange"]
    TerrainChange,
    /// Runs when determining if a Mon is trapped (i.e., cannot switch out).
    ///
    /// Runs in the context of a Mon.
    #[string = "TrapMon"]
    TrapMon,
    /// Runs when a group of stat boosts is being applied to a Mon.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "TryBoost"]
    TryBoost,
    /// Runs when a Mon tries to eat its item.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "TryEatItem"]
    TryEatItem,
    /// Runs when trying to end an effect.
    ///
    /// Can prevent the effect from ending.
    ///
    /// Runs on the effect.
    #[string = "TryEnd"]
    TryEnd,
    /// Runs before a Mon is healed for some amount of damage.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "TryHeal"]
    TryHeal,
    /// Runs when a move is trying to hit a set of targets.
    ///
    /// Can fail the move.
    ///
    /// Runs on the active move and in the context of a move target.
    #[string = "TryHit"]
    TryHit,
    /// Runs when a move is trying to hit the whole field.
    ///
    /// Can fail the move.
    ///
    /// Runs on the active move and in the context of an applying effect on the field.
    #[string = "TryHitField"]
    TryHitField,
    /// Runs when a move is trying to hit an entire side
    ///
    /// Can fail the move.
    ///
    /// Runs on the active move and in the context of an applying effect on a side.
    #[string = "TryHitSide"]
    TryHitSide,
    /// Runs when a move is checking general immunity for its target.
    ///
    /// Can fail the move (by marking the target as immune).
    ///
    /// Runs in the context of the active move.
    #[string = "TryImmunity"]
    TryImmunity,
    /// Runs when trying to use a move.
    ///
    /// Can fail the move.
    ///
    /// Runs on the active move and in the context of a move user.
    #[string = "TryMove"]
    TryMove,
    /// Runs when a move's primary hit is being applied to a target.
    ///
    /// Used to override the core battle engine logic. Can fail the move or return an amount of
    /// damage dealt to the target. If zero damage is returned, the core battle engine assumes a
    /// substitute was hit for the purposes of hit effects (i.e., hit effects do not apply to the
    /// target).
    ///
    /// Runs in the context of a move target.
    #[string = "TryPrimaryHit"]
    TryPrimaryHit,
    /// Runs when a Mon tries to use an item.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "TryUseItem"]
    TryUseItem,
    /// Runs when a Mon is trying to use a move on a set of targets.
    ///
    /// Can fail the move.
    ///
    /// Runs on the active move.
    #[string = "TryUseMove"]
    TryUseMove,
    /// Runs when determining if a Mon has immunity against a single type.
    ///
    /// Runs in the context of a Mon.
    #[string = "TypeImmunity"]
    TypeImmunity,
    /// Runs when determining the types of a Mon.
    ///
    /// Runs in the context of a Mon.
    #[string = "Types"]
    Types,
    /// Runs when miscellaneous Mon effects in the battle could activate.
    ///
    /// Runs in the context of a Mon.
    #[string = "Update"]
    Update,
    /// Runs when a Mon uses a move, to upgrade the chosen move.
    ///
    /// Runs in the context of a move user.
    #[string = "UpgradeMove"]
    UpgradeMove,
    /// Runs when an item is used.
    ///
    /// Runs on the item.
    #[string = "Use"]
    Use,
    /// Runs when a Mon uses a move.
    ///
    /// Can be used to modify a move when it is used.
    ///
    /// Runs on the active move and in the context of a move user.
    #[string = "UseMove"]
    UseMove,
    /// Runs when a custom message should be displayed when a Mon uses a move.
    ///
    /// Runs on the active move.
    #[string = "UseMoveMessage"]
    UseMoveMessage,
    /// Runs when validating a Mon.
    ///
    /// Runs in the context of a Mon.
    #[string = "ValidateMon"]
    ValidateMon,
    /// Runs when validating a team.
    ///
    /// Runs in the context of a player.
    #[string = "ValidateTeam"]
    ValidateTeam,
    /// Runs when weather is activated at the end of each turn.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "Weather"]
    Weather,
    /// Runs when the weather over a Mon changes.
    ///
    /// Runs in the context of an applying effect on a Mon.
    #[string = "WeatherChange"]
    WeatherChange,
    /// Runs when calculating the damage applied to a Mon.
    ///
    /// Runs in the context of a move user.
    #[string = "WeatherModifyDamage"]
    WeatherModifyDamage,
}

impl BattleEvent {
    /// Maps the event to the [`CallbackFlag`] flags.
    pub fn callback_type_flags(&self) -> u32 {
        // Maintain alphabetical order.
        match self {
            Self::AccuracyCheckFailed => CommonCallbackType::MoveVoid as u32,
            Self::AccuracyExempt => CommonCallbackType::MoveBoolean as u32,
            Self::Activate => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::ActivateField => CommonCallbackType::FieldEffectVoid as u32,
            Self::ActivatePlayer => CommonCallbackType::PlayerEffectVoid as u32,
            Self::ActivateSide => CommonCallbackType::SideEffectVoid as u32,
            Self::AddPseudoWeather => CommonCallbackType::FieldEffectResult as u32,
            Self::AddType => CommonCallbackType::ApplyingEffectResult as u32,
            Self::AddVolatile => CommonCallbackType::ApplyingEffectResult as u32,
            Self::AfterAddPseudoWeather => CommonCallbackType::FieldEffectVoid as u32,
            Self::AfterAddVolatile => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::AfterBoost => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::AfterCureStatus => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::AfterDamage => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::AfterEachBoost => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::AfterFainted => CommonCallbackType::MonVoid as u32,
            Self::AfterHeal => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::AfterHit => CommonCallbackType::MoveVoid as u32,
            Self::AfterMegaEvolution => CommonCallbackType::MonVoid as u32,
            Self::AfterMove => CommonCallbackType::SourceMoveVoid as u32,
            Self::AfterMoveSecondaryEffects => CommonCallbackType::MoveVoid as u32,
            Self::AfterMoveSecondaryEffectsDamage => CommonCallbackType::MoveVoid as u32,
            Self::AfterMoveSecondaryEffectsUser => CommonCallbackType::SourceMoveVoid as u32,
            Self::AfterSetAbility => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::AfterSetItem => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::AfterSetStatus => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::AfterSubstituteDamage => CommonCallbackType::MoveVoid as u32,
            Self::AfterTakeItem => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::AfterTerastallization => CommonCallbackType::MonVoid as u32,
            Self::AfterUseItem => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::BasePower => CommonCallbackType::MoveModifier as u32,
            Self::BattleEndTurn => CommonCallbackType::NoContextVoid as u32,
            Self::BeforeChargeMove => CommonCallbackType::SourceMoveVoid as u32,
            Self::BeforeDynamax => CommonCallbackType::MonResult as u32,
            Self::BeforeMove => CommonCallbackType::SourceMoveResult as u32,
            Self::BeforeStart => CommonCallbackType::EffectResult as u32,
            Self::BeforeSwitchIn => CommonCallbackType::MonVoid as u32,
            Self::BeforeSwitchOut => CommonCallbackType::MonVoid as u32,
            Self::BeforeTerastallization => CommonCallbackType::MonResult as u32,
            Self::BeforeTurn => CommonCallbackType::SourceMoveVoid as u32,
            Self::BerryEatingHealth => CommonCallbackType::MonModifier as u32,
            Self::CalculateStat => CommonCallbackType::MaybeApplyingEffectModifier as u32,
            Self::CanDynamax => CommonCallbackType::MonBoolean as u32,
            Self::CanEscape => CommonCallbackType::MonBoolean as u32,
            Self::CanHeal => CommonCallbackType::MonBoolean as u32,
            Self::Catch => CommonCallbackType::MonVoid as u32,
            Self::CatchFailed => CommonCallbackType::MonVoid as u32,
            Self::ChangeBoosts => CommonCallbackType::MonBoostModifier as u32,
            Self::ChargeMove => CommonCallbackType::SourceMoveResult as u32,
            Self::ClearTerrain => CommonCallbackType::FieldEffectResult as u32,
            Self::ClearWeather => CommonCallbackType::FieldEffectResult as u32,
            Self::CopyVolatile => CommonCallbackType::ApplyingEffectResult as u32,
            Self::CriticalHit => CommonCallbackType::MoveBoolean as u32,
            Self::CureStatus => CommonCallbackType::ApplyingEffectResult as u32,
            Self::Damage => CommonCallbackType::ApplyingEffectModifier as u32,
            Self::DamagingHit => CommonCallbackType::MoveVoid as u32,
            Self::DisableMove => CommonCallbackType::MonVoid as u32,
            Self::DeductPp => CommonCallbackType::SourceMoveModifier as u32,
            Self::DragOut => CommonCallbackType::MonResult as u32,
            Self::Duration => CommonCallbackType::ApplyingEffectModifier as u32,
            Self::Effectiveness => CommonCallbackType::ApplyingEffectModifier as u32,
            Self::Eat => CommonCallbackType::MonVoid as u32,
            Self::EatItem => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::End => CommonCallbackType::EffectVoid as u32,
            Self::EndBattle => CommonCallbackType::MonVoid as u32,
            Self::EndTurn => CommonCallbackType::MonVoid as u32,
            Self::Exit => CommonCallbackType::MonVoid as u32,
            Self::Faint => CommonCallbackType::MaybeApplyingEffectVoid as u32,
            Self::FieldEnd => CommonCallbackType::FieldVoid as u32,
            Self::FieldResidual => CommonCallbackType::FieldVoid as u32,
            Self::FieldRestart => CommonCallbackType::FieldResult as u32,
            Self::FieldStart => CommonCallbackType::FieldResult as u32,
            Self::Flinch => CommonCallbackType::MonVoid as u32,
            Self::ForceEffectiveness => CommonCallbackType::ApplyingEffectModifier as u32,
            Self::ForceEscape => CommonCallbackType::MonBoolean as u32,
            Self::ForceTeraType => CommonCallbackType::MonType as u32,
            Self::ForceTypes => CommonCallbackType::MonTypes as u32,
            Self::Hit => CommonCallbackType::MoveResult as u32,
            Self::HitField => CommonCallbackType::MoveFieldResult as u32,
            Self::HitSide => CommonCallbackType::MoveSideResult as u32,
            Self::HitUser => CommonCallbackType::MoveResult as u32,
            Self::IgnoreImmunity => CommonCallbackType::MoveBoolean as u32,
            Self::Immunity => CommonCallbackType::ApplyingEffectBoolean as u32,
            Self::Invulnerability => CommonCallbackType::MoveBoolean as u32,
            Self::IsAsleep => CommonCallbackType::MonBoolean as u32,
            Self::IsAwayFromField => CommonCallbackType::MonBoolean as u32,
            Self::IsBehindSubstitute => CommonCallbackType::MonBoolean as u32,
            Self::IsChoiceLocked => CommonCallbackType::MonBoolean as u32,
            Self::IsContactProof => CommonCallbackType::MonBoolean as u32,
            Self::IsGrounded => CommonCallbackType::MonBoolean as u32,
            Self::IsImmuneToEntryHazards => CommonCallbackType::MonBoolean as u32,
            Self::IsRaining => CommonCallbackType::NoContextBoolean as u32,
            Self::IsSemiInvulnerable => CommonCallbackType::MonBoolean as u32,
            Self::IsSnowing => CommonCallbackType::NoContextBoolean as u32,
            Self::IsSoundproof => CommonCallbackType::MonBoolean as u32,
            Self::IsSunny => CommonCallbackType::NoContextBoolean as u32,
            Self::LockMove => CommonCallbackType::MonInfo as u32,
            Self::ModifyAccuracy => CommonCallbackType::MoveModifier as u32,
            Self::ModifyActionSpeed => CommonCallbackType::MonModifier as u32,
            Self::ModifyAtk => CommonCallbackType::MaybeApplyingEffectModifier as u32,
            Self::ModifyBoosts => CommonCallbackType::MaybeApplyingEffectBoostModifier as u32,
            Self::ModifyCatchRate => CommonCallbackType::ApplyingEffectModifier as u32,
            Self::ModifyCritChance => CommonCallbackType::SourceMoveModifier as u32,
            Self::ModifyCritRatio => CommonCallbackType::SourceMoveModifier as u32,
            Self::ModifyDamage => CommonCallbackType::SourceMoveModifier as u32,
            Self::ModifyDef => CommonCallbackType::MaybeApplyingEffectModifier as u32,
            Self::ModifyDuration => CommonCallbackType::ApplyingEffectModifier as u32,
            Self::ModifyEffectiveness => CommonCallbackType::ApplyingEffectModifier as u32,
            Self::ModifyEvYield => CommonCallbackType::MonStatTableModifier as u32,
            Self::ModifyExperience => CommonCallbackType::MonModifier as u32,
            Self::ModifyFieldDuration => CommonCallbackType::FieldEffectModifier as u32,
            Self::ModifyFriendshipIncrease => CommonCallbackType::MonModifier as u32,
            Self::ModifyMoveType => CommonCallbackType::SourceEffectType as u32,
            Self::ModifyPriority => CommonCallbackType::SourceMoveModifier as u32,
            Self::ModifySecondaryEffects => CommonCallbackType::MoveSecondaryEffectModifier as u32,
            Self::ModifySideDuration => CommonCallbackType::SideEffectModifier as u32,
            Self::ModifySlotDuration => CommonCallbackType::SideEffectModifier as u32,
            Self::ModifySpA => CommonCallbackType::MaybeApplyingEffectModifier as u32,
            Self::ModifySpD => CommonCallbackType::MaybeApplyingEffectModifier as u32,
            Self::ModifySpe => CommonCallbackType::MaybeApplyingEffectModifier as u32,
            Self::ModifySpeciesCatchRate => CommonCallbackType::ApplyingEffectModifier as u32,
            Self::ModifyStab => CommonCallbackType::SourceMoveModifier as u32,
            Self::ModifyTarget => CommonCallbackType::SourceMoveMonModifier as u32,
            Self::ModifyWeight => CommonCallbackType::MonModifier as u32,
            Self::MoveAborted => CommonCallbackType::SourceMoveVoid as u32,
            Self::MoveBasePower => CommonCallbackType::MoveModifier as u32,
            Self::MoveDamage => CommonCallbackType::MoveModifier as u32,
            Self::MoveFailed => CommonCallbackType::SourceMoveVoid as u32,
            Self::MoveTargetOverride => CommonCallbackType::MonMoveTarget as u32,
            Self::NegateImmunity => CommonCallbackType::MonBoolean as u32,
            Self::OverrideWeather => CommonCallbackType::MonInfo as u32,
            Self::OverwriteMove => CommonCallbackType::MonVoid as u32,
            Self::OverrideMove => CommonCallbackType::MonInfo as u32,
            Self::PlayerTryUseItem => CommonCallbackType::EffectBoolean as u32,
            Self::PlayerUse => CommonCallbackType::MonVoid as u32,
            Self::PreMoveEffect => CommonCallbackType::SourceMoveVoid as u32,
            Self::PrepareHit => CommonCallbackType::SourceMoveResult as u32,
            Self::PreventUsedItems => CommonCallbackType::MonBoolean as u32,
            Self::PriorityChargeMove => CommonCallbackType::SourceMoveVoid as u32,
            Self::RedirectTarget => CommonCallbackType::SourceMoveMonModifier as u32,
            Self::Residual => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::Restart => CommonCallbackType::EffectResult as u32,
            Self::RestorePp => CommonCallbackType::ApplyingEffectModifier as u32,
            Self::Select => CommonCallbackType::MonVoid as u32,
            Self::SetAbility => CommonCallbackType::ApplyingEffectResult as u32,
            Self::SetItem => CommonCallbackType::ApplyingEffectResult as u32,
            Self::SetLastMove => CommonCallbackType::MonBoolean as u32,
            Self::SetStatus => CommonCallbackType::ApplyingEffectResult as u32,
            Self::SetTerrain => CommonCallbackType::FieldEffectResult as u32,
            Self::SetTypes => CommonCallbackType::ApplyingEffectResult as u32,
            Self::SetWeather => CommonCallbackType::FieldEffectResult as u32,
            Self::SideConditionStart => CommonCallbackType::SideVoid as u32,
            Self::SideEnd => CommonCallbackType::SideVoid as u32,
            Self::SideResidual => CommonCallbackType::SideVoid as u32,
            Self::SideRestart => CommonCallbackType::SideResult as u32,
            Self::SideStart => CommonCallbackType::SideResult as u32,
            Self::SlotEnd => CommonCallbackType::SideResult as u32,
            Self::SlotRestart => CommonCallbackType::SideResult as u32,
            Self::SlotStart => CommonCallbackType::SideResult as u32,
            Self::StallMove => CommonCallbackType::MonBoolean as u32,
            Self::Start => CommonCallbackType::EffectResult as u32,
            Self::StartBattle => CommonCallbackType::NoContextVoid as u32,
            Self::StartUsingMove => CommonCallbackType::MonVoid as u32,
            Self::StopUsingMove => CommonCallbackType::MonVoid as u32,
            Self::SubPriority => CommonCallbackType::SourceMoveModifier as u32,
            Self::SuppressFieldTerrain => CommonCallbackType::NoContextBoolean as u32,
            Self::SuppressFieldWeather => CommonCallbackType::NoContextBoolean as u32,
            Self::SuppressMonAbility => CommonCallbackType::MonBoolean as u32,
            Self::SuppressMonItem => CommonCallbackType::MonBoolean as u32,
            Self::SuppressMonTerrain => CommonCallbackType::MonBoolean as u32,
            Self::SuppressMonWeather => CommonCallbackType::MonBoolean as u32,
            Self::SwitchIn => CommonCallbackType::MonVoid as u32,
            Self::SwitchingIn => CommonCallbackType::MonVoid as u32,
            Self::SwitchOut => CommonCallbackType::MonVoid as u32,
            Self::TakeItem => CommonCallbackType::ApplyingEffectResult as u32,
            Self::TerrainChange => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::TrapMon => CommonCallbackType::MonBoolean as u32,
            Self::TryBoost => CommonCallbackType::ApplyingEffectBoostModifier as u32,
            Self::TryEatItem => CommonCallbackType::ApplyingEffectResult as u32,
            Self::TryEnd => CommonCallbackType::EffectResult as u32,
            Self::TryHeal => CommonCallbackType::ApplyingEffectModifier as u32,
            Self::TryHit => CommonCallbackType::MoveResult as u32,
            Self::TryHitField => CommonCallbackType::MoveFieldResult as u32,
            Self::TryHitSide => CommonCallbackType::MoveSideResult as u32,
            Self::TryImmunity => CommonCallbackType::MoveBoolean as u32,
            Self::TryMove => CommonCallbackType::SourceMoveResult as u32,
            Self::TryPrimaryHit => CommonCallbackType::MoveHitOutcomeResult as u32,
            Self::TryUseItem => CommonCallbackType::ApplyingEffectResult as u32,
            Self::TryUseMove => CommonCallbackType::SourceMoveResult as u32,
            Self::TypeImmunity => CommonCallbackType::MonBoolean as u32,
            Self::Types => CommonCallbackType::MonTypes as u32,
            Self::Update => CommonCallbackType::MonVoid as u32,
            Self::UpgradeMove => CommonCallbackType::SourceMoveActiveMove as u32,
            Self::Use => CommonCallbackType::MonVoid as u32,
            Self::UseMove => CommonCallbackType::SourceMoveVoid as u32,
            Self::UseMoveMessage => CommonCallbackType::SourceMoveVoid as u32,
            Self::ValidateMon => CommonCallbackType::MonValidator as u32,
            Self::ValidateTeam => CommonCallbackType::PlayerValidator as u32,
            Self::Weather => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::WeatherChange => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::WeatherModifyDamage => CommonCallbackType::SourceMoveModifier as u32,
        }
    }

    /// Checks if the event has the given [`CallbackFlag`] flag set.
    pub fn has_flag(&self, flag: u32) -> bool {
        self.callback_type_flags() & flag != 0
    }

    /// The target of the event callback is the "origin" of the event.
    ///
    /// Most events target a Mon. Some event callbacks receive a Mon as the "source" of the effect.
    /// However, some events trigger against against the source Mon. This is most common for move
    /// events that run "in the context of a move user." In this sense, the target of the event
    /// callback is actually the source.
    ///
    /// This method allows event callbacks to understand who the true "origin" is, no matter who the
    /// target or source is. This is important for effects that need to understand why some event is
    /// running. For example, the ability "Mega Sol" needs to override weather event callbacks
    /// whenever the ability holder is the origin, which can sometimes be the source (e.g.,
    /// ModifySpd) or the target (e.g., WeatherModifyDamage).
    pub fn target_is_event_origin(&self) -> bool {
        self.has_flag(CallbackFlag::TakesGeneralMon) || self.has_flag(CallbackFlag::TakesUserMon)
    }

    /// Does the event allow custom input variables?
    pub fn allows_custom_input_vars(&self) -> bool {
        // Maintain alphabetical order.
        match self {
            Self::Activate => true,
            _ => false,
        }
    }

    /// The name of the input variable by index.
    pub fn input_vars(&self) -> &[(&str, ValueType, bool)] {
        // Maintain alphabetical order.
        match self {
            Self::AddPseudoWeather | Self::AfterAddPseudoWeather => {
                &[("pseudo_weather", ValueType::Effect, true)]
            }
            Self::AddType => &[("type", ValueType::Type, true)],
            Self::AddVolatile | Self::AfterAddVolatile => &[("volatile", ValueType::Effect, true)],
            Self::AfterBoost => &[("boosts", ValueType::BoostTable, true)],
            Self::AfterDamage => &[("damage", ValueType::UFraction, true)],
            Self::AfterEachBoost => &[
                ("boost", ValueType::Boost, true),
                ("value", ValueType::Fraction, true),
            ],
            Self::AfterFainted => &[
                ("count", ValueType::UFraction, true),
                ("effect", ValueType::Effect, false),
            ],
            Self::AfterHeal => &[("damage", ValueType::UFraction, true)],
            Self::AfterMove => &[("success", ValueType::Boolean, true)],
            Self::AfterMoveSecondaryEffectsDamage => &[
                ("damage", ValueType::UFraction, true),
                ("original_hp", ValueType::UFraction, true),
            ],
            Self::AfterMoveSecondaryEffectsUser => &[("targets", ValueType::List, true)],
            Self::AfterSetItem | Self::AfterTakeItem | Self::AfterUseItem => {
                &[("item", ValueType::Effect, true)]
            }
            Self::BasePower => &[("base_power", ValueType::UFraction, true)],
            Self::BerryEatingHealth => &[("hp", ValueType::UFraction, true)],
            Self::CalculateStat => &[
                ("stat", ValueType::UFraction, true),
                ("name", ValueType::Stat, true),
            ],
            Self::CatchFailed => &[("item", ValueType::Effect, true)],
            Self::ChangeBoosts => &[("boosts", ValueType::BoostTable, true)],
            Self::Damage => &[("damage", ValueType::UFraction, true)],
            Self::DamagingHit => &[("damage", ValueType::UFraction, true)],
            Self::DeductPp => &[("pp", ValueType::UFraction, true)],
            Self::EatItem => &[("item", ValueType::Effect, true)],
            Self::Effectiveness => &[
                ("modifier", ValueType::Fraction, true),
                ("type", ValueType::Type, true),
                ("index", ValueType::UFraction, true),
            ],
            Self::ForceEffectiveness => &[("modifier", ValueType::Fraction, true)],
            Self::ForceTeraType => &[("type", ValueType::Type, true)],
            Self::ModifyAccuracy => &[("acc", ValueType::UFraction, true)],
            Self::ModifyActionSpeed => &[("spe", ValueType::UFraction, true)],
            Self::ModifyAtk => &[("atk", ValueType::UFraction, true)],
            Self::ModifyBoosts => &[("boosts", ValueType::BoostTable, true)],
            Self::ModifyCatchRate | Self::ModifySpeciesCatchRate => {
                &[("catch_rate", ValueType::UFraction, true)]
            }
            Self::ModifyCritChance => &[("chance", ValueType::UFraction, true)],
            Self::ModifyCritRatio => &[("crit_ratio", ValueType::UFraction, true)],
            Self::ModifyDamage | Self::WeatherModifyDamage => {
                &[("damage", ValueType::UFraction, true)]
            }
            Self::ModifyDef => &[("def", ValueType::UFraction, true)],
            Self::ModifyDuration | Self::ModifySideDuration | Self::ModifyFieldDuration => &[
                ("duration", ValueType::UFraction, true),
                ("condition", ValueType::Effect, true),
            ],
            Self::ModifyEffectiveness => &[("modifier", ValueType::Fraction, true)],
            Self::ModifyEvYield => &[("evs", ValueType::StatTable, true)],
            Self::ModifyExperience => &[("exp", ValueType::UFraction, true)],
            Self::ModifyFriendshipIncrease => &[("friendship", ValueType::UFraction, true)],
            Self::ModifyMoveType => &[("type", ValueType::Type, true)],
            Self::ModifyPriority => &[("priority", ValueType::Fraction, true)],
            Self::ModifySecondaryEffects => &[("secondary_effects", ValueType::List, true)],
            Self::ModifySlotDuration => &[
                ("duration", ValueType::UFraction, true),
                ("slot", ValueType::UFraction, true),
                ("condition", ValueType::Effect, true),
            ],
            Self::ModifySpA => &[("spa", ValueType::UFraction, true)],
            Self::ModifySpD => &[("spd", ValueType::UFraction, true)],
            Self::ModifySpe => &[("spe", ValueType::UFraction, true)],
            Self::ModifyStab => &[("stab", ValueType::UFraction, true)],
            Self::ModifyTarget => &[("target", ValueType::Mon, false)],
            Self::ModifyWeight => &[("weight", ValueType::UFraction, true)],
            Self::NegateImmunity => &[("type", ValueType::Type, true)],
            Self::OverrideMove => &[("move", ValueType::String, true)],
            Self::PlayerTryUseItem => &[("input", ValueType::Object, true)],
            Self::PlayerUse => &[("input", ValueType::Object, true)],
            Self::RedirectTarget => &[("target", ValueType::Mon, true)],
            Self::RestorePp => &[("pp", ValueType::UFraction, true)],
            Self::Select => &[("selected", ValueType::Mon, true)],
            Self::SetAbility | Self::AfterSetAbility => &[("ability", ValueType::Effect, true)],
            Self::SetItem => &[("item", ValueType::Effect, true)],
            Self::SetStatus | Self::AfterSetStatus => &[("status", ValueType::Effect, true)],
            Self::SetTerrain => &[("terrain", ValueType::Effect, true)],
            Self::SetTypes => &[("types", ValueType::List, true)],
            Self::SetWeather => &[("weather", ValueType::Effect, true)],
            Self::SideConditionStart => &[("condition", ValueType::Effect, true)],
            Self::SlotEnd => &[("slot", ValueType::UFraction, true)],
            Self::SlotRestart => &[("slot", ValueType::UFraction, true)],
            Self::SlotStart => &[("slot", ValueType::UFraction, true)],
            Self::SubPriority => &[("sub_priority", ValueType::Fraction, true)],
            Self::TakeItem => &[("item", ValueType::Effect, true)],
            Self::TryBoost => &[("boosts", ValueType::BoostTable, true)],
            Self::TryEatItem => &[("item", ValueType::Effect, true)],
            Self::TryHit => &[("report", ValueType::Boolean, false)],
            Self::TryUseItem => &[("item", ValueType::Effect, true)],
            Self::TryHeal => &[("damage", ValueType::UFraction, true)],
            Self::TypeImmunity => &[("type", ValueType::Type, true)],
            Self::Types | Self::ForceTypes => &[("types", ValueType::List, true)],
            Self::ValidateMon | Self::ValidateTeam => &[("problems", ValueType::List, true)],
            _ => &[],
        }
    }

    /// Checks if the given output type is allowed.
    pub fn output_type_allowed(&self, value_type: Option<ValueType>) -> bool {
        match value_type {
            Some(value_type) if value_type.is_number() => {
                self.has_flag(CallbackFlag::ReturnsNumber)
            }
            Some(ValueType::Boolean) => {
                self.has_flag(CallbackFlag::ReturnsBoolean | CallbackFlag::ReturnsEventResult)
            }
            Some(ValueType::String) => self.has_flag(
                CallbackFlag::ReturnsString
                    | CallbackFlag::ReturnsEventResult
                    | CallbackFlag::ReturnsMoveTarget,
            ),
            Some(ValueType::EventResult) => self.has_flag(CallbackFlag::ReturnsEventResult),
            Some(ValueType::Mon) => self.has_flag(CallbackFlag::ReturnsMon),
            Some(ValueType::ActiveMove) => self.has_flag(CallbackFlag::ReturnsActiveMove),
            Some(ValueType::BoostTable) => self.has_flag(CallbackFlag::ReturnsBoosts),
            Some(ValueType::MoveTarget) => self.has_flag(CallbackFlag::ReturnsMoveTarget),
            Some(ValueType::StatTable) => self.has_flag(CallbackFlag::ReturnsStatTable),
            Some(ValueType::Type) => self.has_flag(CallbackFlag::ReturnsType),
            Some(ValueType::List) => self.has_flag(
                CallbackFlag::ReturnsTypes
                    | CallbackFlag::ReturnsSecondaryEffects
                    | CallbackFlag::ReturnsStrings,
            ),
            Some(ValueType::Undefined) => self.has_flag(CallbackFlag::ReturnsVoid),
            None => self.has_flag(CallbackFlag::ReturnsVoid),
            _ => false,
        }
    }

    /// The layer that the event is used for callback lookup.
    ///
    /// Some events can be used during callback lookup, which can cause unnecessary and even
    /// infinite recursion. To combat this, we give events used for callback lookup a layer number.
    /// When looking up the callbacks for event A, do not run callback lookup event B if `A.layer <=
    /// B.layer` (if A is below or at the same layer as B).
    ///
    /// For example, `Types` is in layer 0 and `SuppressFieldWeather` is in layer 1. Since
    /// `SuppressFieldWeather` is used for determining a Mon's effective weather, the Mon's
    /// effective weather should not be used as a callback for the `Types` event. In other words,
    /// the weather on the field cannot impact a Mon's types directly.
    ///
    /// This creates some limitations that must be carefully considered. These are very niche edge
    /// cases (such as the one described above), and there is almost always a workaround (in the
    /// above case, weather can apply a volatile effect to Mons for the duration of the weather
    /// that changes each Mon's type).
    ///
    /// An example of infinite recursion:
    /// - The battle engine runs the `Immunity` event for some Mon.
    /// - The Mon's types are included in the set of effects that could have a callback for this
    ///   event.
    /// - To determine the Mon's types, the battle engine runs the `Types` event.
    /// - The Mon's types are included in the set of effects that could have a callback for this
    ///   event.
    /// - The `Types` event leads to infinite recursion.
    ///
    /// An example of unnecessary recursion:
    /// - The battle engine runs the `Immunity` event for some Mon.
    /// - The Mon's types are included in the set of effects that could have a callback for this
    ///   event.
    /// - To determine the Mon's types, the callback lookup code runs the `Types` event.
    /// - The Mon's effective weather is included in the set of effects that could have a callback
    ///   for this event.
    /// - To determine the Mon's effective weather, the battle engine runs the `SuppressMonWeather`
    ///   event.
    /// - If the weather is not suppressed, the effective weather is based on the field's effective
    ///   weather.
    /// - To determine the field's effective weather, the battle engine runs the
    ///   `SuppressFieldWeather` event.
    /// - After those two events run, the effective weather for the `Types` event has been
    ///   determined.
    /// - All callbacks run to determine the Mon's types.
    /// - Then, the `SuppressMonWeather` and `SuppressFieldWeather` events are run *again* for the
    ///   `Immunity` event.
    /// - The weather events are run twice. If weather does not ever impact the Mon's types, we do
    ///   not need to run the weather events in the `Types` event.
    pub fn callback_lookup_layer(&self) -> usize {
        match self {
            Self::SuppressMonAbility => 0,
            Self::SuppressMonItem => 1,
            Self::ForceTypes => 2,
            Self::Types => 2,
            Self::IsGrounded => 3,
            Self::IsSemiInvulnerable => 3,
            Self::SuppressFieldTerrain => 4,
            Self::SuppressFieldWeather => 4,
            Self::SuppressMonTerrain => 5,
            Self::SuppressMonWeather => 5,
            Self::OverrideWeather => 5,
            _ => usize::MAX,
        }
    }

    /// Whether or not to run the event callback on the source effect when running all callbacks for
    /// an event.
    pub fn run_callback_on_source_effect(&self) -> bool {
        match self {
            Self::BasePower => true,
            Self::Damage => true,
            Self::ModifyCatchRate => true,
            Self::ModifySpeciesCatchRate => true,
            Self::ModifyTarget => true,
            Self::WeatherModifyDamage => true,
            _ => false,
        }
    }

    /// Whether or not to force effects to have a default callback for the event.
    ///
    /// This is used for residual events that are suppressed. We keep the callback so that durations
    /// are updated without running the actual callback.
    pub fn force_default_callback(&self) -> bool {
        match self {
            Self::FieldStart | Self::SideStart | Self::SlotStart | Self::Start => true,
            Self::FieldResidual | Self::SideResidual | Self::Residual => true,
            _ => false,
        }
    }

    /// Whether or not to exclude effects that are not started.
    ///
    /// See [`EffectState::started`][`crate::effect::fxlang::EffectState::started`]. Ordinarily,
    /// event callbacks are still run against un-started effects. However, this may be undesirable
    /// for specific events that run very frequently. This option may be used for overriding this
    /// behavior.
    ///
    /// For example, the [`Update`][`Self::Update`] event runs after every action. However,
    /// switch-ins and switch-in events are split across two separate actions. The `Update`
    /// event after the switch-in may trigger a Mon to use its held item (e.g., eat a berry when
    /// it switches in at low HP) immediately.
    ///
    /// However, the item should only be consumed once it "starts" as part of the switch-in events
    /// action. This option forces the `Update` event callback to wait until the item is officially
    /// started, which satisfies our ordering requirements. Events such as entry hazards occur
    /// *before* the item starts and is consumed on the subsequent `Update` event.
    pub fn exclude_unstarted_effects(&self) -> bool {
        match self {
            Self::Update => true,
            _ => false,
        }
    }

    /// Whether or not to use the effect's order (on its
    /// [`EffectState`][`crate::effect::fxlang::EffectState`]) when ordering callbacks.
    pub fn order_using_effect_order(&self) -> bool {
        match self {
            Self::Residual | Self::SwitchIn => true,
            _ => false,
        }
    }

    /// Whether or not the event represents state rather than an active event.
    pub fn state_event(&self) -> bool {
        self.to_string().starts_with("Is")
    }

    /// Whether or not the event is intended to start the associated effect.
    pub fn starts_effect(&self) -> bool {
        match self {
            Self::FieldStart | Self::Start | Self::SideStart | Self::SlotStart => true,
            _ => false,
        }
    }

    /// Whether or not the event is intended to end the associated effect.
    pub fn ends_effect(&self) -> bool {
        match self {
            Self::FieldEnd | Self::End | Self::SideEnd | Self::SlotEnd => true,
            _ => false,
        }
    }

    /// The associated event on the field.
    ///
    /// Only used for events that are distinct when using modifiers.
    pub fn field_event(&self) -> Option<BattleEvent> {
        match self {
            Self::Residual => Some(Self::FieldResidual),
            _ => None,
        }
    }

    /// The associated event on the field.
    ///
    /// Only used for events that are distinct when using modifiers.
    pub fn side_event(&self) -> Option<BattleEvent> {
        match self {
            Self::Residual => Some(Self::SideResidual),
            _ => None,
        }
    }
}

/// An fxlang program, which describes an individual callback for an effect to be interpreted and
/// applied in battle.
///
/// Internally represented as a tree-like structure for interpretation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Program {
    /// A single statement.
    Leaf(String),
    /// A group of statements that should be executed together.
    ///
    /// A branch can be conditionally or repeatedly executed by the preceding statement.
    Branch(Vec<Program>),
}

/// Metadata for an fxlang program.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramMetadata {
    /// Custom parameters, assuming the event supports it.
    pub parameters: Vec<String>,
}

/// An fxlang program with priority information for ordering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramWithPriority {
    pub program: Option<Program>,
    pub order: Option<u32>,
    pub priority: Option<i32>,
    pub sub_order: Option<u32>,
    pub metadata: Option<ProgramMetadata>,
}

/// The input to the [`Callback`] type.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CallbackInput {
    Regular(Program),
    WithPriority(ProgramWithPriority),
}

/// A single callback, to be called when applying an effect on some triggered event.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Callback(Option<CallbackInput>);

impl Callback {
    /// Checks if the callback has an associated [`Program`].
    pub fn has_program(&self) -> bool {
        match &self.0 {
            Some(CallbackInput::Regular(_)) => true,
            Some(CallbackInput::WithPriority(program)) => program.program.is_some(),
            None => false,
        }
    }

    /// Returns a reference to the callback's [`Program`].
    pub fn program(&self) -> Option<&Program> {
        match self.0.as_ref()? {
            CallbackInput::Regular(program) => Some(&program),
            CallbackInput::WithPriority(program) => program.program.as_ref(),
        }
    }

    pub fn metadata(&self) -> Option<&ProgramMetadata> {
        match self.0.as_ref()? {
            CallbackInput::Regular(_) => None,
            CallbackInput::WithPriority(program) => program.metadata.as_ref(),
        }
    }
}

impl SpeedOrderable for Callback {
    fn order(&self) -> u32 {
        match &self.0 {
            Some(CallbackInput::WithPriority(program)) => program.order.unwrap_or(u32::MAX),
            _ => u32::MAX,
        }
    }

    fn priority(&self) -> i32 {
        match &self.0 {
            Some(CallbackInput::WithPriority(program)) => program.priority.unwrap_or(0),
            _ => 0,
        }
    }

    fn sub_priority(&self) -> i32 {
        0
    }

    fn speed(&self) -> u32 {
        0
    }

    fn sub_order(&self) -> u32 {
        match &self.0 {
            Some(CallbackInput::WithPriority(program)) => program.sub_order.unwrap_or(0),
            _ => 0,
        }
    }
}

/// A collection of callbacks for an effect.
pub type Callbacks = HashMap<String, Callback>;

/// Attributes for an [`Effect`] that are meaningful when attaching to some part of a battle.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ConditionAttributes {
    /// The static duration of the effect.
    ///
    /// Can be overwritten by the [`Duration`][`BattleEvent::Duration`] callback.
    pub duration: Option<u8>,

    /// Whether or not the effect can be copied to another Mon.
    ///
    /// If true, moves like "Baton Pass" will not copy this effect. `false` by default.
    #[serde(default)]
    pub no_copy: bool,

    /// Whether or not the effect can be copied when a Mon transforms.
    #[serde(default)]
    pub copy_on_transform: bool,
}

impl ConditionAttributes {
    /// Extends the condition attributes with some other attribute object, overriding data if
    /// applicable.
    pub fn extend(&mut self, other: Self) {
        if let Some(duration) = other.duration {
            self.duration = Some(duration);
        }
        self.no_copy = other.no_copy || self.no_copy;
        self.copy_on_transform = other.copy_on_transform || self.copy_on_transform;
    }
}

/// Attributes for an [`Effect`].
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct EffectAttributes {
    /// Effects to delegate to.
    ///
    /// Format is an effect's fxlang ID: `${type}:${id}`.
    ///
    /// Callbacks from delegate effects are imported. Any callback on this effect overwrites
    /// imported callbacks.
    #[serde(default)]
    pub delegates: Vec<String>,

    /// Attributes for an effect that attaches to some part of a battle.
    #[serde(flatten)]
    pub condition: ConditionAttributes,
}

/// An effect, whose callbacks are triggered in the context of an ongoing battle.
///
/// When an effect is active, its event callbacks are triggered throughout the course of a battle.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Effect {
    /// Event callbacks for the effect.
    #[serde(default)]
    pub callbacks: Callbacks,

    /// Local data for the effects.
    #[serde(default)]
    pub local_data: LocalData,

    /// Effect attributes.
    #[serde(flatten)]
    pub attributes: EffectAttributes,
}

impl TryFrom<serde_json::Value> for Effect {
    type Error = Error;
    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value).wrap_error_with_message("invalid fxlang effect")
    }
}
