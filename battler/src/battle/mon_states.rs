use alloc::vec::Vec;

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
    if let Some(effective_types) = context
        .mon()
        .volatile_state
        .effect_cache
        .effective_types
        .clone()
    {
        return effective_types;
    }
    let effective_types = {
        let types = core_battle_effects::run_event_for_mon_expecting_types(
            context,
            fxlang::BattleEvent::ForceTypes,
            Vec::default(),
        );
        if !types.is_empty() {
            types
        } else {
            effective_types_before_forced_types(context)
        }
    };
    context
        .mon_mut()
        .volatile_state
        .effect_cache
        .effective_types = Some(effective_types.clone());
    effective_types
}

/// The effective types for the Mon, before forced types (e.g., Terastallization).
///
/// Non-empty. [`Type::None`] is returned when the Mon has no types
fn effective_types_before_forced_types(context: &mut MonContext) -> Vec<Type> {
    if let Some(effective_types_before_forced_types) = context
        .mon()
        .volatile_state
        .effect_cache
        .effective_types_before_forced_types
        .clone()
    {
        return effective_types_before_forced_types;
    }
    let effective_types_before_forced_types = {
        let types = core_battle_effects::run_event_for_mon_expecting_types(
            context,
            fxlang::BattleEvent::Types,
            context.mon().volatile_state.types.clone(),
        );
        if !types.is_empty() {
            types
        } else {
            Vec::from_iter([Type::None])
        }
    };
    context
        .mon_mut()
        .volatile_state
        .effect_cache
        .effective_types_before_forced_types = Some(effective_types_before_forced_types.clone());
    effective_types_before_forced_types
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
    if let Some(berry_eating_health) = context
        .mon()
        .volatile_state
        .effect_cache
        .berry_eating_health
    {
        return berry_eating_health;
    }
    let berry_eating_health = {
        let health = context.mon().max_hp / 4;
        core_battle_effects::run_event_for_mon_expecting_u16(
            context,
            fxlang::BattleEvent::BerryEatingHealth,
            health,
            fxlang::VariableInput::default(),
        )
    };
    context
        .mon_mut()
        .volatile_state
        .effect_cache
        .berry_eating_health = Some(berry_eating_health);
    berry_eating_health
}

/// Checks if the [`Mon`][`crate::battle::Mon`] can heal.
///
/// Does not necessarily check if the Mon needs to heal.
pub fn can_heal(context: &mut MonContext) -> bool {
    if let Some(can_heal) = context.mon().volatile_state.effect_cache.can_heal {
        return can_heal;
    }
    let can_heal = core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
        context,
        fxlang::BattleEvent::CanHeal,
        true,
    );
    context.mon_mut().volatile_state.effect_cache.can_heal = Some(can_heal);
    can_heal
}

/// Checks if the [`Mon`][`crate::battle::Mon`] is asleep.
pub fn is_asleep(context: &mut MonContext) -> bool {
    if let Some(is_asleep) = context.mon().volatile_state.effect_cache.is_asleep {
        return is_asleep;
    }
    let is_asleep = core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
        context,
        fxlang::BattleEvent::IsAsleep,
        false,
    );
    context.mon_mut().volatile_state.effect_cache.is_asleep = Some(is_asleep);
    is_asleep
}

/// Checks if the [`Mon`][`crate::battle::Mon`] is away from the field (e.g., immobilized by Sky
/// Drop).
pub fn is_away_from_field(context: &mut MonContext) -> bool {
    if let Some(is_away_from_field) = context.mon().volatile_state.effect_cache.is_away_from_field {
        return is_away_from_field;
    }
    let is_away_from_field = core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
        context,
        fxlang::BattleEvent::IsAwayFromField,
        false,
    );
    context
        .mon_mut()
        .volatile_state
        .effect_cache
        .is_away_from_field = Some(is_away_from_field);
    is_away_from_field
}

/// Checks if the [`Mon`][`crate::battle::Mon`] is behind a substitute.
pub fn is_behind_substitute(context: &mut MonContext) -> bool {
    if let Some(is_behind_substitute) = context
        .mon()
        .volatile_state
        .effect_cache
        .is_behind_substitute
    {
        return is_behind_substitute;
    }
    let is_behind_substitute = core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
        context,
        fxlang::BattleEvent::IsBehindSubstitute,
        false,
    );
    context
        .mon_mut()
        .volatile_state
        .effect_cache
        .is_behind_substitute = Some(is_behind_substitute);
    is_behind_substitute
}

/// Checks if the [`Mon`][`crate::battle::Mon`] is protected from making contact with other Mons.
pub fn is_contact_proof(context: &mut MonContext) -> bool {
    if let Some(is_contact_proof) = context.mon().volatile_state.effect_cache.is_contact_proof {
        return is_contact_proof;
    }
    let is_contact_proof = core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
        context,
        fxlang::BattleEvent::IsContactProof,
        false,
    );
    context
        .mon_mut()
        .volatile_state
        .effect_cache
        .is_contact_proof = Some(is_contact_proof);
    is_contact_proof
}

/// Checks if the [`Mon`][`crate::battle::Mon`] is grounded.
pub fn is_grounded(context: &mut MonContext) -> bool {
    if let Some(is_grounded) = context.mon().volatile_state.effect_cache.is_grounded {
        return is_grounded;
    }
    let is_grounded = core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
        context,
        fxlang::BattleEvent::IsGrounded,
        true,
    );
    context.mon_mut().volatile_state.effect_cache.is_grounded = Some(is_grounded);
    is_grounded
}

/// Checks if the [`Mon`][`crate::battle::Mon`] is immune to entry hazards.
pub fn is_immune_to_entry_hazards(context: &mut MonContext) -> bool {
    if let Some(is_immune_to_entry_hazards) = context
        .mon()
        .volatile_state
        .effect_cache
        .is_immune_to_entry_hazards
    {
        return is_immune_to_entry_hazards;
    }
    let is_immune_to_entry_hazards =
        core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
            context,
            fxlang::BattleEvent::IsImmuneToEntryHazards,
            false,
        );
    context
        .mon_mut()
        .volatile_state
        .effect_cache
        .is_immune_to_entry_hazards = Some(is_immune_to_entry_hazards);
    is_immune_to_entry_hazards
}

/// Checks if the [`Mon`][`crate::battle::Mon`] is soundproof.
pub fn is_soundproof(context: &mut MonContext) -> bool {
    if let Some(is_soundproof) = context.mon().volatile_state.effect_cache.is_soundproof {
        return is_soundproof;
    }
    let is_soundproof = core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
        context,
        fxlang::BattleEvent::IsSoundproof,
        false,
    );
    context.mon_mut().volatile_state.effect_cache.is_soundproof = Some(is_soundproof);
    is_soundproof
}

/// Checks if the [`Mon`][`crate::battle::Mon`] is semi-invulnerable.
pub fn is_semi_invulnerable(context: &mut MonContext) -> bool {
    if let Some(is_semi_invulnerable) = context
        .mon()
        .volatile_state
        .effect_cache
        .is_semi_invulnerable
    {
        return is_semi_invulnerable;
    }
    let is_semi_invulnerable = core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
        context,
        fxlang::BattleEvent::IsSemiInvulnerable,
        false,
    );
    context
        .mon_mut()
        .volatile_state
        .effect_cache
        .is_semi_invulnerable = Some(is_semi_invulnerable);
    is_semi_invulnerable
}

/// The effective weather for the [`Mon`][`crate::battle::Mon`].
///
/// Weather can be suppressed for the Mon by abilities and items.
pub fn effective_weather(context: &mut MonContext) -> Option<Id> {
    if let Some(effective_weather) = context
        .mon()
        .volatile_state
        .effect_cache
        .effective_weather
        .clone()
    {
        return effective_weather;
    }
    let weather = Field::effective_weather(context.as_battle_context_mut()).clone()?;
    let effective_weather = {
        if core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
            context,
            fxlang::BattleEvent::SuppressMonWeather,
            false,
        ) {
            None
        } else {
            Some(weather)
        }
    };
    context
        .mon_mut()
        .volatile_state
        .effect_cache
        .effective_weather = Some(effective_weather.clone());
    effective_weather
}

/// The effective terrain for the [`Mon`][`crate::battle::Mon`].
///
/// Terrain can be suppressed for the Mon by abilities and items.
pub fn effective_terrain(context: &mut MonContext) -> Option<Id> {
    if let Some(effective_terrain) = context
        .mon()
        .volatile_state
        .effect_cache
        .effective_terrain
        .clone()
    {
        return effective_terrain;
    }
    let terrain = Field::effective_terrain(context.as_battle_context_mut()).clone()?;
    let effective_terrain = {
        if !is_grounded(context) || is_semi_invulnerable(context) {
            None
        } else if core_battle_effects::run_event_for_mon_expecting_bool_quick_return(
            context,
            fxlang::BattleEvent::SuppressMonTerrain,
            false,
        ) {
            None
        } else {
            Some(terrain)
        }
    };
    context
        .mon_mut()
        .volatile_state
        .effect_cache
        .effective_terrain = Some(effective_terrain.clone());
    effective_terrain
}

fn check_ability_suppression(context: &mut MonContext) -> (Option<Id>, bool) {
    if let Some(effective_ability) = context
        .mon()
        .volatile_state
        .effect_cache
        .effective_ability
        .clone()
        && let Some(can_suppress_ability) = context
            .mon()
            .volatile_state
            .effect_cache
            .can_suppress_ability
    {
        return (effective_ability, can_suppress_ability);
    }
    let ability = context.mon().volatile_state.ability.id.clone();
    let (effective_ability, can_suppress_ability) = {
        let suppress_ability =
            core_battle_effects::run_event_for_mon_expecting_bool_quick_return_no_default(
                context,
                fxlang::BattleEvent::SuppressMonAbility,
            );
        match suppress_ability {
            Some(true) => (None, false),
            Some(false) => (Some(ability), false),
            None => (Some(ability), true),
        }
    };
    context
        .mon_mut()
        .volatile_state
        .effect_cache
        .can_suppress_ability = Some(can_suppress_ability);
    context
        .mon_mut()
        .volatile_state
        .effect_cache
        .effective_ability = Some(effective_ability.clone());
    (effective_ability, can_suppress_ability)
}

/// Checks if the [`Mon`][`crate::battle::Mon`]'s ability can be suppressed.
pub fn can_suppress_ability(context: &mut MonContext) -> bool {
    check_ability_suppression(context).1
}

/// The effective ability of the [`Mon`][`crate::battle::Mon`].
///
/// Abilities can be suppressed by other effects and abilities.
pub fn effective_ability(context: &mut MonContext) -> Option<Id> {
    check_ability_suppression(context).0
}

fn check_item_suppression(context: &mut MonContext) -> (Option<Id>, bool) {
    if let Some(effective_item) = context
        .mon()
        .volatile_state
        .effect_cache
        .effective_item
        .clone()
        && let Some(can_suppress_item) = context.mon().volatile_state.effect_cache.can_suppress_item
    {
        return (effective_item, can_suppress_item);
    }
    let item = match context.mon().item.clone() {
        Some(item) => item,
        None => return (None, false),
    };
    let (effective_item, can_suppress_item) = {
        let suppress_item =
            core_battle_effects::run_event_for_mon_expecting_bool_quick_return_no_default(
                context,
                fxlang::BattleEvent::SuppressMonItem,
            );
        match suppress_item {
            Some(true) => (None, false),
            Some(false) => (Some(item), false),
            None => (Some(item), true),
        }
    };
    context
        .mon_mut()
        .volatile_state
        .effect_cache
        .can_suppress_item = Some(can_suppress_item);
    context.mon_mut().volatile_state.effect_cache.effective_item = Some(effective_item.clone());
    (effective_item, can_suppress_item)
}

/// Checks if the [`Mon`][`crate::battle::Mon`]'s item can be suppressed.
pub fn can_suppress_item(context: &mut MonContext) -> bool {
    check_item_suppression(context).1
}

/// The effective item of the [`Mon`][`crate::battle::Mon`].
///
/// Items can be suppressed by other effects and abilities.
pub fn effective_item(context: &mut MonContext) -> Option<Id> {
    check_item_suppression(context).0
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
