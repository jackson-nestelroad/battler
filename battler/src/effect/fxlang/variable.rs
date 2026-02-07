use alloc::{
    borrow::ToOwned,
    format,
    string::{
        String,
        ToString,
    },
    vec::Vec,
};

use anyhow::{
    Error,
    Result,
};
use battler_data::{
    Fraction,
    Identifiable,
    ZPower,
};
use zone_alloc::{
    BorrowError,
    ElementRef,
    ElementRefMut,
    KeyedRegistry,
};

use crate::{
    WrapOptionError,
    battle::{
        CoreBattle,
        Field,
        Mon,
        MonExitType,
        Player,
        mon_states,
        weather_states,
    },
    common::UnsafelyDetachBorrowMut,
    effect::{
        ActiveMoveEffectStateConnector,
        EffectHandle,
        MonStatusEffectStateConnector,
        fxlang::{
            EffectStateConnector,
            EvaluationContext,
            MaybeReferenceValueForOperation,
            Value,
            ValueRef,
            ValueRefMut,
            ValueRefToStoredValue,
            ValueType,
        },
    },
    general_error,
    integer_overflow_error,
};

/// A registry of variables for an fxlang program evaluation.
pub struct VariableRegistry {
    vars: KeyedRegistry<String, Value>,
}

impl VariableRegistry {
    pub fn new() -> Self {
        Self {
            vars: KeyedRegistry::new(),
        }
    }

    pub fn get(&self, var: &str) -> Result<Option<ElementRef<'_, Value>>> {
        match self.vars.get(var) {
            Ok(val) => Ok(Some(val)),
            Err(BorrowError::OutOfBounds) => Ok(None),
            Err(_) => Err(general_error(format!("failed to borrow variable ${var}"))),
        }
    }

    pub fn get_mut(&self, var: &str) -> Result<Option<ElementRefMut<'_, Value>>> {
        match self.vars.get_mut(var) {
            Ok(val) => Ok(Some(val)),
            Err(BorrowError::OutOfBounds) => Ok(None),
            Err(_) => Err(general_error(format!("failed to borrow variable ${var}"))),
        }
    }

    pub fn set(&self, var: &str, value: Value) -> Result<()> {
        match self.vars.get_mut(var) {
            Ok(mut var) => {
                *var = value;
                Ok(())
            }
            Err(BorrowError::OutOfBounds) => {
                self.vars.register(var.to_owned(), value);
                Ok(())
            }
            Err(_) => Err(general_error(format!(
                "failed to mutably borrow variable ${var}"
            ))),
        }
    }
}

/// An fxlang variable.
///
/// Acts as a wrapper for an immutable access of a variable that can be consumed at some later time.
pub struct Variable<'eval, 'program> {
    stored: Option<ElementRef<'eval, Value>>,
    member_access: Vec<&'program str>,
}

impl<'eval, 'program> Variable<'eval, 'program>
where
    'program: 'eval,
{
    pub fn new(
        stored: Option<ElementRef<'eval, Value>>,
        member_access: Vec<&'program str>,
    ) -> Self {
        Self {
            stored,
            member_access,
        }
    }

    fn bad_member_access(member: &str, value_type: ValueType) -> Error {
        general_error(format!("value of type {value_type} has no member {member}"))
    }

    fn get_ref<'var>(&'var self, context: &'eval mut EvaluationContext) -> Result<ValueRef<'var>> {
        let mut value = match &self.stored {
            Some(stored) => ValueRef::from(stored),
            None => ValueRef::Undefined,
        };
        for member in &self.member_access {
            // SAFETY: For changing the lifetime of context: the mutable reference inside of
            // `value_ref` is only mutated at the very end of this method. Thus, this entire for
            // loop is actually immutable. Furthermore, since we only hold one
            // `value_ref` at a time, so there is no way to hold multiple mutable borrow
            // of values inside `context` at the same time.
            //
            // We can imagine that `value_ref` really does correctly mutably borrow `context`. If
            // the next iteration somehow also needs to borrow from `context`, the previous
            // `value_ref` value (i.e., the mutable borrow inside of it) is dropped.
            let value_type = value.value_type();

            match *member {
                "is_boolean" => {
                    value = ValueRef::Boolean(value.is_boolean());
                }
                "is_defined" => {
                    value = ValueRef::Boolean(!value.is_undefined());
                }
                "is_empty" => {
                    value = ValueRef::Boolean(value.is_empty());
                }
                "is_undefined" => {
                    value = ValueRef::Boolean(value.is_undefined());
                }
                "length" => {
                    value = match value.len() {
                        Some(len) => ValueRef::UFraction(
                            TryInto::<u64>::try_into(len)
                                .map_err(integer_overflow_error)?
                                .into(),
                        ),
                        None => ValueRef::Undefined,
                    }
                }
                "to_string" => {
                    value = ValueRef::TempString(
                        MaybeReferenceValueForOperation::from(value).for_formatted_string()?,
                    )
                }
                _ => {
                    let mut effect_matched = false;
                    if let Some(effect_handle) = value.effect_handle() {
                        effect_matched = true;
                        let context = unsafe { context.unsafely_detach_borrow_mut() };
                        value = match *member {
                            "accuracy" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| ValueRef::Accuracy(mov.data.accuracy))
                            .unwrap_or(ValueRef::Undefined),
                            "base_power" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| ValueRef::UFraction(mov.data.base_power.into()))
                            .unwrap_or(ValueRef::Undefined),
                            "callable" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| ValueRef::Boolean(mov.callable()))
                            .unwrap_or(ValueRef::Undefined),
                            "category" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| ValueRef::MoveCategory(mov.data.category))
                            .unwrap_or(ValueRef::Undefined),
                            "condition" => ValueRef::TempEffect(
                                effect_handle
                                    .condition_handle(context.battle_context())?
                                    .wrap_expectation("effect has no associated condition")?,
                            ),
                            "damage" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| match mov.data.damage {
                                Some(damage) => ValueRef::UFraction(damage.into()),
                                None => ValueRef::Undefined,
                            })
                            .unwrap_or(ValueRef::Undefined),
                            "drain_percent" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| {
                                ValueRef::UFraction(
                                    mov.data
                                        .drain_percent
                                        .unwrap_or(Fraction::from(0u16))
                                        .convert(),
                                )
                            })
                            .unwrap_or(ValueRef::Undefined),
                            "id" => ValueRef::TempString(
                                CoreBattle::get_effect_by_handle(
                                    context.battle_context(),
                                    &effect_handle,
                                )?
                                .id()
                                .as_ref()
                                .to_owned(),
                            ),
                            "is_ability" => ValueRef::Boolean(effect_handle.is_ability()),
                            "is_item" => ValueRef::Boolean(effect_handle.is_item()),
                            "is_move" => ValueRef::Boolean(effect_handle.is_active_move()),
                            "is_move_secondary" => {
                                ValueRef::Boolean(effect_handle.is_active_move_secondary())
                            }
                            "is_raining" => ValueRef::Boolean(weather_states::is_raining(
                                context.effect_context_for_handle(&effect_handle)?.as_mut(),
                            )),
                            "is_snowing" => ValueRef::Boolean(weather_states::is_snowing(
                                context.effect_context_for_handle(&effect_handle)?.as_mut(),
                            )),
                            "is_sunny" => ValueRef::Boolean(weather_states::is_sunny(
                                context.effect_context_for_handle(&effect_handle)?.as_mut(),
                            )),
                            "max_move_base_power" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| {
                                mov.data
                                    .max_move
                                    .as_ref()
                                    .map(|max_move| max_move.base_power)
                                    .map(|val| ValueRef::UFraction(val.into()))
                                    .unwrap_or(ValueRef::Undefined)
                            })
                            .unwrap_or(ValueRef::Undefined),
                            "move_target" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| ValueRef::MoveTarget(mov.data.target))
                            .unwrap_or(ValueRef::Undefined),
                            "multiaccuracy" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| ValueRef::Boolean(mov.data.multiaccuracy))
                            .unwrap_or(ValueRef::Undefined),
                            "multihit" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| {
                                mov.data
                                    .multihit
                                    .map(|val| ValueRef::MultihitType(val))
                                    .unwrap_or(ValueRef::Undefined)
                            })
                            .unwrap_or(ValueRef::Undefined),
                            "name" => ValueRef::TempString(
                                CoreBattle::get_effect_by_handle(
                                    context.battle_context(),
                                    &effect_handle,
                                )?
                                .name()
                                .to_owned(),
                            ),
                            "ohko" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| ValueRef::Boolean(mov.data.ohko_type.is_some()))
                            .unwrap_or(ValueRef::Undefined),
                            "priority" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| ValueRef::Fraction(mov.data.priority.into()))
                            .unwrap_or(ValueRef::Undefined),
                            "recoil_percent" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| {
                                ValueRef::UFraction(
                                    mov.data
                                        .recoil
                                        .as_ref()
                                        .map(|recoil| recoil.percent)
                                        .unwrap_or(Fraction::from(0u16))
                                        .convert(),
                                )
                            })
                            .unwrap_or(ValueRef::Undefined),
                            "target" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| ValueRef::MoveTarget(mov.data.target))
                            .unwrap_or(ValueRef::Undefined),
                            "type" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| ValueRef::Type(mov.data.primary_type))
                            .unwrap_or(ValueRef::Undefined),
                            "typeless" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| ValueRef::Boolean(mov.data.typeless))
                            .unwrap_or(ValueRef::Undefined),
                            "z_move_base_power" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| {
                                mov.data
                                    .z_move
                                    .as_ref()
                                    .map(|z_move| z_move.base_power)
                                    .map(|val| ValueRef::UFraction(val.into()))
                                    .unwrap_or(ValueRef::Undefined)
                            })
                            .unwrap_or(ValueRef::Undefined),
                            "z_power_boosts" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| {
                                mov.data
                                    .z_move
                                    .as_ref()
                                    .map(|z_move| {
                                        z_move.z_power.as_ref().and_then(|z_power| match z_power {
                                            ZPower::Boost(boosts) => Some(boosts),
                                            _ => None,
                                        })
                                    })
                                    .flatten()
                                    .map(|val| ValueRef::TempBoostTable(val.clone()))
                                    .unwrap_or(ValueRef::Undefined)
                            })
                            .unwrap_or(ValueRef::Undefined),
                            "z_power_effect" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| {
                                mov.data
                                    .z_move
                                    .as_ref()
                                    .map(|z_move| {
                                        z_move.z_power.as_ref().and_then(|z_power| match z_power {
                                            ZPower::Effect(effect) => Some(effect),
                                            _ => None,
                                        })
                                    })
                                    .flatten()
                                    .map(|val| ValueRef::TempString(val.clone()))
                                    .unwrap_or(ValueRef::Undefined)
                            })
                            .unwrap_or(ValueRef::Undefined),
                            _ => {
                                if effect_handle.is_active_move() {
                                    // Allow active move to fall through.
                                    effect_matched = false;
                                    value
                                } else {
                                    return Err(Self::bad_member_access(member, value_type));
                                }
                            }
                        }
                    }

                    if effect_matched {
                        continue;
                    }

                    if let Some(active_move_handle) = value.active_move_handle() {
                        let context = unsafe { context.unsafely_detach_borrow_mut() };
                        value = match *member {
                            "effect_state" => ValueRef::EffectState(
                                ActiveMoveEffectStateConnector::new(active_move_handle)
                                    .make_dynamic(),
                            ),
                            "external" => {
                                ValueRef::Boolean(context.active_move(active_move_handle)?.external)
                            }
                            "hit" => ValueRef::UFraction(
                                context.active_move(active_move_handle)?.hit.into(),
                            ),
                            "hit_effect" => context
                                .active_move(active_move_handle)?
                                .data
                                .hit_effect
                                .as_ref()
                                .map(ValueRef::HitEffect)
                                .unwrap_or(ValueRef::Undefined),
                            "ignore_all_secondary_effects" => ValueRef::Boolean(
                                context
                                    .active_move(active_move_handle)?
                                    .ignore_all_secondary_effects,
                            ),
                            "secondary_effects" => ValueRef::TempList(
                                context
                                    .active_move(active_move_handle)?
                                    .data
                                    .secondary_effects
                                    .iter()
                                    .map(|val| {
                                        ValueRefToStoredValue::new(
                                            self.stored.clone(),
                                            ValueRef::SecondaryHitEffect(val),
                                        )
                                    })
                                    .collect(),
                            ),
                            "source" | "user" => {
                                match context.active_move(active_move_handle)?.used_by {
                                    Some(mon) => ValueRef::Mon(mon),
                                    None => ValueRef::Undefined,
                                }
                            }
                            "spread_hit" => ValueRef::Boolean(
                                context.active_move(active_move_handle)?.spread_hit,
                            ),
                            "total_damage" => ValueRef::UFraction(
                                context.active_move(active_move_handle)?.total_damage.into(),
                            ),
                            "upgraded" => ValueRef::Boolean(
                                context.active_move(active_move_handle)?.upgraded.is_some(),
                            ),
                            "upgraded_base_move" => context
                                .active_move(active_move_handle)?
                                .upgraded
                                .as_ref()
                                .map(|upgraded| upgraded.base_move())
                                .flatten()
                                .map(|val| ValueRef::TempString(val.to_string()))
                                .unwrap_or(ValueRef::Undefined),
                            "user_effect" => context
                                .active_move(active_move_handle)?
                                .data
                                .user_effect
                                .as_ref()
                                .map(ValueRef::HitEffect)
                                .unwrap_or(ValueRef::Undefined),
                            "user_effect_chance" => context
                                .active_move(active_move_handle)?
                                .data
                                .user_effect_chance
                                .map(|val| ValueRef::UFraction(val.convert()))
                                .unwrap_or(ValueRef::Undefined),
                            "override_offensive_stat" => {
                                match context
                                    .active_move(active_move_handle)?
                                    .data
                                    .override_offensive_stat
                                {
                                    Some(stat) => ValueRef::Stat(stat),
                                    None => ValueRef::Undefined,
                                }
                            }
                            "override_defensive_stat" => {
                                match context
                                    .active_move(active_move_handle)?
                                    .data
                                    .override_defensive_stat
                                {
                                    Some(stat) => ValueRef::Stat(stat),
                                    None => ValueRef::Undefined,
                                }
                            }
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let Some(mon_handle) = value.mon_handle() {
                        let context = unsafe { context.unsafely_detach_borrow_mut() };
                        value = match *member {
                            "ability" => ValueRef::TempString(
                                context
                                    .mon(mon_handle)?
                                    .volatile_state
                                    .ability
                                    .id
                                    .to_string(),
                            ),
                            "active" => ValueRef::Boolean(context.mon(mon_handle)?.active),
                            "active_move" => context
                                .mon(mon_handle)?
                                .active_move
                                .map(|active_move| ValueRef::ActiveMove(active_move))
                                .unwrap_or(ValueRef::Undefined),
                            "active_move_actions" => ValueRef::UFraction(
                                context.mon(mon_handle)?.active_move_actions.into(),
                            ),
                            "active_position" => match context.mon(mon_handle)?.active_position {
                                Some(active_position) => ValueRef::UFraction(
                                    TryInto::<u64>::try_into(active_position)
                                        .map_err(integer_overflow_error)?
                                        .into(),
                                ),
                                None => ValueRef::Undefined,
                            },
                            "active_turns" => {
                                ValueRef::UFraction(context.mon(mon_handle)?.active_turns.into())
                            }
                            "affection_level" => ValueRef::UFraction(
                                context.mon(mon_handle)?.affection_level().into(),
                            ),
                            "base_max_hp" => {
                                ValueRef::UFraction(context.mon(mon_handle)?.base_max_hp.into())
                            }
                            "base_species" => {
                                ValueRef::Str(&context.mon(mon_handle)?.base_species.as_ref())
                            }
                            "base_stats" => {
                                ValueRef::StatTable(&context.mon(mon_handle)?.base_stored_stats)
                            }
                            "being_called_back" => ValueRef::Boolean(
                                context.mon(mon_handle)?.switch_state.being_called_back,
                            ),
                            "berry_eating_health" => ValueRef::UFraction(
                                mon_states::berry_eating_health(
                                    &mut context.mon_context(mon_handle)?,
                                )
                                .into(),
                            ),
                            "boosts" => ValueRef::BoostTable(
                                &context.mon(mon_handle)?.volatile_state.boosts,
                            ),
                            "can_heal" => ValueRef::Boolean(mon_states::can_heal(
                                &mut context.mon_context(mon_handle)?,
                            )),
                            "can_suppress_ability" => {
                                ValueRef::Boolean(mon_states::can_suppress_ability(
                                    &mut context.mon_context(mon_handle)?,
                                ))
                            }
                            "can_suppress_item" => {
                                ValueRef::Boolean(mon_states::can_suppress_item(
                                    &mut context.mon_context(mon_handle)?,
                                ))
                            }
                            "damaged_this_turn" => ValueRef::Boolean(
                                context.mon(mon_handle)?.volatile_state.damaged_this_turn,
                            ),
                            "dynamaxed" => ValueRef::Boolean(context.mon(mon_handle)?.dynamaxed),
                            "effective_ability" => {
                                match mon_states::effective_ability(
                                    &mut context.mon_context(mon_handle)?,
                                ) {
                                    Some(ability) => {
                                        ValueRef::TempEffect(EffectHandle::Ability(ability))
                                    }
                                    None => ValueRef::Undefined,
                                }
                            }
                            "effective_item" => {
                                match mon_states::effective_item(
                                    &mut context.mon_context(mon_handle)?,
                                ) {
                                    Some(item) => ValueRef::TempEffect(EffectHandle::Item(item)),
                                    None => ValueRef::Undefined,
                                }
                            }
                            "effective_types" => ValueRef::TempList(
                                mon_states::effective_types(&mut context.mon_context(mon_handle)?)
                                    .iter()
                                    .map(|val| {
                                        ValueRefToStoredValue::new(None, ValueRef::Type(*val))
                                    })
                                    .collect(),
                            ),
                            "effective_weather" => {
                                match mon_states::effective_weather(
                                    &mut context.mon_context(mon_handle)?,
                                ) {
                                    Some(weather) => ValueRef::Effect(
                                        context
                                            .battle_context_mut()
                                            .battle_mut()
                                            .get_effect_handle_by_id(&weather)?,
                                    ),
                                    None => ValueRef::Undefined,
                                }
                            }
                            "effective_terrain" => {
                                match mon_states::effective_terrain(
                                    &mut context.mon_context(mon_handle)?,
                                ) {
                                    Some(weather) => ValueRef::Effect(
                                        context
                                            .battle_context_mut()
                                            .battle_mut()
                                            .get_effect_handle_by_id(&weather)?,
                                    ),
                                    None => ValueRef::Undefined,
                                }
                            }
                            "exited" => {
                                ValueRef::Boolean(context.mon(mon_handle)?.exited.is_some())
                            }
                            "fainted" => ValueRef::Boolean(
                                context.mon(mon_handle)?.exited == Some(MonExitType::Fainted),
                            ),
                            "foe_side" => {
                                ValueRef::Side(context.mon_context(mon_handle)?.foe_side().index)
                            }
                            "force_switch" => ValueRef::Boolean(
                                context.mon(mon_handle)?.switch_state.force_switch.is_some(),
                            ),
                            "friendship" => {
                                ValueRef::UFraction(context.mon(mon_handle)?.friendship.into())
                            }
                            "gender" => ValueRef::Gender(context.mon(mon_handle)?.gender),
                            "hidden_power_type" => {
                                ValueRef::Type(context.mon(mon_handle)?.hidden_power_type)
                            }
                            "hp" => ValueRef::UFraction(context.mon(mon_handle)?.hp.into()),
                            "illusion" => ValueRef::Boolean(
                                context.mon(mon_handle)?.volatile_state.illusion.is_some(),
                            ),
                            "is_asleep" => ValueRef::Boolean(mon_states::is_asleep(
                                &mut context.mon_context(mon_handle)?,
                            )),
                            "is_away_from_field" => {
                                ValueRef::Boolean(mon_states::is_away_from_field(
                                    &mut context.mon_context(mon_handle)?,
                                ))
                            }
                            "is_behind_substitute" => {
                                ValueRef::Boolean(mon_states::is_behind_substitute(
                                    &mut context.mon_context(mon_handle)?,
                                ))
                            }
                            "is_grounded" => ValueRef::Boolean(mon_states::is_grounded(
                                &mut context.mon_context(mon_handle)?,
                            )),
                            "is_immune_to_entry_hazards" => {
                                ValueRef::Boolean(mon_states::is_immune_to_entry_hazards(
                                    &mut context.mon_context(mon_handle)?,
                                ))
                            }
                            "is_semi_invulnerable" => {
                                ValueRef::Boolean(mon_states::is_semi_invulnerable(
                                    &mut context.mon_context(mon_handle)?,
                                ))
                            }
                            "is_soundproof" => ValueRef::Boolean(mon_states::is_soundproof(
                                &mut context.mon_context(mon_handle)?,
                            )),
                            "item" => match context.mon(mon_handle)?.item.as_ref() {
                                Some(item) => ValueRef::TempString(item.to_string()),
                                None => ValueRef::Undefined,
                            },
                            "item_used_this_turn" => ValueRef::Boolean(
                                context.mon(mon_handle)?.volatile_state.item_used_this_turn,
                            ),
                            "last_item" => {
                                match context.mon(mon_handle)?.volatile_state.last_item.as_ref() {
                                    Some(item) => ValueRef::TempString(item.to_string()),
                                    None => ValueRef::Undefined,
                                }
                            }
                            "last_move" => {
                                match context.mon(mon_handle)?.volatile_state.last_move {
                                    Some(last_move) => ValueRef::ActiveMove(last_move),
                                    _ => ValueRef::Undefined,
                                }
                            }
                            "last_move_used" => {
                                match context.mon(mon_handle)?.volatile_state.last_move_used {
                                    Some(last_move_used) => ValueRef::ActiveMove(last_move_used),
                                    _ => ValueRef::Undefined,
                                }
                            }
                            "last_target_location" => {
                                match context
                                    .mon(mon_handle)?
                                    .volatile_state
                                    .last_move_target_location
                                {
                                    Some(last_target_location) => ValueRef::Fraction(
                                        TryInto::<i32>::try_into(last_target_location)
                                            .map_err(integer_overflow_error)?
                                            .into(),
                                    ),
                                    None => ValueRef::Undefined,
                                }
                            }
                            "level" => ValueRef::UFraction(context.mon(mon_handle)?.level.into()),
                            "max_hp" => ValueRef::UFraction(context.mon(mon_handle)?.max_hp.into()),
                            "move_last_turn_succeeded" => ValueRef::Boolean(
                                context
                                    .mon(mon_handle)?
                                    .volatile_state
                                    .move_last_turn_outcome
                                    .map(|outcome| outcome.success())
                                    .unwrap_or(false),
                            ),
                            "move_slots" => ValueRef::TempList(
                                context
                                    .mon(mon_handle)?
                                    .volatile_state
                                    .move_slots
                                    .iter()
                                    .map(|move_slot| {
                                        ValueRefToStoredValue::new(
                                            self.stored.clone(),
                                            ValueRef::MoveSlot(move_slot),
                                        )
                                    })
                                    .collect(),
                            ),
                            "move_this_turn_failed" => ValueRef::Boolean(
                                context
                                    .mon(mon_handle)?
                                    .volatile_state
                                    .move_this_turn_outcome
                                    .map(|outcome| !outcome.success())
                                    .unwrap_or(false),
                            ),
                            "moved_this_turn" => ValueRef::Boolean(
                                context
                                    .mon(mon_handle)?
                                    .volatile_state
                                    .move_this_turn_outcome
                                    .is_some(),
                            ),
                            "name" => ValueRef::String(&context.mon(mon_handle)?.name),
                            "nature" => ValueRef::Nature(context.mon(mon_handle)?.nature),
                            "needs_switch" => ValueRef::Boolean(
                                context.mon(mon_handle)?.switch_state.needs_switch.is_some(),
                            ),
                            "newly_switched" => {
                                ValueRef::Boolean(context.mon(mon_handle)?.newly_switched)
                            }
                            "not_fully_evolved" => ValueRef::Boolean(
                                context
                                    .battle_context()
                                    .battle()
                                    .dex
                                    .species
                                    .get_by_id(&context.mon(mon_handle)?.base_species)?
                                    .data
                                    .not_fully_evolved(),
                            ),
                            "player" => ValueRef::Player(context.mon(mon_handle)?.player),
                            "position" => {
                                match Mon::position_on_side(&context.mon_context(mon_handle)?) {
                                    Some(position) => ValueRef::UFraction(
                                        TryInto::<u32>::try_into(position)
                                            .map_err(integer_overflow_error)?
                                            .into(),
                                    ),
                                    None => ValueRef::Undefined,
                                }
                            }
                            "position_details" => ValueRef::TempString(format!(
                                "{}",
                                Mon::position_details(&context.mon_context(mon_handle)?)?
                            )),
                            "positive_boosts" => ValueRef::UFraction(
                                Mon::positive_boosts(&context.mon_context(mon_handle)?).into(),
                            ),
                            "side" => ValueRef::Side(context.mon(mon_handle)?.side),
                            "species" => ValueRef::Str(
                                &context.mon(mon_handle)?.volatile_state.species.as_ref(),
                            ),
                            "stats" => {
                                ValueRef::StatTable(&context.mon(mon_handle)?.volatile_state.stats)
                            }
                            "status" => match context.mon(mon_handle)?.status.as_ref() {
                                Some(status) => ValueRef::TempString(status.as_ref().to_owned()),
                                None => ValueRef::Undefined,
                            },
                            "terastallized" => context
                                .mon(mon_handle)?
                                .terastallized
                                .map(|val| ValueRef::Type(val))
                                .unwrap_or(ValueRef::Undefined),
                            "transformed" => ValueRef::Boolean(
                                context.mon(mon_handle)?.volatile_state.transformed,
                            ),
                            "true_nature" => ValueRef::Nature(context.mon(mon_handle)?.true_nature),
                            "types" => ValueRef::TempList(
                                context
                                    .mon(mon_handle)?
                                    .volatile_state
                                    .types
                                    .iter()
                                    .map(|val| {
                                        ValueRefToStoredValue::new(None, ValueRef::Type(*val))
                                    })
                                    .collect(),
                            ),
                            "undynamaxed_hp" => ValueRef::UFraction(
                                context.mon(mon_handle)?.undynamaxed_hp().into(),
                            ),
                            "weight" => ValueRef::UFraction(
                                Mon::get_weight(&mut context.mon_context(mon_handle)?).into(),
                            ),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::Player(player) = value {
                        let context = unsafe { context.unsafely_detach_borrow_mut() };
                        value = match *member {
                            "can_escape" => ValueRef::Boolean(Player::can_escape(
                                &context.battle_context_mut().player_context(player)?,
                            )),
                            "mon_fainted_last_turn" => ValueRef::Boolean(
                                context
                                    .battle_context_mut()
                                    .player_context(player)?
                                    .player()
                                    .fainted_last_turn,
                            ),
                            "team" => ValueRef::TempList(
                                context
                                    .battle_context_mut()
                                    .player_context(player)?
                                    .player()
                                    .mon_handles()
                                    .map(|mon| {
                                        ValueRefToStoredValue::new(None, ValueRef::Mon(*mon))
                                    })
                                    .collect(),
                            ),
                            "team_by_effective_position" => ValueRef::TempList(
                                Player::mon_handles_by_effective_position(
                                    &context.battle_context_mut().player_context(player)?,
                                )?
                                .map(|mon| ValueRefToStoredValue::new(None, ValueRef::Mon(mon)))
                                .collect(),
                            ),
                            "total_active_positions" => ValueRef::UFraction(
                                TryInto::<u64>::try_into(
                                    context
                                        .battle_context_mut()
                                        .player_context(player)?
                                        .player()
                                        .total_active_positions(),
                                )
                                .map_err(integer_overflow_error)?
                                .into(),
                            ),
                            "wild_encounter_type" => Player::wild_encounter_type(
                                &mut context.battle_context_mut().player_context(player)?,
                            )
                            .map(|val| ValueRef::WildEncounterType(val))
                            .unwrap_or(ValueRef::Undefined),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::MoveSlot(move_slot) = value {
                        value = match *member {
                            "id" => ValueRef::Str(move_slot.id.as_ref()),
                            "max_pp" => ValueRef::UFraction(move_slot.max_pp.into()),
                            "name" => ValueRef::String(&move_slot.name),
                            "pp" => ValueRef::UFraction(move_slot.pp.into()),
                            "used" => ValueRef::Boolean(move_slot.used),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::Battle = value {
                        let context = unsafe { context.unsafely_detach_borrow_mut() };
                        value = match *member {
                            "ending" => {
                                ValueRef::Boolean(context.battle_context().battle().ending())
                            }
                            "last_move" => context
                                .battle_context()
                                .battle()
                                .last_move()
                                .map(|move_handle| ValueRef::ActiveMove(move_handle))
                                .unwrap_or(ValueRef::Undefined),
                            "last_successful_move" => context
                                .battle_context()
                                .battle()
                                .last_successful_move()
                                .map(|move_handle| ValueRef::ActiveMove(move_handle))
                                .unwrap_or(ValueRef::Undefined),
                            "turn" => {
                                ValueRef::UFraction(context.battle_context().battle().turn().into())
                            }
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::Field = value {
                        let context = unsafe { context.unsafely_detach_borrow_mut() };
                        value = match *member {
                            "effective_terrain" => {
                                match Field::effective_terrain(context.battle_context_mut()) {
                                    Some(weather) => ValueRef::Effect(
                                        context
                                            .battle_context_mut()
                                            .battle_mut()
                                            .get_effect_handle_by_id(&weather)?,
                                    ),
                                    None => ValueRef::Undefined,
                                }
                            }
                            "effective_weather" => {
                                match Field::effective_weather(context.battle_context_mut()) {
                                    Some(weather) => ValueRef::Effect(
                                        context
                                            .battle_context_mut()
                                            .battle_mut()
                                            .get_effect_handle_by_id(&weather)?,
                                    ),
                                    None => ValueRef::Undefined,
                                }
                            }
                            "environment" => ValueRef::FieldEnvironment(
                                context.battle_context().battle().field.environment,
                            ),
                            "sides" => ValueRef::TempList(
                                context
                                    .battle_context()
                                    .battle()
                                    .side_indices()
                                    .map(|side_index| {
                                        ValueRefToStoredValue::new(None, ValueRef::Side(side_index))
                                    })
                                    .collect(),
                            ),
                            "time" => {
                                ValueRef::TimeOfDay(context.battle_context().battle().field.time)
                            }
                            "weather" => {
                                match context.battle_context().battle().field.weather.clone() {
                                    Some(weather) => ValueRef::Effect(
                                        context
                                            .battle_context_mut()
                                            .battle_mut()
                                            .get_effect_handle_by_id(&weather)?,
                                    ),
                                    None => ValueRef::Undefined,
                                }
                            }
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::Format = value {
                        let context = unsafe { context.unsafely_detach_borrow_mut() };
                        value = match *member {
                            "mons_per_side" => ValueRef::UFraction(
                                TryInto::<u64>::try_into(
                                    context.battle_context().battle().max_side_length(),
                                )
                                .map_err(integer_overflow_error)?
                                .into(),
                            ),
                            "obedience_cap" => ValueRef::UFraction(
                                context
                                    .battle_context()
                                    .battle()
                                    .format
                                    .rules
                                    .numeric_rules
                                    .obedience_cap
                                    .into(),
                            ),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        };
                    } else if let ValueRef::HitEffect(hit_effect) = value {
                        value = match *member {
                            "boosts" => hit_effect
                                .boosts
                                .as_ref()
                                .map(ValueRef::BoostTable)
                                .unwrap_or(ValueRef::Undefined),
                            "volatile_status" => hit_effect
                                .volatile_status
                                .as_ref()
                                .map(ValueRef::String)
                                .unwrap_or(ValueRef::Undefined),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::SecondaryHitEffect(secondary_effect) = value {
                        value = match *member {
                            "chance" => secondary_effect
                                .chance
                                .as_ref()
                                .map(|val| ValueRef::UFraction(val.convert()))
                                .unwrap_or(ValueRef::Undefined),
                            "target" => secondary_effect
                                .target
                                .as_ref()
                                .map(ValueRef::HitEffect)
                                .unwrap_or(ValueRef::Undefined),
                            "user" => secondary_effect
                                .user
                                .as_ref()
                                .map(ValueRef::HitEffect)
                                .unwrap_or(ValueRef::Undefined),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::BoostTable(boosts) = value {
                        value = match *member {
                            "acc" => ValueRef::Fraction(boosts.acc.into()),
                            "atk" => ValueRef::Fraction(boosts.atk.into()),
                            "def" => ValueRef::Fraction(boosts.def.into()),
                            "eva" => ValueRef::Fraction(boosts.eva.into()),
                            "spa" => ValueRef::Fraction(boosts.spa.into()),
                            "spd" => ValueRef::Fraction(boosts.spd.into()),
                            "spe" => ValueRef::Fraction(boosts.spe.into()),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::StatTable(stats) = value {
                        value = match *member {
                            "atk" => ValueRef::Fraction(stats.atk.into()),
                            "def" => ValueRef::Fraction(stats.def.into()),
                            "hp" => ValueRef::Fraction(stats.hp.into()),
                            "spa" => ValueRef::Fraction(stats.spa.into()),
                            "spd" => ValueRef::Fraction(stats.spd.into()),
                            "spe" => ValueRef::Fraction(stats.spe.into()),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::Nature(nature) = value {
                        value = match *member {
                            "boosts" => {
                                ValueRef::Boost(nature.boosts().try_into().map_err(general_error)?)
                            }
                            "drops" => {
                                ValueRef::Boost(nature.drops().try_into().map_err(general_error)?)
                            }
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::SpecialItemData(special_item_data) = value {
                        value = match *member {
                            "fling" => special_item_data
                                .fling
                                .as_ref()
                                .map(ValueRef::FlingData)
                                .unwrap_or(ValueRef::Undefined),
                            "judgment" => special_item_data
                                .judgment
                                .as_ref()
                                .map(ValueRef::JudgmentData)
                                .unwrap_or(ValueRef::Undefined),
                            "natural_gift" => special_item_data
                                .natural_gift
                                .as_ref()
                                .map(ValueRef::NaturalGiftData)
                                .unwrap_or(ValueRef::Undefined),
                            "techno_blast" => special_item_data
                                .techno_blast
                                .as_ref()
                                .map(ValueRef::TechnoBlastData)
                                .unwrap_or(ValueRef::Undefined),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::FlingData(fling_data) = value {
                        value = match *member {
                            "hit_effect" => fling_data
                                .hit_effect
                                .as_ref()
                                .map(|hit_effect| ValueRef::HitEffect(hit_effect))
                                .unwrap_or(ValueRef::Undefined),
                            "power" => ValueRef::UFraction(fling_data.power.into()),
                            "use_item" => ValueRef::Boolean(fling_data.use_item),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::NaturalGiftData(natural_gift_data) = value {
                        value = match *member {
                            "power" => ValueRef::UFraction(natural_gift_data.power.into()),
                            "type" => ValueRef::Type(natural_gift_data.typ),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::MultihitType(multihit) = value {
                        value = match *member {
                            "max" => ValueRef::UFraction(multihit.max().into()),
                            "min" => ValueRef::UFraction(multihit.min().into()),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::JudgmentData(judgment_data) = value {
                        value = match *member {
                            "type" => ValueRef::Type(judgment_data.typ),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::TechnoBlastData(techno_blast_data) = value {
                        value = match *member {
                            "type" => ValueRef::Type(techno_blast_data.typ),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::EffectState(connector) = value {
                        let context = unsafe { context.unsafely_detach_borrow_mut() };
                        value = connector
                            .get_mut(context.battle_context_mut())?
                            .get(*member)
                            .map(ValueRef::from)
                            .unwrap_or(ValueRef::Undefined);
                    } else if let ValueRef::Object(object) = value {
                        value = match object.get(*member) {
                            Some(value) => ValueRef::from(value),
                            _ => ValueRef::Undefined,
                        };
                    } else {
                        return Err(Self::bad_member_access(member, value_type));
                    }
                }
            }
        }

        Ok(value)
    }

    pub fn get(
        self,
        context: &'eval mut EvaluationContext,
    ) -> Result<ValueRefToStoredValue<'eval>> {
        let value_ref = self.get_ref(context)?;
        // SAFETY: This ValueRef references some internal part of `self.stored`. Since we are
        // bundling this reference alongside the owner object (which has runtime borrow checking),
        // we promote this reference to its lifetime.
        //
        // An added bonus is that we know we only use this value for an immutable operation.
        let value_ref = unsafe { core::mem::transmute::<ValueRef<'_>, ValueRef<'eval>>(value_ref) };
        Ok(ValueRefToStoredValue::new(self.stored, value_ref))
    }
}

/// A mutable fxlang variable.
///
/// Acts as a wrapper for a mutable access of a variable that can be consumed at some later time.
pub struct VariableMut<'eval, 'program> {
    stored: ElementRefMut<'eval, Value>,
    member_access: Vec<&'program str>,
}

impl<'eval, 'program> VariableMut<'eval, 'program>
where
    'program: 'eval,
{
    pub fn new(stored: ElementRefMut<'eval, Value>, member_access: Vec<&'program str>) -> Self {
        Self {
            stored,
            member_access,
        }
    }

    fn bad_member_or_mutable_access(member: &str, value_type: ValueType) -> Error {
        general_error(format!(
            "value of type {value_type} has no member {member} or the member is immutable",
        ))
    }

    fn get_ref_mut<'var>(
        &'var mut self,
        context: &'eval mut EvaluationContext,
    ) -> Result<ValueRefMut<'var>> {
        let mut value = ValueRefMut::from(self.stored.as_mut());

        for member in &self.member_access {
            let value_type = value.value_type();

            // SAFETY: For changing the lifetime of context: the mutable reference inside of
            // `value_ref` is only mutated at the very end of this method. Thus, this entire for
            // loop is actually immutable. Furthermore, since we only hold one
            // `value_ref` at a time, so there is no way to hold multiple mutable borrow
            // of values inside `context` at the same time.
            //
            // We can imagine that `value_ref` really does correctly mutably borrow `context`. If
            // the next iteration somehow also needs to borrow from `context`, the previous
            // `value_ref` value (i.e., the mutable borrow inside of it) is dropped.
            match value {
                ValueRefMut::Mon(ref mon_handle) => {
                    let context = unsafe { context.unsafely_detach_borrow_mut() };
                    value = match *member {
                        "boosts" => ValueRefMut::BoostTable(
                            &mut context.mon_mut(**mon_handle)?.volatile_state.boosts,
                        ),
                        "last_item" => ValueRefMut::OptionalId(
                            &mut context.mon_mut(**mon_handle)?.volatile_state.last_item,
                        ),
                        "last_target_location" => ValueRefMut::OptionalISize(
                            &mut context
                                .mon_mut(**mon_handle)?
                                .volatile_state
                                .last_move_target_location,
                        ),
                        "skip_before_switch_out" => ValueRefMut::Boolean(
                            &mut context
                                .mon_mut(**mon_handle)?
                                .switch_state
                                .skip_before_switch_out,
                        ),
                        "stats" => ValueRefMut::StatTable(
                            &mut context.mon_mut(**mon_handle)?.volatile_state.stats,
                        ),
                        "status_state" => ValueRefMut::TempEffectState(
                            MonStatusEffectStateConnector::new(**mon_handle).make_dynamic(),
                        ),
                        _ => return Err(Self::bad_member_or_mutable_access(member, value_type)),
                    }
                }
                ValueRefMut::ActiveMove(ref active_move_handle) => {
                    let context = unsafe { context.unsafely_detach_borrow_mut() };
                    value = match *member {
                        "accuracy" => ValueRefMut::Accuracy(
                            &mut context.active_move_mut(**active_move_handle)?.data.accuracy,
                        ),
                        "base_power" => ValueRefMut::U32(
                            &mut context
                                .active_move_mut(**active_move_handle)?
                                .data
                                .base_power,
                        ),
                        "category" => ValueRefMut::MoveCategory(
                            &mut context.active_move_mut(**active_move_handle)?.data.category,
                        ),
                        "damage" => ValueRefMut::OptionalU16(
                            &mut context.active_move_mut(**active_move_handle)?.data.damage,
                        ),
                        "effect_state" => ValueRefMut::TempEffectState(
                            ActiveMoveEffectStateConnector::new(**active_move_handle)
                                .make_dynamic(),
                        ),
                        "force_stab" => ValueRefMut::Boolean(
                            &mut context
                                .active_move_mut(**active_move_handle)?
                                .data
                                .force_stab,
                        ),
                        "hit_effect" => ValueRefMut::OptionalHitEffect(
                            &mut context
                                .active_move_mut(**active_move_handle)?
                                .data
                                .hit_effect,
                        ),
                        "ignore_all_secondary_effects" => ValueRefMut::Boolean(
                            &mut context
                                .active_move_mut(**active_move_handle)?
                                .ignore_all_secondary_effects,
                        ),
                        "multiaccuracy" => ValueRefMut::Boolean(
                            &mut context
                                .active_move_mut(**active_move_handle)?
                                .data
                                .multiaccuracy,
                        ),
                        "multihit" => ValueRefMut::OptionalMultihitType(
                            &mut context.active_move_mut(**active_move_handle)?.data.multihit,
                        ),
                        "priority" => ValueRefMut::I8(
                            &mut context.active_move_mut(**active_move_handle)?.data.priority,
                        ),
                        "secondary_effects" => ValueRefMut::SecondaryHitEffectList(
                            &mut context
                                .active_move_mut(**active_move_handle)?
                                .data
                                .secondary_effects,
                        ),
                        "target" => ValueRefMut::MoveTarget(
                            &mut context.active_move_mut(**active_move_handle)?.data.target,
                        ),
                        "total_damage" => ValueRefMut::U64(
                            &mut context.active_move_mut(**active_move_handle)?.total_damage,
                        ),
                        "type" => ValueRefMut::Type(
                            &mut context
                                .active_move_mut(**active_move_handle)?
                                .data
                                .primary_type,
                        ),
                        "user_effect" => ValueRefMut::OptionalHitEffect(
                            &mut context
                                .active_move_mut(**active_move_handle)?
                                .data
                                .user_effect,
                        ),
                        "user_effect_chance" => ValueRefMut::OptionalFractionU16(
                            &mut context
                                .active_move_mut(**active_move_handle)?
                                .data
                                .user_effect_chance,
                        ),
                        "override_offensive_stat" => ValueRefMut::OptionalStat(
                            &mut context
                                .active_move_mut(**active_move_handle)?
                                .data
                                .override_offensive_stat,
                        ),
                        "override_defensive_stat" => ValueRefMut::OptionalStat(
                            &mut context
                                .active_move_mut(**active_move_handle)?
                                .data
                                .override_defensive_stat,
                        ),
                        _ => return Err(Self::bad_member_or_mutable_access(member, value_type)),
                    }
                }
                ValueRefMut::HitEffect(hit_effect)
                | ValueRefMut::OptionalHitEffect(Some(hit_effect)) => {
                    value = match *member {
                        "boosts" => ValueRefMut::OptionalBoostTable(&mut hit_effect.boosts),
                        "heal_percent" => {
                            ValueRefMut::OptionalFractionU16(&mut hit_effect.heal_percent)
                        }
                        "side_condition" => {
                            ValueRefMut::OptionalString(&mut hit_effect.side_condition)
                        }
                        "status" => ValueRefMut::OptionalString(&mut hit_effect.status),
                        "volatile_status" => {
                            ValueRefMut::OptionalString(&mut hit_effect.volatile_status)
                        }
                        _ => return Err(Self::bad_member_or_mutable_access(member, value_type)),
                    }
                }
                ValueRefMut::SecondaryHitEffect(secondary_effect) => {
                    value = match *member {
                        "chance" => ValueRefMut::OptionalFractionU16(&mut secondary_effect.chance),
                        "target" => ValueRefMut::OptionalHitEffect(&mut secondary_effect.target),
                        "user" => ValueRefMut::OptionalHitEffect(&mut secondary_effect.user),
                        _ => return Err(Self::bad_member_or_mutable_access(member, value_type)),
                    }
                }
                ValueRefMut::BoostTable(boosts) => {
                    value = match *member {
                        "acc" => ValueRefMut::I8(&mut boosts.acc),
                        "atk" => ValueRefMut::I8(&mut boosts.atk),
                        "def" => ValueRefMut::I8(&mut boosts.def),
                        "eva" => ValueRefMut::I8(&mut boosts.eva),
                        "spa" => ValueRefMut::I8(&mut boosts.spa),
                        "spd" => ValueRefMut::I8(&mut boosts.spd),
                        "spe" => ValueRefMut::I8(&mut boosts.spe),
                        _ => return Err(Self::bad_member_or_mutable_access(member, value_type)),
                    }
                }
                ValueRefMut::StatTable(stats) => {
                    value = match *member {
                        "atk" => ValueRefMut::U16(&mut stats.atk),
                        "def" => ValueRefMut::U16(&mut stats.def),
                        "spa" => ValueRefMut::U16(&mut stats.spa),
                        "spd" => ValueRefMut::U16(&mut stats.spd),
                        "spe" => ValueRefMut::U16(&mut stats.spe),
                        _ => return Err(Self::bad_member_or_mutable_access(member, value_type)),
                    }
                }
                ValueRefMut::EffectState(connector) => {
                    let context = unsafe { context.unsafely_detach_borrow_mut() };
                    value = ValueRefMut::from(
                        connector
                            .get_mut(context.battle_context_mut())?
                            .get_mut(*member),
                    );
                }
                ValueRefMut::TempEffectState(connector) => {
                    let context = unsafe { context.unsafely_detach_borrow_mut() };
                    value = ValueRefMut::from(
                        connector
                            .get_mut(context.battle_context_mut())?
                            .get_mut(*member),
                    );
                }
                ValueRefMut::Object(ref mut object) => {
                    // SAFETY: Mutably borrowing the object requires mutably borrowing this entire
                    // variable, so this can only happen once. If an object contains other objects,
                    // we are grabbing a different mutable borrow at each layer.
                    //
                    // When assigning to this mutable borrow, we take ownership of the assigning
                    // value first, so no operation will alter the object between grabbing this
                    // borrow and consuming it with an assignment.
                    let object = unsafe { object.unsafely_detach_borrow_mut() };
                    let entry = object
                        .entry((*member).to_owned())
                        .or_insert(Value::Undefined);
                    value = ValueRefMut::from(entry);
                }
                _ => return Err(Self::bad_member_or_mutable_access(member, value_type)),
            }
        }
        Ok(value)
    }

    pub fn get_mut<'var>(
        &'var mut self,
        context: &'eval mut EvaluationContext,
    ) -> Result<ValueRefMut<'var>> {
        self.get_ref_mut(context)
    }
}
