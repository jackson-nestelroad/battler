use ahash::HashMap;
use anyhow::Error;
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

    pub const ReturnsMoveTarget: u32 = 1 << 21;
    pub const ReturnsStrings: u32 = 1 << 22;
    pub const ReturnsSecondaryEffects: u32 = 1 << 23;
    pub const ReturnsTypes: u32 = 1 << 24;
    pub const ReturnsMon: u32 = 1 << 25;
    pub const ReturnsBoosts: u32 = 1 << 26;
    pub const ReturnsString: u32 = 1 << 27;
    pub const ReturnsMoveResult: u32 = 1 << 28;
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
    ApplyingEffectResult = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesEffect
        | CallbackFlag::ReturnsBoolean
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

    EffectResult = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesSourceEffect
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsVoid,
    EffectVoid = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesSourceEffect
        | CallbackFlag::ReturnsVoid,

    NoContextResult = CallbackFlag::ReturnsBoolean | CallbackFlag::ReturnsVoid,

    SourceMoveModifier = CallbackFlag::TakesUserMon
        | CallbackFlag::TakesSourceTargetMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsNumber
        | CallbackFlag::ReturnsVoid,
    SourceMoveResult = CallbackFlag::TakesUserMon
        | CallbackFlag::TakesSourceTargetMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsVoid,
    SourceMoveVoid = CallbackFlag::TakesUserMon
        | CallbackFlag::TakesSourceTargetMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsVoid,
    SourceMoveControllingResult = CallbackFlag::TakesUserMon
        | CallbackFlag::TakesSourceTargetMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsMoveResult
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsVoid,
    SourceMoveMonModifier = CallbackFlag::TakesUserMon
        | CallbackFlag::TakesSourceTargetMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsMon
        | CallbackFlag::ReturnsVoid,

    MoveModifier = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsNumber
        | CallbackFlag::ReturnsVoid,
    MoveResult = CallbackFlag::TakesTargetMon
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
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsVoid,
    MoveControllingResult = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsMoveResult
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsVoid,
    MoveSecondaryEffectModifier = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsSecondaryEffects
        | CallbackFlag::ReturnsVoid,

    MonModifier =
        CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsNumber | CallbackFlag::ReturnsVoid,
    MonResult =
        CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsBoolean | CallbackFlag::ReturnsVoid,
    MonVoid = CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsVoid,
    MonInfo =
        CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsString | CallbackFlag::ReturnsVoid,
    MonTypes = CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsTypes,
    MonBoostModifier =
        CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsBoosts | CallbackFlag::ReturnsVoid,
    MonValidator = CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsStrings,
    MonMoveTarget =
        CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsMoveTarget | CallbackFlag::ReturnsVoid,

    PlayerValidator = CallbackFlag::TakesPlayer | CallbackFlag::ReturnsStrings,

    SideVoid = CallbackFlag::TakesSide
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesSourceEffect
        | CallbackFlag::ReturnsVoid,
    SideResult = CallbackFlag::TakesSide
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesSourceEffect
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsVoid,

    MoveSideResult = CallbackFlag::TakesSide
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsVoid,
    MoveSideControllingResult = CallbackFlag::TakesSide
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsMoveResult
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsVoid,

    MoveFieldResult = CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsMoveResult
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsVoid,
    MoveFieldControllingResult = CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsVoid,

    FieldVoid =
        CallbackFlag::TakesSourceMon | CallbackFlag::TakesSourceEffect | CallbackFlag::ReturnsVoid,
    FieldResult = CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesSourceEffect
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsVoid,

    FieldEffectResult = CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesEffect
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsVoid,
    FieldEffectVoid =
        CallbackFlag::TakesSourceMon | CallbackFlag::TakesEffect | CallbackFlag::ReturnsVoid,
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
    /// Runs when the accuracy of a move against a target is being determined.
    ///
    /// Runs in the context of a move target.
    #[string = "AccuracyExempt"]
    AccuracyExempt,
    /// Runs when a pseudo-weather is being added to the field.
    ///
    /// Runs before the pseudo-weather effect is applied. Can be used to fail the pseudo-weather.
    ///
    /// Runs in the context of an applying effect on the field.
    #[string = "AddPseudoWeather"]
    AddPseudoWeather,
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
    /// Runs after a Mon hits another Mon with a move.
    ///
    /// Runs on the active move.
    #[string = "AfterHit"]
    AfterHit,
    /// Runs after a Mon finishes using a move.
    ///
    /// Runs on the active move and in the context of a move user.
    #[string = "AfterMove"]
    AfterMove,
    /// Runs after a move's secondary effects have been applied.
    ///
    /// Should be viewed as the last effect the move needs to apply.
    ///
    /// Runs on the active move and in the context of a move target.
    #[string = "AfterMoveSecondaryEffects"]
    AfterMoveSecondaryEffects,
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
    /// Runs before a Mon uses a move.
    ///
    /// Can prevent the move from being used.
    ///
    /// Runs on the active move and in the context of a move user.
    #[string = "BeforeMove"]
    BeforeMove,
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
    /// Runs before a turn of a battle.
    ///
    /// Runs in the context of a Mon.
    #[string = "BeforeTurn"]
    BeforeTurn,
    /// Runs when determining the health at which the Mon should eat berries.
    ///
    /// Runs in the context of a Mon.
    #[string = "BerryEatingHealth"]
    BerryEatingHealth,
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
    /// Runs when a group of stat boosts is being applied to a Mon.
    ///
    /// Runs in the context of a Mon.
    #[string = "ChangeBoosts"]
    ChangeBoosts,
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
    /// Runs in the context a Mon.
    #[string = "DeductPp"]
    DeductPp,
    /// Runs when determining which moves are disabled.
    ///
    /// Runs in the context a Mon.
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
    /// Runs on the active move and in the context of a move target.
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
    /// Runs when a Mon is affected by an entry hazard.
    ///
    /// Runs in the context of a Mon.
    #[string = "EntryHazard"]
    EntryHazard,
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
    /// Runs when a Mon is attempting to escape from battle, prior to any speed check.
    ///
    /// Runs in the context of a Mon.
    #[string = "ForceEscape"]
    ForceEscape,
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
    /// Runs in the context of a Mon.
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
    /// Runs when calculating the amount of experience gained by a Mon.
    ///
    /// Runs in the context of a Mon.
    #[string = "ModifyExperience"]
    ModifyExperience,
    /// Runs when calculating the amount of friendship gained by a Mon.
    ///
    /// Runs in the context of a Mon.
    #[string = "ModifyFriendshipIncrease"]
    ModifyFriendshipIncrease,
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
    /// Runs when a move is aborted due to failing the BeforeMove event.
    ///
    /// Runs in the context of a move user.
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
    /// Runs in the context of a Mon.
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
    /// Runs when an item is used.
    ///
    /// Runs on the item.
    #[string = "Use"]
    Use,
    /// Runs when a Mon uses a move.
    ///
    /// Can be used to modify a move when it is used.
    ///
    /// Runs on the active move.
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
        match self {
            Self::AccuracyExempt => CommonCallbackType::MoveResult as u32,
            Self::AddPseudoWeather => CommonCallbackType::FieldEffectResult as u32,
            Self::AddVolatile => CommonCallbackType::ApplyingEffectResult as u32,
            Self::AfterAddPseudoWeather => CommonCallbackType::FieldEffectVoid as u32,
            Self::AfterAddVolatile => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::AfterHit => CommonCallbackType::MoveVoid as u32,
            Self::AfterMove => CommonCallbackType::SourceMoveVoid as u32,
            Self::AfterMoveSecondaryEffects => CommonCallbackType::MoveVoid as u32,
            Self::AfterSetItem => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::AfterSetStatus => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::AfterSubstituteDamage => CommonCallbackType::MoveVoid as u32,
            Self::AfterTakeItem => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::AfterUseItem => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::BasePower => CommonCallbackType::MoveModifier as u32,
            Self::BeforeMove => CommonCallbackType::SourceMoveResult as u32,
            Self::BeforeSwitchIn => CommonCallbackType::MonVoid as u32,
            Self::BeforeSwitchOut => CommonCallbackType::MonVoid as u32,
            Self::BeforeTurn => CommonCallbackType::MonVoid as u32,
            Self::BerryEatingHealth => CommonCallbackType::MonModifier as u32,
            Self::CanEscape => CommonCallbackType::MonResult as u32,
            Self::CanHeal => CommonCallbackType::MonResult as u32,
            Self::ChangeBoosts => CommonCallbackType::MonBoostModifier as u32,
            Self::ChargeMove => CommonCallbackType::SourceMoveResult as u32,
            Self::ClearTerrain => CommonCallbackType::FieldEffectResult as u32,
            Self::ClearWeather => CommonCallbackType::FieldEffectResult as u32,
            Self::CopyVolatile => CommonCallbackType::ApplyingEffectResult as u32,
            Self::CriticalHit => CommonCallbackType::MoveResult as u32,
            Self::CureStatus => CommonCallbackType::ApplyingEffectResult as u32,
            Self::Damage => CommonCallbackType::ApplyingEffectModifier as u32,
            Self::DamagingHit => CommonCallbackType::MoveVoid as u32,
            Self::DisableMove => CommonCallbackType::MonVoid as u32,
            Self::DeductPp => CommonCallbackType::MonModifier as u32,
            Self::DragOut => CommonCallbackType::MonResult as u32,
            Self::Duration => CommonCallbackType::ApplyingEffectModifier as u32,
            Self::Effectiveness => CommonCallbackType::MoveModifier as u32,
            Self::Eat => CommonCallbackType::MonVoid as u32,
            Self::EatItem => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::End => CommonCallbackType::EffectVoid as u32,
            Self::EndBattle => CommonCallbackType::MonVoid as u32,
            Self::EntryHazard => CommonCallbackType::MonVoid as u32,
            Self::Exit => CommonCallbackType::MonVoid as u32,
            Self::Faint => CommonCallbackType::MaybeApplyingEffectVoid as u32,
            Self::FieldEnd => CommonCallbackType::FieldVoid as u32,
            Self::FieldResidual => CommonCallbackType::FieldVoid as u32,
            Self::FieldRestart => CommonCallbackType::FieldResult as u32,
            Self::FieldStart => CommonCallbackType::FieldResult as u32,
            Self::Flinch => CommonCallbackType::MonVoid as u32,
            Self::ForceEscape => CommonCallbackType::MonResult as u32,
            Self::Hit => CommonCallbackType::MoveResult as u32,
            Self::HitField => CommonCallbackType::MoveFieldResult as u32,
            Self::HitSide => CommonCallbackType::MoveSideResult as u32,
            Self::IgnoreImmunity => CommonCallbackType::MoveResult as u32,
            Self::Immunity => CommonCallbackType::ApplyingEffectResult as u32,
            Self::Invulnerability => CommonCallbackType::MoveResult as u32,
            Self::IsAsleep => CommonCallbackType::MonResult as u32,
            Self::IsAwayFromField => CommonCallbackType::MonResult as u32,
            Self::IsBehindSubstitute => CommonCallbackType::MonResult as u32,
            Self::IsContactProof => CommonCallbackType::MonResult as u32,
            Self::IsGrounded => CommonCallbackType::MonResult as u32,
            Self::IsImmuneToEntryHazards => CommonCallbackType::MonResult as u32,
            Self::IsRaining => CommonCallbackType::NoContextResult as u32,
            Self::IsSemiInvulnerable => CommonCallbackType::MonResult as u32,
            Self::IsSnowing => CommonCallbackType::NoContextResult as u32,
            Self::IsSoundproof => CommonCallbackType::MonResult as u32,
            Self::IsSunny => CommonCallbackType::NoContextResult as u32,
            Self::LockMove => CommonCallbackType::MonInfo as u32,
            Self::ModifyAccuracy => CommonCallbackType::MoveModifier as u32,
            Self::ModifyActionSpeed => CommonCallbackType::MonModifier as u32,
            Self::ModifyAtk => CommonCallbackType::MaybeApplyingEffectModifier as u32,
            Self::ModifyBoosts => CommonCallbackType::MaybeApplyingEffectBoostModifier as u32,
            Self::ModifyCatchRate => CommonCallbackType::ApplyingEffectModifier as u32,
            Self::ModifyCritChance => CommonCallbackType::MoveModifier as u32,
            Self::ModifyCritRatio => CommonCallbackType::MoveModifier as u32,
            Self::ModifyDamage => CommonCallbackType::SourceMoveModifier as u32,
            Self::ModifyDef => CommonCallbackType::MaybeApplyingEffectModifier as u32,
            Self::ModifyExperience => CommonCallbackType::MonModifier as u32,
            Self::ModifyFriendshipIncrease => CommonCallbackType::MonModifier as u32,
            Self::ModifyPriority => CommonCallbackType::SourceMoveModifier as u32,
            Self::ModifySecondaryEffects => CommonCallbackType::MoveSecondaryEffectModifier as u32,
            Self::ModifySpA => CommonCallbackType::MaybeApplyingEffectModifier as u32,
            Self::ModifySpD => CommonCallbackType::MaybeApplyingEffectModifier as u32,
            Self::ModifySpe => CommonCallbackType::MaybeApplyingEffectModifier as u32,
            Self::ModifyStab => CommonCallbackType::SourceMoveModifier as u32,
            Self::ModifyTarget => CommonCallbackType::SourceMoveMonModifier as u32,
            Self::MoveAborted => CommonCallbackType::SourceMoveVoid as u32,
            Self::MoveBasePower => CommonCallbackType::MoveModifier as u32,
            Self::MoveDamage => CommonCallbackType::MoveModifier as u32,
            Self::MoveFailed => CommonCallbackType::SourceMoveVoid as u32,
            Self::MoveTargetOverride => CommonCallbackType::MonMoveTarget as u32,
            Self::NegateImmunity => CommonCallbackType::MonResult as u32,
            Self::OverrideMove => CommonCallbackType::MonInfo as u32,
            Self::PlayerTryUseItem => CommonCallbackType::EffectResult as u32,
            Self::PlayerUse => CommonCallbackType::MonVoid as u32,
            Self::PrepareHit => CommonCallbackType::SourceMoveControllingResult as u32,
            Self::PreventUsedItems => CommonCallbackType::MonResult as u32,
            Self::PriorityChargeMove => CommonCallbackType::MonVoid as u32,
            Self::RedirectTarget => CommonCallbackType::SourceMoveMonModifier as u32,
            Self::Residual => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::Restart => CommonCallbackType::EffectResult as u32,
            Self::RestorePp => CommonCallbackType::ApplyingEffectModifier as u32,
            Self::SetAbility => CommonCallbackType::ApplyingEffectResult as u32,
            Self::SetItem => CommonCallbackType::ApplyingEffectResult as u32,
            Self::SetLastMove => CommonCallbackType::MonResult as u32,
            Self::SetStatus => CommonCallbackType::ApplyingEffectResult as u32,
            Self::SetTerrain => CommonCallbackType::FieldEffectResult as u32,
            Self::SetWeather => CommonCallbackType::FieldEffectResult as u32,
            Self::SideConditionStart => CommonCallbackType::SideVoid as u32,
            Self::SideEnd => CommonCallbackType::SideVoid as u32,
            Self::SideResidual => CommonCallbackType::SideVoid as u32,
            Self::SideRestart => CommonCallbackType::SideResult as u32,
            Self::SideStart => CommonCallbackType::SideResult as u32,
            Self::SlotEnd => CommonCallbackType::SideResult as u32,
            Self::SlotRestart => CommonCallbackType::SideResult as u32,
            Self::SlotStart => CommonCallbackType::SideResult as u32,
            Self::StallMove => CommonCallbackType::MonResult as u32,
            Self::Start => CommonCallbackType::EffectResult as u32,
            Self::SubPriority => CommonCallbackType::SourceMoveModifier as u32,
            Self::SuppressFieldTerrain => CommonCallbackType::NoContextResult as u32,
            Self::SuppressFieldWeather => CommonCallbackType::NoContextResult as u32,
            Self::SuppressMonAbility => CommonCallbackType::MonResult as u32,
            Self::SuppressMonItem => CommonCallbackType::MonResult as u32,
            Self::SuppressMonTerrain => CommonCallbackType::MonResult as u32,
            Self::SuppressMonWeather => CommonCallbackType::MonResult as u32,
            Self::SwitchIn => CommonCallbackType::MonVoid as u32,
            Self::SwitchOut => CommonCallbackType::MonVoid as u32,
            Self::TakeItem => CommonCallbackType::ApplyingEffectResult as u32,
            Self::TrapMon => CommonCallbackType::MonResult as u32,
            Self::TryBoost => CommonCallbackType::ApplyingEffectBoostModifier as u32,
            Self::TryEatItem => CommonCallbackType::ApplyingEffectResult as u32,
            Self::TryHeal => CommonCallbackType::ApplyingEffectModifier as u32,
            Self::TryHit => CommonCallbackType::MoveControllingResult as u32,
            Self::TryHitField => CommonCallbackType::MoveFieldControllingResult as u32,
            Self::TryHitSide => CommonCallbackType::MoveSideControllingResult as u32,
            Self::TryImmunity => CommonCallbackType::MoveResult as u32,
            Self::TryMove => CommonCallbackType::SourceMoveControllingResult as u32,
            Self::TryPrimaryHit => CommonCallbackType::MoveHitOutcomeResult as u32,
            Self::TryUseItem => CommonCallbackType::ApplyingEffectResult as u32,
            Self::TryUseMove => CommonCallbackType::SourceMoveControllingResult as u32,
            Self::TypeImmunity => CommonCallbackType::MonResult as u32,
            Self::Types => CommonCallbackType::MonTypes as u32,
            Self::Update => CommonCallbackType::MonVoid as u32,
            Self::Use => CommonCallbackType::MonVoid as u32,
            Self::UseMove => CommonCallbackType::SourceMoveVoid as u32,
            Self::UseMoveMessage => CommonCallbackType::SourceMoveVoid as u32,
            Self::ValidateMon => CommonCallbackType::MonValidator as u32,
            Self::ValidateTeam => CommonCallbackType::PlayerValidator as u32,
            Self::Weather => CommonCallbackType::ApplyingEffectResult as u32,
            Self::WeatherChange => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::WeatherModifyDamage => CommonCallbackType::SourceMoveModifier as u32,
        }
    }

    /// Checks if the event has the given [`CallbackFlag`] flag set.
    pub fn has_flag(&self, flag: u32) -> bool {
        self.callback_type_flags() & flag != 0
    }

    /// The name of the input variable by index.
    pub fn input_vars(&self) -> &[(&str, ValueType, bool)] {
        match self {
            Self::AddPseudoWeather | Self::AfterAddPseudoWeather => {
                &[("pseudo_weather", ValueType::Effect, true)]
            }
            Self::AddVolatile | Self::AfterAddVolatile => &[("volatile", ValueType::Effect, true)],
            Self::AfterSetItem | Self::AfterTakeItem | Self::AfterUseItem => {
                &[("item", ValueType::Effect, true)]
            }
            Self::BasePower => &[("base_power", ValueType::UFraction, true)],
            Self::BerryEatingHealth => &[("hp", ValueType::UFraction, true)],
            Self::ChangeBoosts => &[("boosts", ValueType::BoostTable, true)],
            Self::Damage => &[("damage", ValueType::UFraction, true)],
            Self::DeductPp => &[("pp", ValueType::UFraction, true)],
            Self::DamagingHit => &[("damage", ValueType::UFraction, true)],
            Self::EatItem => &[("item", ValueType::Effect, true)],
            Self::Effectiveness => &[
                ("modifier", ValueType::Fraction, true),
                ("type", ValueType::Type, true),
            ],
            Self::ModifyAccuracy => &[("acc", ValueType::UFraction, true)],
            Self::ModifyActionSpeed => &[("spe", ValueType::UFraction, true)],
            Self::ModifyAtk => &[("atk", ValueType::UFraction, true)],
            Self::ModifyBoosts => &[("boosts", ValueType::BoostTable, true)],
            Self::ModifyCatchRate => &[("catch_rate", ValueType::UFraction, true)],
            Self::ModifyCritChance => &[("chance", ValueType::UFraction, true)],
            Self::ModifyCritRatio => &[("crit_ratio", ValueType::UFraction, true)],
            Self::ModifyDamage | Self::WeatherModifyDamage => {
                &[("damage", ValueType::UFraction, true)]
            }
            Self::ModifyDef => &[("def", ValueType::UFraction, true)],
            Self::ModifyExperience => &[("exp", ValueType::UFraction, true)],
            Self::ModifyFriendshipIncrease => &[("friendship", ValueType::UFraction, true)],
            Self::ModifyPriority => &[("priority", ValueType::Fraction, true)],
            Self::ModifySecondaryEffects => &[("secondary_effects", ValueType::List, true)],
            Self::ModifySpA => &[("spa", ValueType::UFraction, true)],
            Self::ModifySpD => &[("spd", ValueType::UFraction, true)],
            Self::ModifySpe => &[("spe", ValueType::UFraction, true)],
            Self::ModifyStab => &[("stab", ValueType::UFraction, true)],
            Self::ModifyTarget => &[("target", ValueType::Mon, false)],
            Self::NegateImmunity => &[("type", ValueType::Type, true)],
            Self::OverrideMove => &[("move", ValueType::ActiveMove, true)],
            Self::PlayerTryUseItem => &[("input", ValueType::Object, true)],
            Self::PlayerUse => &[("input", ValueType::Object, true)],
            Self::RedirectTarget => &[("target", ValueType::Mon, true)],
            Self::RestorePp => &[("pp", ValueType::UFraction, true)],
            Self::SetAbility => &[("ability", ValueType::Effect, true)],
            Self::SetItem => &[("item", ValueType::Effect, true)],
            Self::SetStatus | Self::AfterSetStatus => &[("status", ValueType::Effect, true)],
            Self::SetTerrain => &[("terrain", ValueType::Effect, true)],
            Self::SetWeather => &[("weather", ValueType::Effect, true)],
            Self::SideConditionStart => &[("condition", ValueType::Effect, true)],
            Self::SlotEnd => &[("slot", ValueType::UFraction, true)],
            Self::SlotRestart => &[("slot", ValueType::UFraction, true)],
            Self::SlotStart => &[("slot", ValueType::UFraction, true)],
            Self::SubPriority => &[("sub_priority", ValueType::Fraction, true)],
            Self::TakeItem => &[("item", ValueType::Effect, true)],
            Self::TryBoost => &[("boosts", ValueType::BoostTable, true)],
            Self::TryEatItem => &[("item", ValueType::Effect, true)],
            Self::TryUseItem => &[("item", ValueType::Effect, true)],
            Self::TryHeal => &[("damage", ValueType::UFraction, true)],
            Self::TypeImmunity => &[("type", ValueType::Type, true)],
            Self::Types => &[("types", ValueType::List, true)],
            Self::UseMove => &[("selected_target", ValueType::Mon, false)],
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
            Some(ValueType::Boolean) => self.has_flag(CallbackFlag::ReturnsBoolean),
            Some(ValueType::String) => self.has_flag(
                CallbackFlag::ReturnsString
                    | CallbackFlag::ReturnsMoveResult
                    | CallbackFlag::ReturnsMoveTarget,
            ),
            Some(ValueType::BoostTable) => self.has_flag(CallbackFlag::ReturnsBoosts),
            Some(ValueType::Mon) => self.has_flag(CallbackFlag::ReturnsMon),
            Some(ValueType::List) => self.has_flag(
                CallbackFlag::ReturnsTypes
                    | CallbackFlag::ReturnsSecondaryEffects
                    | CallbackFlag::ReturnsStrings,
            ),
            Some(ValueType::MoveTarget) => self.has_flag(CallbackFlag::ReturnsMoveTarget),
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
            Self::SuppressMonItem => 0,
            Self::SuppressMonAbility => 1,
            Self::Types => 2,
            Self::IsGrounded => 3,
            Self::IsSemiInvulnerable => 3,
            Self::SuppressFieldTerrain => 4,
            Self::SuppressFieldWeather => 4,
            Self::SuppressMonTerrain => 5,
            Self::SuppressMonWeather => 5,
            _ => usize::MAX,
        }
    }

    /// Whether or not to run the event callback on the source effect when running all callbacks for
    /// an event.
    pub fn run_callback_on_source_effect(&self) -> bool {
        match self {
            Self::Damage => true,
            Self::ModifyTarget => true,
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

/// An fxlang program with priority information for ordering.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgramWithPriority {
    pub program: Option<Program>,
    pub order: Option<u32>,
    pub priority: Option<i32>,
    pub sub_order: Option<u32>,
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
    /// Can be overwritten by the [`on_duration`][`Callbacks::on_duration`] callback.
    pub duration: Option<u8>,

    /// Whether or not the effect can be copied to another Mon.
    ///
    /// If true, moves like "Baton Pass" will not copy this effect. `false` by default.
    #[serde(default)]
    pub no_copy: bool,
}

impl ConditionAttributes {
    /// Extends the condition attributes with some other attribute object, overriding data if
    /// applicable.
    pub fn extend(&mut self, other: Self) {
        if let Some(duration) = other.duration {
            self.duration = Some(duration);
        }
        self.no_copy = other.no_copy || self.no_copy;
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

    #[serde(flatten)]
    pub attributes: EffectAttributes,
}

impl TryFrom<serde_json::Value> for Effect {
    type Error = Error;
    fn try_from(value: serde_json::Value) -> Result<Self, Self::Error> {
        serde_json::from_value(value).wrap_error_with_message("invalid fxlang effect")
    }
}
