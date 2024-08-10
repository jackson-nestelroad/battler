use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    battle::{
        core_battle_effects,
        Context,
    },
    common::Id,
    effect::fxlang,
};

/// Data for the field of a battle.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct FieldData {
    /// The default weather on the field.
    pub weather: Option<String>,
}

/// The battle field, which represents the shared environment that all Mons (from both sides) battle
/// on.
///
/// Effects can be applied to the entire field, which will affect all Mons.
pub struct Field {
    pub default_weather: Option<Id>,
    pub weather: Option<Id>,
    pub weather_state: fxlang::EffectState,
}

impl Field {
    /// Creates a new field.
    pub fn new(data: FieldData) -> Self {
        Self {
            default_weather: data.weather.map(|weather| Id::from(weather)),
            weather: None,
            weather_state: fxlang::EffectState::new(),
        }
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
