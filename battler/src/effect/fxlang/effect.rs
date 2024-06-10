use std::{
    fmt,
    fmt::Display,
};

use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    battle::SpeedOrderable,
    effect::fxlang::ValueType,
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

    pub const ReturnsNumber: u32 = 1 << 29;
    pub const ReturnsBoolean: u32 = 1 << 30;
    pub const ReturnsVoid: u32 = 1 << 31;
}

/// Common types of [`Callback`]s, defined for convenience.
#[repr(u32)]
enum CommonCallbackType {
    EffectModifier = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesEffect
        | CallbackFlag::ReturnsNumber,
    MoveModifier = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsNumber,
    SourceMoveModifier = CallbackFlag::TakesUserMon
        | CallbackFlag::TakesSourceTargetMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsNumber,
    MonModifier = CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsNumber,
    EffectResult = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesEffect
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsVoid,
    MoveResult = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsVoid,
    SourceMoveResult = CallbackFlag::TakesUserMon
        | CallbackFlag::TakesSourceTargetMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsBoolean
        | CallbackFlag::ReturnsVoid,
    EffectVoid = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesEffect
        | CallbackFlag::ReturnsVoid,
    MoveVoid = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsVoid,
    MonVoid = CallbackFlag::TakesGeneralMon | CallbackFlag::ReturnsVoid,
}

/// A battle event that can trigger a [`Callback`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BattleEvent {
    AfterMoveSecondary,
    AfterSetStatus,
    AllySetStatus,
    BeforeMove,
    BasePower,
    DamagingHit,
    Duration,
    ModifyAtk,
    ModifyDamage,
    ModifyDef,
    ModifyMove,
    ModifySpA,
    ModifySpD,
    ModifySpe,
    Residual,
    SetStatus,
    Start,
    SwitchIn,
    UseMove,
    UseMoveMessage,
}

impl BattleEvent {
    /// Maps the event to the [`CallbackType`] flags.
    pub fn callback_type_flags(&self) -> u32 {
        match self {
            Self::AfterMoveSecondary => CommonCallbackType::MoveVoid as u32,
            Self::AfterSetStatus => CommonCallbackType::EffectVoid as u32,
            Self::AllySetStatus => CommonCallbackType::EffectResult as u32,
            Self::BasePower => CommonCallbackType::MoveModifier as u32,
            Self::BeforeMove => CommonCallbackType::SourceMoveResult as u32,
            Self::DamagingHit => CommonCallbackType::MoveVoid as u32,
            Self::Duration => CommonCallbackType::EffectModifier as u32,
            Self::ModifyAtk => CommonCallbackType::MonModifier as u32,
            Self::ModifyDamage => CommonCallbackType::SourceMoveModifier as u32,
            Self::ModifyDef => CommonCallbackType::MonModifier as u32,
            Self::ModifyMove => CommonCallbackType::MoveVoid as u32,
            Self::ModifySpA => CommonCallbackType::MonModifier as u32,
            Self::ModifySpD => CommonCallbackType::MonModifier as u32,
            Self::ModifySpe => CommonCallbackType::MonModifier as u32,
            Self::Residual => CommonCallbackType::EffectVoid as u32,
            Self::SetStatus => CommonCallbackType::EffectResult as u32,
            Self::Start => CommonCallbackType::EffectResult as u32,
            Self::SwitchIn => CommonCallbackType::MonVoid as u32,
            Self::UseMove => CommonCallbackType::MoveVoid as u32,
            Self::UseMoveMessage => CommonCallbackType::MoveVoid as u32,
        }
    }

    /// Checks if the event has the given [`CallbackType`] flag set.
    pub fn has_flag(&self, flag: u32) -> bool {
        self.callback_type_flags() & flag != 0
    }

    /// The name of the input variable by index.
    pub fn input_vars(&self) -> &[(&str, ValueType)] {
        match self {
            Self::BasePower => &[("power", ValueType::U32)],
            Self::DamagingHit => &[("damage", ValueType::U16)],
            Self::ModifyDamage => &[("damage", ValueType::U32)],
            Self::ModifySpe => &[("spe", ValueType::U16)],
            Self::SetStatus | Self::AllySetStatus => &[("status", ValueType::Effect)],
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

impl Display for BattleEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProgramWithPriority {
    pub program: Program,
    pub order: Option<u32>,
    pub priority: Option<i32>,
    pub sub_order: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
enum CallbackInput {
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
    pub on_after_move_secondary: Callback,
    pub on_after_set_status: Callback,
    pub on_ally_set_status: Callback,
    pub on_base_power: Callback,
    pub on_before_move: Callback,
    pub on_damaging_hit: Callback,
    pub on_duration: Callback,
    pub on_modify_atk: Callback,
    pub on_modify_damage: Callback,
    pub on_modify_def: Callback,
    pub on_modify_move: Callback,
    pub on_modify_spa: Callback,
    pub on_modify_spd: Callback,
    pub on_modify_spe: Callback,
    pub on_residual: Callback,
    pub on_set_status: Callback,
    pub on_start: Callback,
    pub on_switch_in: Callback,
    pub on_use_move: Callback,
    pub on_use_move_message: Callback,
}

impl Callbacks {
    pub fn event(&self, event: BattleEvent) -> Option<&Callback> {
        match event {
            BattleEvent::AfterMoveSecondary => Some(&self.on_after_move_secondary),
            BattleEvent::AfterSetStatus => Some(&self.on_after_set_status),
            BattleEvent::AllySetStatus => Some(&self.on_ally_set_status),
            BattleEvent::BasePower => Some(&self.on_base_power),
            BattleEvent::BeforeMove => Some(&self.on_before_move),
            BattleEvent::DamagingHit => Some(&self.on_damaging_hit),
            BattleEvent::Duration => Some(&self.on_duration),
            BattleEvent::ModifyAtk => Some(&self.on_modify_atk),
            BattleEvent::ModifyDamage => Some(&self.on_modify_damage),
            BattleEvent::ModifyDef => Some(&self.on_modify_def),
            BattleEvent::ModifyMove => Some(&self.on_modify_damage),
            BattleEvent::ModifySpA => Some(&self.on_modify_spa),
            BattleEvent::ModifySpD => Some(&self.on_modify_spd),
            BattleEvent::ModifySpe => Some(&self.on_modify_spe),
            BattleEvent::Residual => Some(&self.on_residual),
            BattleEvent::SetStatus => Some(&self.on_set_status),
            BattleEvent::Start => Some(&self.on_start),
            BattleEvent::SwitchIn => Some(&self.on_switch_in),
            BattleEvent::UseMove => Some(&self.on_use_move),
            BattleEvent::UseMoveMessage => Some(&self.on_use_move_message),
        }
    }
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
    /// Callbacks associated with the condition.
    pub callbacks: Callbacks,
}

/// An effect, whose callbacks are triggered in the context of an ongoing battle.
///
/// When an effect is active, its event callbacks are triggered throughout the course of a battle.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Effect {
    /// Event callbacks for the effect.
    pub callbacks: Callbacks,
}
