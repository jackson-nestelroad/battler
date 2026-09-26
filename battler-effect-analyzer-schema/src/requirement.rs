use battler_data::Stat;
use serde::{
    Deserialize,
    Serialize,
};

/// Conditions required for an effect to be viable (used for Phase 1 viability pruning).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EffectRequirement {
    /// Target must not already have a non-volatile status condition.
    TargetNotStatused,
    /// Target must not already have the specified volatile condition active.
    TargetNotCondition(String),
    /// Target side must have fewer than the maximum stacks of the specified side condition.
    SideConditionUnderMaxStacks(String, u32),
    /// The specified weather must not already be active on the field.
    WeatherNotActive(String),
    /// The specified terrain must not already be active on the field.
    TerrainNotActive(String),
    /// Target's stat boost must not already be at or beyond the specified cap (-6 or +6).
    StatNotCapped(Stat, i8),
    /// Target's health must be below 100% full HP.
    HealthBelowFull,
}
