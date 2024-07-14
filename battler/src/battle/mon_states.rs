use crate::{
    battle::{
        core_battle_effects,
        MonContext,
    },
    effect::fxlang,
};

/// Checks if the [`Mon`][`crate::battle::Mon`] is asleep.
pub fn is_asleep(context: &mut MonContext) -> bool {
    core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
        context,
        fxlang::BattleEvent::IsAsleep,
    )
}
