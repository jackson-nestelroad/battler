use ahash::HashMap;
use battler_data::Id;
use serde::{
    Deserialize,
    Serialize,
};
use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

use crate::{
    battle::{
        Context,
        core_battle_effects,
    },
    effect::fxlang,
};

/// The environment of the battle field.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    SerializeLabeledStringEnum,
    DeserializeLabeledStringEnum,
)]
pub enum FieldEnvironment {
    #[default]
    #[string = "Normal"]
    Normal,
    #[string = "Cave"]
    Cave,
    #[string = "Sand"]
    Sand,
    #[string = "Water"]
    Water,
    #[string = "Ice"]
    Ice,
    #[string = "Sky"]
    Sky,
    #[string = "Grass"]
    Grass,
    #[string = "Volcano"]
    Volcano,
}

/// The time of day of the battle field.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    SerializeLabeledStringEnum,
    DeserializeLabeledStringEnum,
)]
pub enum TimeOfDay {
    #[default]
    #[string = "Day"]
    Day,
    #[string = "Morning"]
    Morning,
    #[string = "Evening"]
    Evening,
    #[string = "Night"]
    Night,
}

/// Data for the field of a battle.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FieldData {
    /// The default weather on the field.
    pub weather: Option<String>,
    /// The default terrain on the field.
    pub terrain: Option<String>,
    /// The environment of the field.
    #[serde(default)]
    pub environment: FieldEnvironment,
    /// The time of day of the field.
    #[serde(default)]
    pub time: TimeOfDay,
}

/// The battle field, which represents the shared environment that all Mons (from both sides) battle
/// on.
///
/// Effects can be applied to the entire field, which will affect all Mons.
pub struct Field {
    pub default_weather: Option<Id>,
    pub default_terrain: Option<Id>,
    pub environment: FieldEnvironment,
    pub time: TimeOfDay,
    pub weather: Option<Id>,
    pub weather_state: fxlang::EffectState,
    pub terrain: Option<Id>,
    pub terrain_state: fxlang::EffectState,
    pub pseudo_weathers: HashMap<Id, fxlang::EffectState>,
}

impl Field {
    /// Creates a new field.
    pub fn new(data: FieldData) -> Self {
        Self {
            default_weather: data.weather.map(|weather| Id::from(weather)),
            default_terrain: data.terrain.map(|terrain| Id::from(terrain)),
            environment: data.environment,
            time: data.time,
            weather: None,
            weather_state: fxlang::EffectState::new(),
            terrain: None,
            terrain_state: fxlang::EffectState::new(),
            pseudo_weathers: HashMap::default(),
        }
    }

    /// The effective terrain for the field.
    ///
    /// Terrain can be suppressed for the entire field by abilities.
    pub fn effective_terrain(context: &mut Context) -> Option<Id> {
        if core_battle_effects::run_event_for_battle_expecting_bool_quick_return(
            context,
            fxlang::BattleEvent::SuppressFieldTerrain,
        ) {
            return None;
        }
        context.battle().field.terrain.clone()
    }

    /// The effective weather for the field.
    ///
    /// Weather can be suppressed for the entire field by abilities.
    pub fn effective_weather(context: &mut Context) -> Option<Id> {
        if core_battle_effects::run_event_for_battle_expecting_bool_quick_return(
            context,
            fxlang::BattleEvent::SuppressFieldWeather,
        ) {
            return None;
        }
        context.battle().field.weather.clone()
    }
}
