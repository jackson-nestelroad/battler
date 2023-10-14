use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

use crate::common::Identifiable;

/// The type of an effect.
#[derive(Debug, PartialEq, SerializeLabeledStringEnum, DeserializeLabeledStringEnum)]
pub enum EffectType {
    #[string = "Mon"]
    #[alias = "Species"]
    Species,
}

/// An effect that can be applied to a battle.
///
/// All entities are represented as an Effect so that they can share logic for hooking into event
/// callbacks.
pub trait Effect: Identifiable {
    fn effect_type() -> EffectType;
}
