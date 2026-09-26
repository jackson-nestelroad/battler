use battler_data::Id;
use serde::{
    Deserialize,
    Serialize,
};

use crate::polarity::EffectPolarity;

/// The scope of a battle condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConditionScope {
    /// Volatile condition on an individual Mon (e.g., Confusion, Taunt, Substitute).
    Volatile,
    /// Condition affecting an entire side of the field (e.g., Stealth Rock, Spikes, Tailwind).
    SideCondition,
    /// Condition affecting the entire field (e.g., Trick Room, Magic Room).
    FieldCondition,
    /// Pseudo-weather affecting the entire field.
    PseudoWeather,
    /// Active weather (e.g., Rain, Sun, Sandstorm, Snow).
    Weather,
    /// Active terrain (e.g., Electric, Grassy, Misty, Psychic).
    Terrain,
}

/// Precompiled semantic manifest for a condition.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConditionManifest {
    /// Unique ID of the condition.
    pub id: Id,
    /// Scope of the condition.
    pub scope: ConditionScope,
    /// Semantic polarity of the condition.
    pub polarity: EffectPolarity,
    /// Maximum number of active stacks allowed (e.g., 1 for Tailwind, 3 for Spikes).
    #[serde(default = "default_max_stacks")]
    pub max_stacks: u32,
    /// Standard duration in turns, if fixed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<u32>,
}

fn default_max_stacks() -> u32 {
    1
}

impl ConditionManifest {
    /// Creates a new condition manifest.
    pub fn new(
        id: Id,
        scope: ConditionScope,
        polarity: EffectPolarity,
        max_stacks: u32,
        duration: Option<u32>,
    ) -> Self {
        Self {
            id,
            scope,
            polarity,
            max_stacks,
            duration,
        }
    }
}
