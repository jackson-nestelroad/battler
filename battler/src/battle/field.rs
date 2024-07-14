use crate::{
    battle::{
        core_battle_effects,
        Context,
    },
    common::Id,
    effect::fxlang,
};

/// The battle field, which represents the shared environment that all Mons (from both sides) battle
/// on.
///
/// Effects can be applied to the entire field, which will affect all Mons.
pub struct Field {
    pub weather: Option<Id>,
    pub weather_state: fxlang::EffectState,
}

impl Field {
    /// Creates a new field.
    pub fn new() -> Self {
        Self {
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
