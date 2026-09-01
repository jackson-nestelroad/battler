use alloc::string::String;

use hashbrown::{
    HashMap,
    HashSet,
};
use serde::{
    Deserialize,
    Serialize,
};

/// A position on the field.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct FieldPosition {
    pub side: usize,
    pub position: usize,
}

/// A reference to a Mon that is likely not active on the field.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct MonReference {
    pub player: String,
    pub name: String,
}

/// A Mon participating in the battle.
///
/// The Mon may be active or inactive. Active Mons can be seen on the field; inactive Mons can only
/// be referred to by name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct ActiveMonReference {
    #[serde(flatten)]
    pub position: FieldPosition,
    #[serde(flatten)]
    pub reference: MonReference,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export, rename = "UiMon"))]
pub enum Mon {
    Active(ActiveMonReference),
    Inactive(MonReference),
}

/// The target of a move.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub enum MoveTarget {
    #[serde(untagged)]
    Single(Mon),
    #[serde(untagged)]
    Spread(
        #[cfg_attr(feature = "typescript", ts(as = "std::collections::BTreeSet<Mon>"))]
        HashSet<Mon>,
    ),
}

/// A generic effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct Effect {
    pub effect_type: Option<String>,
    pub name: String,
}

/// A generic, domain-agnostic parsed value.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub enum LogValue {
    Boolean(bool),
    Number(i64),
    Fraction(u64, u64),
    String(String),
    Mon(Mon),
    MonList(#[cfg_attr(feature = "typescript", ts(as = "Vec<Mon>"))] Vec<Mon>),
}

/// A battle log entry specifically for the battle UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct UiLogEntry {
    /// The title of the log.
    pub title: String,

    /// The side targeted by the effect.
    pub side: Option<usize>,
    /// The slot targeted by the effect.
    pub slot: Option<usize>,
    /// The player targeted by the effect.
    pub player: Option<String>,
    /// The Mon targeted by the effect.
    pub target: Option<Mon>,
    /// The Mon that triggered the effect.
    pub source: Option<Mon>,
    /// The effect that activated.
    pub effect: Option<Effect>,
    /// The effect that triggered the effect.
    pub source_effect: Option<Effect>,

    /// Typed key-value data.
    #[cfg_attr(
        feature = "typescript",
        ts(as = "std::collections::BTreeMap<String, LogValue>")
    )]
    pub values: HashMap<String, LogValue>,
}

pub trait IntoLogValue {
    fn into_log_value(self) -> crate::ui::LogValue;
}
impl IntoLogValue for bool {
    fn into_log_value(self) -> crate::ui::LogValue {
        crate::ui::LogValue::Boolean(self)
    }
}
impl IntoLogValue for i64 {
    fn into_log_value(self) -> crate::ui::LogValue {
        crate::ui::LogValue::Number(self)
    }
}
impl IntoLogValue for u64 {
    fn into_log_value(self) -> crate::ui::LogValue {
        crate::ui::LogValue::Number(self as i64)
    }
}
impl IntoLogValue for i32 {
    fn into_log_value(self) -> crate::ui::LogValue {
        crate::ui::LogValue::Number(self as i64)
    }
}
impl IntoLogValue for u32 {
    fn into_log_value(self) -> crate::ui::LogValue {
        crate::ui::LogValue::Number(self as i64)
    }
}
impl IntoLogValue for String {
    fn into_log_value(self) -> crate::ui::LogValue {
        crate::ui::LogValue::String(self)
    }
}
impl IntoLogValue for &str {
    fn into_log_value(self) -> crate::ui::LogValue {
        crate::ui::LogValue::String(self.to_owned())
    }
}
impl IntoLogValue for crate::ui::Mon {
    fn into_log_value(self) -> crate::ui::LogValue {
        crate::ui::LogValue::Mon(self)
    }
}
impl IntoLogValue for (u64, u64) {
    fn into_log_value(self) -> crate::ui::LogValue {
        crate::ui::LogValue::Fraction(self.0, self.1)
    }
}

#[macro_export]
macro_rules! ui_log {
    (
        title = $title:expr
        $(, side = $side:expr)?
        $(, slot = $slot:expr)?
        $(, player = $player:expr)?
        $(, target = $target:expr)?
        $(, source = $source:expr)?
        $(, effect = $effect:expr)?
        $(, source_effect = $source_effect:expr)?
        $(, values = { $($k:expr => $v:expr),* $(,)? })?
    ) => {{
        #[allow(unused_mut)]
        let mut values = hashbrown::HashMap::<String, crate::ui::LogValue>::new();
        $($(
            values.insert($k.to_owned(), $crate::ui::IntoLogValue::into_log_value($v));
        )*)?
        crate::ui::UiLogEntry {
            title: $title.to_owned(),
            side: ui_log!(@opt $($side)?),
            slot: ui_log!(@opt $($slot)?),
            player: ui_log!(@opt $($player)?),
            target: ui_log!(@opt $($target)?),
            source: ui_log!(@opt $($source)?),
            effect: ui_log!(@opt $($effect)?),
            source_effect: ui_log!(@opt $($source_effect)?),
            values,
        }
    }};
    (@opt) => { None };
    (@opt $val:expr) => { Some($val.clone().into()) };
}
