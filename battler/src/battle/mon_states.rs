use battler_data::{
    Id,
    MoveFlag,
    Type,
};

use crate::{
    battle::{
        ActiveMoveContext,
        Field,
        MonContext,
        core_battle_effects,
    },
    effect::fxlang,
};

/// The effective types for the Mon.
///
/// Non-empty. [`Type::None`] is returned when the Mon has no types
pub fn effective_types(context: &mut MonContext) -> Vec<Type> {
    let types = core_battle_effects::run_event_for_mon_expecting_types(
        context,
        fxlang::BattleEvent::ForceTypes,
        Vec::default(),
    );
    if !types.is_empty() {
        return types;
    }
    effective_types_before_forced_types(context)
}

/// The effective types for the Mon, before forced types (e.g., Terastallization).
///
/// Non-empty. [`Type::None`] is returned when the Mon has no types
fn effective_types_before_forced_types(context: &mut MonContext) -> Vec<Type> {
    let types = core_battle_effects::run_event_for_mon_expecting_types(
        context,
        fxlang::BattleEvent::Types,
        context.mon().volatile_state.types.clone(),
    );
    if !types.is_empty() {
        return types;
    }
    Vec::from_iter([Type::None])
}

/// Checks if the Mon has the given type.
pub fn has_type(context: &mut MonContext, typ: Type) -> bool {
    let types = effective_types(context);
    types.contains(&typ)
}

/// Checks if the Mon has the given type, before forced types (e.g., Terastallization).
pub fn has_type_before_forced_types(context: &mut MonContext, typ: Type) -> bool {
    let types = effective_types_before_forced_types(context);
    types.contains(&typ)
}

/// The health at which the [`Mon`][`crate::battle::Mon`] eats berries.
pub fn berry_eating_health(context: &mut MonContext) -> u16 {
    let health = context.mon().max_hp / 4;
    core_battle_effects::run_event_for_mon_expecting_u16(
        context,
        fxlang::BattleEvent::BerryEatingHealth,
        health,
    )
}

/// Checks if the [`Mon`][`crate::battle::Mon`] can heal.
///
/// Does not necessarily check if the Mon needs to heal.
pub fn can_heal(context: &mut MonContext) -> bool {
    core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
        context,
        fxlang::BattleEvent::CanHeal,
        true,
    )
}

/// Checks if the [`Mon`][`crate::battle::Mon`] is asleep.
pub fn is_asleep(context: &mut MonContext) -> bool {
    core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
        context,
        fxlang::BattleEvent::IsAsleep,
        false,
    )
}

/// Checks if the [`Mon`][`crate::battle::Mon`] is away from the field (e.g., immobilized by Sky
/// Drop).
pub fn is_away_from_field(context: &mut MonContext) -> bool {
    core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
        context,
        fxlang::BattleEvent::IsAwayFromField,
        false,
    )
}

/// Checks if the [`Mon`][`crate::battle::Mon`] is behind a substitute.
pub fn is_behind_substitute(context: &mut MonContext) -> bool {
    core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
        context,
        fxlang::BattleEvent::IsBehindSubstitute,
        false,
    )
}

/// Checks if the [`Mon`][`crate::battle::Mon`] is protected from making contact with other Mons.
pub fn is_contact_proof(context: &mut MonContext) -> bool {
    core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
        context,
        fxlang::BattleEvent::IsContactProof,
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

/// Checks if the [`Mon`][`crate::battle::Mon`] is semi-invulnerable.
pub fn is_semi_invulnerable(context: &mut MonContext) -> bool {
    core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
        context,
        fxlang::BattleEvent::IsSemiInvulnerable,
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

/// The effective terrain for the [`Mon`][`crate::battle::Mon`].
///
/// Terrain can be suppressed for the Mon by abilities and items.
pub fn effective_terrain(context: &mut MonContext) -> Option<Id> {
    if !is_grounded(context) || is_semi_invulnerable(context) {
        return None;
    }
    if core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
        context,
        fxlang::BattleEvent::SuppressMonTerrain,
        false,
    ) {
        return None;
    }
    Field::effective_terrain(context.as_battle_context_mut())
}

/// The effective ability of the [`Mon`][`crate::battle::Mon`].
///
/// Abilities can be suppressed by other effects and abilities.
pub fn effective_ability(context: &mut MonContext) -> Option<Id> {
    if core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
        context,
        fxlang::BattleEvent::SuppressMonAbility,
        false,
    ) {
        return None;
    }
    Some(context.mon().volatile_state.ability.id.clone())
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
    context.mon().item.clone()
}

/// Checks if the [`Move`][`crate::moves::Move`] makes contact with its targets.
pub fn move_makes_contact(context: &mut ActiveMoveContext) -> bool {
    if !context
        .active_move()
        .data
        .flags
        .contains(&MoveFlag::Contact)
    {
        return false;
    }

    // Check if the attacker is contact-proof.
    return !is_contact_proof(context.as_mon_context_mut());
}
