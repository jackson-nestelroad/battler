use crate::{
    battle::{
        EffectContext,
        core_battle_effects,
    },
    effect::fxlang,
};

/// Checks if weather includes rain.
pub fn is_raining(context: &mut EffectContext) -> bool {
    core_battle_effects::run_effect_event::<_, bool>(context, fxlang::BattleEvent::IsRaining)
}

/// Checks if weather includes snow.
pub fn is_snowing(context: &mut EffectContext) -> bool {
    core_battle_effects::run_effect_event::<_, bool>(context, fxlang::BattleEvent::IsSnowing)
}

/// Checks if weather includes the sun.
pub fn is_sunny(context: &mut EffectContext) -> bool {
    core_battle_effects::run_effect_event::<_, bool>(context, fxlang::BattleEvent::IsSunny)
}
