use serde::{
    Deserialize,
    Serialize,
};
use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

use crate::{
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
    pub const TakesBoosts: u32 = 1 << 10;

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
        | CallbackFlag::TakesBoosts
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

    SourceMoveModifier = CallbackFlag::TakesUserMon
        | CallbackFlag::TakesSourceTargetMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsNumber
        | CallbackFlag::ReturnsVoid,
    SourceMoveResult = CallbackFlag::TakesUserMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsVoid,
    SourceMoveVoid =
        CallbackFlag::TakesUserMon | CallbackFlag::TakesActiveMove | CallbackFlag::ReturnsVoid,
    SourceMoveControllingResult = CallbackFlag::TakesUserMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsMoveResult
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsVoid,
    SourceMoveMonModifier = CallbackFlag::TakesUserMon
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

    MonModifier =
        CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsNumber | CallbackFlag::ReturnsVoid,
    MonResult =
        CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsBoolean | CallbackFlag::ReturnsVoid,
    MonVoid = CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsVoid,
    MonInfo = CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsString,
    MonTypes = CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsTypes,

    SideVoid = CallbackFlag::TakesSide | CallbackFlag::TakesSourceMon | CallbackFlag::ReturnsVoid,
    SideResult = CallbackFlag::TakesSide
        | CallbackFlag::TakesSourceMon
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
    /// Runs in the context of an active move on the target.
    #[string = "AccuracyExempt"]
    AccuracyExempt,
    /// Runs after a volatile effect is added to a Mon.
    ///
    /// Runs in the context of an applying effect.
    #[string = "AddVolatile"]
    AddVolatile,
    /// Runs after a Mon finishes using a move.
    ///
    /// Runs on the active move itself and in the context of an active move from the user.
    #[string = "AfterMove"]
    AfterMove,
    /// Runs after a move's secondary effects have been applied.
    ///
    /// Should be viewed as the last effect the move needs to apply.
    ///
    /// Runs on the active move itself and the context of an active move on the target.
    #[string = "AfterMoveSecondaryEffects"]
    AfterMoveSecondaryEffects,
    /// Runs after a Mon's status effect is changed.
    ///
    /// Only runs if the status has been set successfully. This event will not undo a status
    /// change.
    ///
    /// Runs in the context of an applying effect.
    #[string = "AfterSetStatus"]
    AfterSetStatus,
    /// Runs after damage is applied to a substitute.
    ///
    /// Hitting a substitute does not trigger ordinary effects that run when a target is hit. Thus,
    /// this event is used to cover for scenarios where hitting a substitute should still trigger
    /// some callback.
    ///
    /// Runs on the active move itself and in the context of an active move on the target.
    #[string = "AfterSubstituteDamage"]
    AfterSubstituteDamage,
    /// Runs when a Mon's ally's status effect is changed.
    ///
    /// Runs in the context of an applying effect.
    #[string = "AllySetStatus"]
    AllySetStatus,
    /// Runs before a Mon uses a move.
    ///
    /// Can prevent the move from being used.
    ///
    /// Runs in the context of an active move from the user.
    #[string = "BeforeMove"]
    BeforeMove,
    /// Runs before a turn of a battle.
    ///
    /// Runs in the context of an applying effect on the target.
    #[string = "BeforeTurn"]
    BeforeTurn,
    /// Runs when a move's base power is being calculated for a target.
    ///
    /// Used to apply dynamic base powers.
    ///
    /// Runs on the active move itself.
    #[string = "BasePower"]
    BasePower,
    /// Runs when a Mon is using a charge move, on the charging turn.
    ///
    /// Runs in the context of an active move from the user.
    #[string = "ChargeMove"]
    ChargeMove,
    /// Runs when a move's damage is being calculated for a target.
    ///
    /// Used to override damage calculations.
    ///
    /// Runs on the active move itself.
    #[string = "Damage"]
    Damage,
    /// Runs after a Mon receives damage, regardless of the source.
    ///
    /// Runs in the context of an applying effect on the target.
    #[string = "DamageReceived"]
    DamageReceived,
    /// Runs after a Mon hits another Mon with a move, causing a nonzero amount of damage.
    ///
    /// Run for each target. Run once per hit (i.e., multi-hit moves execute one event per hit).
    ///
    /// Runs in the context of an active move on the target.
    #[string = "DamagingHit"]
    DamagingHit,
    /// Runs after a move is used that should have PP deducted.
    ///
    /// Runs in the context of the target Mon.
    #[string = "DeductPp"]
    DeductPp,
    /// Runs when determining which moves are disabled.
    ///
    /// Runs in the context of the target Mon.
    #[string = "DisableMove"]
    DisableMove,
    /// Runs when determining the duration of an effect.
    ///
    /// Used to apply dynamic durations.
    ///
    /// Runs on the effect itself.
    #[string = "Duration"]
    Duration,
    /// Runs when an effect ends.
    ///
    /// Runs on the effect itself.
    #[string = "End"]
    End,
    /// Runs when a Mon flinches.
    ///
    /// Runs in the context of the target Mon.
    #[string = "Flinch"]
    Flinch,
    /// Runs when a Mon is hit by a move.
    ///
    /// Can fail, but will only fail the move if everything else failed. Can be viewed as part of
    /// the applying hit effect.
    ///
    /// Runs on the active move itself and in the context of an active move on the target.
    #[string = "Hit"]
    Hit,
    /// Runs when the field is hit by a move.
    ///
    /// Can fail, but will only fail the move if everything else failed. Can be viewed as part of
    /// the applying hit effect.
    ///
    /// Runs on the active move itself.
    #[string = "HitField"]
    HitField,
    /// Runs when a side is hit by a move.
    ///
    /// Can fail, but will only fail the move if everything else failed. Can be viewed as part of
    /// the applying hit effect.
    ///
    /// Runs on the active move itself.
    #[string = "HitSide"]
    HitSide,
    /// Runs when determining if a Mon is immune to some status.
    ///
    /// Runs in the context of an applying effect on the target, including moves.
    #[string = "Immunity"]
    Immunity,
    /// Runs when determining if a Mon is invulnerable to targeting moves.
    ///
    /// Runs as the very first step in a move.
    ///
    /// Runs in the context of an active move on the target.
    #[string = "Invulnerability"]
    Invulnerability,
    /// Runs when determining if a Mon is asleep.
    ///
    /// Runs in the context of the target Mon.
    #[string = "IsAsleep"]
    IsAsleep,
    /// Runs when determining if a Mon is locked into a move.
    ///
    /// Runs in the context of the target Mon.
    #[string = "LockMove"]
    LockMove,
    /// Runs when calculating a Mon's Atk stat.
    ///
    /// Runs in the context of the target Mon.
    #[string = "ModifyAtk"]
    ModifyAtk,
    /// Runs when calculating a move's critical hit ratio.
    ///
    /// Runs in the context of an active move from the user.
    #[string = "ModifyCritRatio"]
    ModifyCritRatio,
    /// Runs when calculating the damage applied to a Mon.
    ///
    /// Runs as the very last step in the regular damage calculation formula.
    ///
    /// Runs in the context of an active move from the user.
    #[string = "ModifyDamage"]
    ModifyDamage,
    /// Runs when calculating a Mon's Def stat.
    ///
    /// Runs in the context of the target Mon.
    #[string = "ModifyDef"]
    ModifyDef,
    /// Runs when calculating a Mon's SpA stat.
    ///
    /// Runs in the context of the target Mon.
    #[string = "ModifySpA"]
    ModifySpA,
    /// Runs when caclculating a Mon's SpD stat.
    ///
    /// Runs in the context of the target Mon.
    #[string = "ModifySpD"]
    ModifySpD,
    /// Runs when calculating a Mon's Spe stat.
    ///
    /// Runs in the context of the target Mon.
    #[string = "ModifySpe"]
    ModifySpe,
    /// Runs when a move is aborted due to failing the BeforeMove event.
    ///
    /// Runs in the context of an active move from the user.
    #[string = "MoveAborted"]
    MoveAborted,
    /// Runs when a move fails, only on the move itself.
    ///
    /// A move fails when it is successfully used by the user, but it does not hit or apply its
    /// primary effect to any targets.
    ///
    /// Runs on the active move itself.
    #[string = "MoveFailed"]
    MoveFailed,
    /// Runs when a Mon is preparing to hit all of its targets with a move.
    ///
    /// Can fail the move.
    ///
    /// Runs on the active move itself and in the context of an active move from the user.
    #[string = "PrepareHit"]
    PrepareHit,
    /// Runs when a move is going to target one Mon but can be redirected towards a different
    /// target.
    ///
    /// Runs in the context of an active move from the user.
    #[string = "RedirectTarget"]
    RedirectTarget,
    /// Runs at the end of every turn to apply residual effects.
    ///
    /// Runs in the context of an applying effect on the target.
    #[string = "Residual"]
    Residual,
    /// Runs when a volatile effect is applied to a Mon that already has the volatile effect.
    ///
    /// Runs on the effect itself.
    #[string = "Restart"]
    Restart,
    /// Runs when the Mon's last move selected is being set.
    ///
    /// Runs in the context of the target Mon.
    #[string = "SetLastMove"]
    SetLastMove,
    /// Runs when a Mon's status effect is being set.
    ///
    /// Runs before the status effect is applied. Can be used to fail the status change.
    ///
    /// Runs in the context of an applying effect on the target.
    #[string = "SetStatus"]
    SetStatus,
    /// Runs when a side condition starts successfully.
    ///
    /// Runs in the context of an applying effect on the side.
    #[string = "SideConditionStart"]
    SideConditionStart,
    /// Runs when a side condition ends.
    ///
    /// Runs in the context of the side condition itself.
    #[string = "SideEnd"]
    SideEnd,
    /// Runs at the end of every turn to apply residual effects on the side.
    ///
    /// Runs in the context of the side condition itself.
    #[string = "SideResidual"]
    SideResidual,
    /// Runs when a side condition restarts.
    ///
    /// Runs in the context of the side condition itself.
    #[string = "SideRestart"]
    SideRestart,
    /// Runs when a side condition starts.
    ///
    /// Runs in the context of the side condition itself.
    #[string = "SideStart"]
    SideStart,
    /// Runs when a Mon is the target of a damage calculation (i.e., a Mon is calculating damage to
    /// apply against it).
    ///
    /// Used to modify damage calculations impacted by effets on the target Mon.
    ///
    /// Runs in the context of an active move from the user.
    #[string = "SourceModifyDamage"]
    SourceModifyDamage,
    /// Runs when an effect starts.
    ///
    /// Used to set up state.
    ///
    /// Runs on the effect itself.
    #[string = "Start"]
    Start,
    /// Runs when a Mon switches in.
    ///
    /// Runs in the context of the target Mon.
    #[string = "SwitchIn"]
    SwitchIn,
    /// Runs when determining if a Mon is trapped (i.e., cannot switch out).
    ///
    /// Runs in the context of the target Mon.
    #[string = "TrapMon"]
    TrapMon,
    /// Runs when a group of stat boosts is being applied to a Mon.
    ///
    /// Runs in the context of an applying effect on the target.
    #[string = "TryBoost"]
    TryBoost,
    /// Runs when a move is trying to hit a set of targets.
    ///
    /// Can fail the move.
    ///
    /// Runs on the active move itself and in the context of an applying effect on each target.
    #[string = "TryHit"]
    TryHit,
    /// Runs when a move is trying to hit the whole field.
    ///
    /// Can fail the move.
    ///
    /// Runs on the active move itself and in the context of an applying effect on the field.
    #[string = "TryHitField"]
    TryHitField,
    /// Runs when a move is trying to hit the whole field.
    ///
    /// Can fail the move.
    ///
    /// Runs on the active move itself and in the context of an applying effect on the side.
    #[string = "TryHitSide"]
    TryHitSide,
    /// Runs when a move is checking general immunity for its target.
    ///
    /// Can fail the move (by marking the target as immune).
    ///
    /// Runs in the context of the active move itself.
    #[string = "TryImmunity"]
    TryImmunity,
    /// Runs when a move's primary hit is being applied to a target.
    ///
    /// Used to override the core battle engine logic. Can fail the move or return an amount of
    /// damage dealt to the target. If zero damage is returned, the core battle engien assumes a
    /// substitute was hit for the purposes of hit effects (i.e., hit effects do not apply to the
    /// target).
    ///
    /// Runs in the context of an active move on the target.
    #[string = "TryPrimaryHit"]
    TryPrimaryHit,
    /// Runs when a Mon is trying to use a move on a set of targets.
    ///
    /// Can fail the move.
    ///
    /// Runs on the active move itself.
    #[string = "TryUseMove"]
    TryUseMove,
    /// Runs when determining the types of a Mon.
    ///
    /// Runs in the context of the target Mon.
    #[string = "Types"]
    Types,
    /// Runs when a Mon uses a move.
    ///
    /// Can be used to modify a move when it is used.
    ///
    /// Runs on the active move itself.
    #[string = "UseMove"]
    UseMove,
    /// Runs when a custom message should be displayed when a Mon uses a move.
    ///
    /// Runs on the active move itself.
    #[string = "UseMoveMessage"]
    UseMoveMessage,
}

impl BattleEvent {
    /// Maps the event to the [`CallbackFlag`] flags.
    pub fn callback_type_flags(&self) -> u32 {
        match self {
            Self::AccuracyExempt => CommonCallbackType::MoveResult as u32,
            Self::AddVolatile => CommonCallbackType::ApplyingEffectResult as u32,
            Self::AfterMove => CommonCallbackType::SourceMoveVoid as u32,
            Self::AfterMoveSecondaryEffects => CommonCallbackType::MoveVoid as u32,
            Self::AfterSetStatus => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::AfterSubstituteDamage => CommonCallbackType::MoveVoid as u32,
            Self::AllySetStatus => CommonCallbackType::ApplyingEffectResult as u32,
            Self::BasePower => CommonCallbackType::MoveModifier as u32,
            Self::BeforeMove => CommonCallbackType::SourceMoveResult as u32,
            Self::BeforeTurn => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::ChargeMove => CommonCallbackType::SourceMoveVoid as u32,
            Self::Damage => CommonCallbackType::MoveModifier as u32,
            Self::DamageReceived => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::DamagingHit => CommonCallbackType::MoveVoid as u32,
            Self::DisableMove => CommonCallbackType::MonVoid as u32,
            Self::DeductPp => CommonCallbackType::MonModifier as u32,
            Self::Duration => CommonCallbackType::ApplyingEffectModifier as u32,
            Self::End => CommonCallbackType::EffectVoid as u32,
            Self::Flinch => CommonCallbackType::MonVoid as u32,
            Self::Hit => CommonCallbackType::MoveResult as u32,
            Self::HitField => CommonCallbackType::MoveFieldResult as u32,
            Self::HitSide => CommonCallbackType::MoveSideResult as u32,
            Self::Immunity => CommonCallbackType::ApplyingEffectResult as u32,
            Self::Invulnerability => CommonCallbackType::MoveResult as u32,
            Self::IsAsleep => CommonCallbackType::MonResult as u32,
            Self::LockMove => CommonCallbackType::MonInfo as u32,
            Self::ModifyAtk => CommonCallbackType::MonModifier as u32,
            Self::ModifyCritRatio => CommonCallbackType::MoveModifier as u32,
            Self::ModifyDamage => CommonCallbackType::SourceMoveModifier as u32,
            Self::ModifyDef => CommonCallbackType::MonModifier as u32,
            Self::ModifySpA => CommonCallbackType::MonModifier as u32,
            Self::ModifySpD => CommonCallbackType::MonModifier as u32,
            Self::ModifySpe => CommonCallbackType::MonModifier as u32,
            Self::MoveAborted => CommonCallbackType::SourceMoveVoid as u32,
            Self::MoveFailed => CommonCallbackType::SourceMoveVoid as u32,
            Self::PrepareHit => CommonCallbackType::SourceMoveResult as u32,
            Self::RedirectTarget => CommonCallbackType::SourceMoveMonModifier as u32,
            Self::Residual => CommonCallbackType::ApplyingEffectVoid as u32,
            Self::Restart => CommonCallbackType::EffectResult as u32,
            Self::SetLastMove => CommonCallbackType::MonResult as u32,
            Self::SetStatus => CommonCallbackType::ApplyingEffectResult as u32,
            Self::SideConditionStart => CommonCallbackType::SideVoid as u32,
            Self::SideEnd => CommonCallbackType::SideVoid as u32,
            Self::SideResidual => CommonCallbackType::SideVoid as u32,
            Self::SideRestart => CommonCallbackType::SideResult as u32,
            Self::SideStart => CommonCallbackType::SideResult as u32,
            Self::SourceModifyDamage => CommonCallbackType::SourceMoveModifier as u32,
            Self::Start => CommonCallbackType::EffectResult as u32,
            Self::SwitchIn => CommonCallbackType::MonVoid as u32,
            Self::TrapMon => CommonCallbackType::MonVoid as u32,
            Self::TryBoost => CommonCallbackType::ApplyingEffectBoostModifier as u32,
            Self::TryHit => CommonCallbackType::MoveControllingResult as u32,
            Self::TryHitField => CommonCallbackType::MoveFieldControllingResult as u32,
            Self::TryHitSide => CommonCallbackType::MoveSideControllingResult as u32,
            Self::TryImmunity => CommonCallbackType::MoveResult as u32,
            Self::TryPrimaryHit => CommonCallbackType::MoveHitOutcomeResult as u32,
            Self::TryUseMove => CommonCallbackType::SourceMoveControllingResult as u32,
            Self::Types => CommonCallbackType::MonTypes as u32,
            Self::UseMove => CommonCallbackType::SourceMoveVoid as u32,
            Self::UseMoveMessage => CommonCallbackType::SourceMoveVoid as u32,
        }
    }

    /// Checks if the event has the given [`CallbackFlag`] flag set.
    pub fn has_flag(&self, flag: u32) -> bool {
        self.callback_type_flags() & flag != 0
    }

    /// The name of the input variable by index.
    pub fn input_vars(&self) -> &[(&str, ValueType)] {
        match self {
            Self::AddVolatile => &[("volatile", ValueType::Effect)],
            Self::DeductPp => &[("pp", ValueType::U64)],
            Self::DamageReceived => &[("damage", ValueType::U64)],
            Self::DamagingHit => &[("damage", ValueType::U64)],
            Self::ModifyAtk => &[("atk", ValueType::U64)],
            Self::ModifyCritRatio => &[("crit_ratio", ValueType::U64)],
            Self::ModifyDamage | Self::SourceModifyDamage => &[("damage", ValueType::U64)],
            Self::ModifyDef => &[("def", ValueType::U64)],
            Self::ModifySpA => &[("spa", ValueType::U64)],
            Self::ModifySpD => &[("spd", ValueType::U64)],
            Self::ModifySpe => &[("spe", ValueType::U64)],
            Self::RedirectTarget => &[("target", ValueType::Mon)],
            Self::SetStatus | Self::AllySetStatus | Self::AfterSetStatus => {
                &[("status", ValueType::Effect)]
            }
            Self::SideConditionStart => &[("condition", ValueType::Effect)],
            Self::TryBoost => &[("boosts", ValueType::BoostTable)],
            Self::Types => &[("types", ValueType::List)],
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
            Some(ValueType::String) => {
                self.has_flag(CallbackFlag::ReturnsString | CallbackFlag::ReturnsMoveResult)
            }
            Some(ValueType::BoostTable) => self.has_flag(CallbackFlag::ReturnsBoosts),
            Some(ValueType::Mon) => self.has_flag(CallbackFlag::ReturnsMon),
            Some(ValueType::List) => self.has_flag(CallbackFlag::ReturnsTypes),
            None => self.has_flag(CallbackFlag::ReturnsVoid),
            _ => false,
        }
    }

    /// Returns the associated ally event.
    pub fn ally_event(&self) -> Option<BattleEvent> {
        match self {
            Self::SetStatus => Some(Self::AllySetStatus),
            _ => None,
        }
    }

    /// Returns the associated foe event.
    pub fn foe_event(&self) -> Option<BattleEvent> {
        match self {
            _ => None,
        }
    }

    /// Returns the associated source event.
    pub fn source_event(&self) -> Option<BattleEvent> {
        match self {
            Self::ModifyDamage => Some(Self::SourceModifyDamage),
            _ => None,
        }
    }

    /// Returns the associated any event.
    pub fn any_event(&self) -> Option<BattleEvent> {
        match self {
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
    pub program: Program,
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
        self.0.is_some()
    }

    /// Returns a reference to the callback's [`Program`].
    pub fn program(&self) -> Option<&Program> {
        match self.0.as_ref()? {
            CallbackInput::Regular(program) => Some(&program),
            CallbackInput::WithPriority(program) => Some(&program.program),
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
///
/// All possible callbacks for an effect should be defined here.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Callbacks {
    pub is_asleep: Callback,
    pub on_accuracy_exempt: Callback,
    pub on_add_volatile: Callback,
    pub on_after_move: Callback,
    pub on_after_move_secondary_effects: Callback,
    pub on_after_set_status: Callback,
    pub on_after_substitute_damage: Callback,
    pub on_ally_set_status: Callback,
    pub on_base_power: Callback,
    pub on_before_move: Callback,
    pub on_before_turn: Callback,
    pub on_charge_move: Callback,
    pub on_damage: Callback,
    pub on_damage_received: Callback,
    pub on_damaging_hit: Callback,
    pub on_disable_move: Callback,
    pub on_deduct_pp: Callback,
    pub on_duration: Callback,
    pub on_end: Callback,
    pub on_flinch: Callback,
    pub on_hit: Callback,
    pub on_hit_field: Callback,
    pub on_hit_side: Callback,
    pub on_immunity: Callback,
    pub on_invulnerability: Callback,
    pub on_lock_move: Callback,
    pub on_modify_atk: Callback,
    pub on_modify_crit_ratio: Callback,
    pub on_modify_damage: Callback,
    pub on_modify_def: Callback,
    pub on_modify_spa: Callback,
    pub on_modify_spd: Callback,
    pub on_modify_spe: Callback,
    pub on_move_aborted: Callback,
    pub on_move_failed: Callback,
    pub on_prepare_hit: Callback,
    pub on_redirect_target: Callback,
    pub on_residual: Callback,
    pub on_restart: Callback,
    pub on_set_last_move: Callback,
    pub on_set_status: Callback,
    pub on_side_condition_start: Callback,
    pub on_side_end: Callback,
    pub on_side_residual: Callback,
    pub on_side_restart: Callback,
    pub on_side_start: Callback,
    pub on_source_modify_damage: Callback,
    pub on_start: Callback,
    pub on_switch_in: Callback,
    pub on_trap_mon: Callback,
    pub on_try_boost: Callback,
    pub on_try_hit: Callback,
    pub on_try_hit_field: Callback,
    pub on_try_hit_side: Callback,
    pub on_try_immunity: Callback,
    pub on_try_primary_hit: Callback,
    pub on_try_use_move: Callback,
    pub on_types: Callback,
    pub on_use_move: Callback,
    pub on_use_move_message: Callback,
}

impl Callbacks {
    pub fn event(&self, event: BattleEvent) -> Option<&Callback> {
        match event {
            BattleEvent::AccuracyExempt => Some(&self.on_accuracy_exempt),
            BattleEvent::AddVolatile => Some(&self.on_add_volatile),
            BattleEvent::AfterMove => Some(&self.on_after_move),
            BattleEvent::AfterMoveSecondaryEffects => Some(&self.on_after_move_secondary_effects),
            BattleEvent::AfterSetStatus => Some(&self.on_after_set_status),
            BattleEvent::AfterSubstituteDamage => Some(&self.on_after_substitute_damage),
            BattleEvent::AllySetStatus => Some(&self.on_ally_set_status),
            BattleEvent::BasePower => Some(&self.on_base_power),
            BattleEvent::BeforeMove => Some(&self.on_before_move),
            BattleEvent::BeforeTurn => Some(&self.on_before_turn),
            BattleEvent::ChargeMove => Some(&self.on_charge_move),
            BattleEvent::Damage => Some(&self.on_damage),
            BattleEvent::DamageReceived => Some(&self.on_damage_received),
            BattleEvent::DamagingHit => Some(&self.on_damaging_hit),
            BattleEvent::DeductPp => Some(&self.on_deduct_pp),
            BattleEvent::DisableMove => Some(&self.on_disable_move),
            BattleEvent::Duration => Some(&self.on_duration),
            BattleEvent::End => Some(&self.on_end),
            BattleEvent::Flinch => Some(&self.on_flinch),
            BattleEvent::Hit => Some(&self.on_hit),
            BattleEvent::HitField => Some(&self.on_hit_field),
            BattleEvent::HitSide => Some(&self.on_hit_side),
            BattleEvent::Immunity => Some(&self.on_immunity),
            BattleEvent::Invulnerability => Some(&self.on_invulnerability),
            BattleEvent::IsAsleep => Some(&self.is_asleep),
            BattleEvent::LockMove => Some(&self.on_lock_move),
            BattleEvent::ModifyAtk => Some(&self.on_modify_atk),
            BattleEvent::ModifyCritRatio => Some(&self.on_modify_crit_ratio),
            BattleEvent::ModifyDamage => Some(&self.on_modify_damage),
            BattleEvent::ModifyDef => Some(&self.on_modify_def),
            BattleEvent::ModifySpA => Some(&self.on_modify_spa),
            BattleEvent::ModifySpD => Some(&self.on_modify_spd),
            BattleEvent::ModifySpe => Some(&self.on_modify_spe),
            BattleEvent::MoveAborted => Some(&self.on_move_aborted),
            BattleEvent::MoveFailed => Some(&self.on_move_failed),
            BattleEvent::PrepareHit => Some(&self.on_prepare_hit),
            BattleEvent::RedirectTarget => Some(&self.on_redirect_target),
            BattleEvent::Residual => Some(&self.on_residual),
            BattleEvent::Restart => Some(&self.on_restart),
            BattleEvent::SetLastMove => Some(&self.on_set_last_move),
            BattleEvent::SetStatus => Some(&self.on_set_status),
            BattleEvent::SideConditionStart => Some(&self.on_side_condition_start),
            BattleEvent::SideEnd => Some(&self.on_side_end),
            BattleEvent::SideResidual => Some(&self.on_side_residual),
            BattleEvent::SideRestart => Some(&self.on_side_restart),
            BattleEvent::SideStart => Some(&self.on_side_start),
            BattleEvent::SourceModifyDamage => Some(&self.on_source_modify_damage),
            BattleEvent::Start => Some(&self.on_start),
            BattleEvent::SwitchIn => Some(&self.on_switch_in),
            BattleEvent::TrapMon => Some(&self.on_trap_mon),
            BattleEvent::TryBoost => Some(&self.on_try_boost),
            BattleEvent::TryHit => Some(&self.on_try_hit),
            BattleEvent::TryHitField => Some(&self.on_try_hit_field),
            BattleEvent::TryHitSide => Some(&self.on_try_hit_side),
            BattleEvent::TryImmunity => Some(&self.on_try_immunity),
            BattleEvent::TryPrimaryHit => Some(&self.on_try_primary_hit),
            BattleEvent::TryUseMove => Some(&self.on_try_use_move),
            BattleEvent::Types => Some(&self.on_types),
            BattleEvent::UseMove => Some(&self.on_use_move),
            BattleEvent::UseMoveMessage => Some(&self.on_use_move_message),
        }
    }
}

/// An effect, whose callbacks are triggered in the context of an ongoing battle.
///
/// When an effect is active, its event callbacks are triggered throughout the course of a battle.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Effect {
    /// Event callbacks for the effect.
    pub callbacks: Callbacks,

    /// Local data for the effects.
    #[serde(default)]
    pub local_data: LocalData,
}

/// A condition enabled by an effect.
///
/// While an effect has its own set of callbacks, an effect can also apply a condition to some
/// entity, which will repeatedly apply its callbacks for the specified duration.
///
/// Note that an effect's condition can outlive the effect itself.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Condition {
    /// The static duration of the condition.
    ///
    /// Can be overwritten by the [`on_duration`][`Callbacks::on_duration`] callback.
    pub duration: Option<u8>,

    /// Whether or not the condition can be copied to another Mon.
    ///
    /// If true, moves like "Baton Pass" will not copy this condition. `false` by default.
    #[serde(default)]
    pub no_copy: bool,

    /// The effect of the condition.
    #[serde(flatten)]
    pub effect: Effect,
}
