use crate::{
    battle::{
        core_battle_effects,
        Field,
        MonContext,
    },
    common::Id,
    effect::fxlang,
};

/// Checks if the [`Mon`][`crate::battle::Mon`] is asleep.
pub fn is_asleep(context: &mut MonContext) -> bool {
    core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
        context,
        fxlang::BattleEvent::IsAsleep,
        false,
    )
}

/// Checks if the [`Mon`][`crate::battle::Mon`] is grounded.
pub fn is_grounded(context: &mut MonContext) -> bool {
    core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
        context,
        fxlang::BattleEvent::IsGrounded,
        true,
    )
}

/// Checks if the [`Mon`][`crate::battle::Mon`] is immune to entry hazards.
pub fn is_immune_to_entry_hazards(context: &mut MonContext) -> bool {
    core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
        context,
        fxlang::BattleEvent::IsImmuneToEntryHazards,
        false,
    )
}

/// Checks if the [`Mon`][`crate::battle::Mon`] is soundproof.
pub fn is_soundproof(context: &mut MonContext) -> bool {
    core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
        context,
        fxlang::BattleEvent::IsSoundproof,
        false,
    )
}

/// The effective weather for the [`Mon`][`crate::battle::Mon`].
///
/// Weather can be suppressed for the Mon by abilities and items.
pub fn effective_weather(context: &mut MonContext) -> Option<Id> {
    if core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
        context,
        fxlang::BattleEvent::SuppressMonWeather,
        false,
    ) {
        return None;
    }
    Field::effective_weather(context.as_battle_context_mut())
}

/// The effective ability of the [`Mon`][`crate::battle::Mon`].
///
/// Abilities can be suppressed by other effects and abilities.
pub fn effective_ability(context: &mut MonContext) -> Option<Id> {
    // TODO: SuppressAbility event.
    //  - First, check if ability is breakable (flag).
    //  - If so, run the event.
    //      - Mold Breaker suppresses during move execution of the ability holder.
    //          - BeforeMove => suppress
    //          - AfterMove => unsuppress
    //      - Ability Shield unsuppresses (higher priority than Mold Breaker).
    Some(context.mon().ability.id.clone())
}

/// The effective item of the [`Mon`][`crate::battle::Mon`].
///
/// Items can be suppressed by other effects and abilities.
pub fn effective_item(context: &mut MonContext) -> Option<Id> {
    if core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
        context,
        fxlang::BattleEvent::SuppressMonItem,
        false,
    ) {
        return None;
    }
    context.mon().item.as_ref().map(|item| item.id.clone())
}
