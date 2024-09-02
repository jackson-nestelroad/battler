use crate::{
    battle::{
        core_battle_effects,
        EffectContext,
    },
    effect::fxlang,
};

/// Checks if weather includes rain.
pub fn is_raining(context: &mut EffectContext) -> bool {
    core_battle_effects::run_effect_event_expecting_bool(context, fxlang::BattleEvent::IsRaining)
        .unwrap_or(false)
}

/// Checks if weather includes snow.
pub fn is_snowing(context: &mut EffectContext) -> bool {
    core_battle_effects::run_effect_event_expecting_bool(context, fxlang::BattleEvent::IsSnowing)
        .unwrap_or(false)
}

/// Checks if weather includes the sun.
pub fn is_sunny(context: &mut EffectContext) -> bool {
    core_battle_effects::run_effect_event_expecting_bool(context, fxlang::BattleEvent::IsSunny)
        .unwrap_or(false)
}
