use std::{
    fmt,
    fmt::Display,
};

use serde::{
    Deserialize,
    Serialize,
};

use crate::effect::fxlang::ValueType;

/// Flags used to indicate the input and output of a [`Callback`].
#[allow(non_snake_case)]
#[allow(non_upper_case_globals)]
pub mod CallbackFlag {
    pub const TakesGeneralMon: u32 = 1 << 1;
    pub const TakesTargetMon: u32 = 1 << 2;
    pub const TakesSourceMon: u32 = 1 << 3;
    pub const TakesEffect: u32 = 1 << 4;
    pub const TakesActiveMove: u32 = 1 << 5;

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
    MoveResult = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsBoolean,
    EffectVoid = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesEffect
        | CallbackFlag::ReturnsVoid,
    MoveVoid = CallbackFlag::TakesTargetMon
        | CallbackFlag::TakesSourceMon
        | CallbackFlag::TakesActiveMove
        | CallbackFlag::ReturnsVoid,
}

/// A battle event that can trigger a [`Callback`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BattleEvent {
    BasePower,
    Duration,
    UseMove,
    UseMoveMessage,
}

impl BattleEvent {
    /// Maps the event to the [`CallbackType`] flags.
    pub fn callback_type_flags(&self) -> u32 {
        match self {
            Self::BasePower => CommonCallbackType::MoveModifier as u32,
            Self::Duration => CommonCallbackType::EffectModifier as u32,
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

/// A single callback, to be called when applying an effect on some triggered event.
pub type Callback = Option<Program>;

/// A collection of callbacks for an effect.
///
/// All possible callbacks for an effect should be defined here.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Callbacks {
    pub on_base_power: Callback,
    pub on_duration: Callback,
    pub on_use_move: Callback,
    pub on_use_move_message: Callback,
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
    pub duration: u8,
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
