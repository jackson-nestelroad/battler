use alloc::{
    borrow::ToOwned,
    collections::VecDeque,
    format,
    string::{
        String,
        ToString,
    },
    vec::Vec,
};
use core::str::FromStr;

use anyhow::{
    Error,
    Result,
};
use battler_data::{
    AbilityFlag,
    Boost,
    BoostOrderIterator,
    BoostTable,
    HitEffect,
    Id,
    Identifiable,
    ItemFlag,
    MoveFlag,
    MoveTarget,
    SecondaryEffectData,
    SpeciesFlag,
    TypeEffectiveness,
};
use battler_prng::rand_util;
use hashbrown::{
    HashMap,
    HashSet,
};

use crate::{
    Type,
    battle::{
        ActiveMoveContext,
        ApplyingEffectContext,
        Context,
        CoreBattle,
        EffectContext,
        FieldEffectContext,
        Mon,
        MonContext,
        MonHandle,
        MoveOutcomeOnTarget,
        MoveSlot,
        Player,
        Side,
        SideEffectContext,
        core_battle_actions,
        core_battle_effects,
        core_battle_logs,
        mon_states,
    },
    battle_log_entry,
    effect::{
        AppliedEffectHandle,
        EffectHandle,
        MonAbilityEffectStateConnector,
        MonStatusEffectStateConnector,
        MonVolatileStatusEffectStateConnector,
        SideConditionEffectStateConnector,
        fxlang::{
            BattleEvent,
            DynamicEffectStateConnector,
            EffectStateConnector,
            EvaluationContext,
            EventState,
            MaybeReferenceValueForOperation,
            Value,
            ValueType,
            VariableInput,
        },
    },
    error::{
        WrapOptionError,
        WrapResultError,
        general_error,
    },
    log::UncommittedBattleLogEntry,
    moves::{
        Move,
        UpgradedMoveSource,
    },
};

/// Runs an fxlang function.
///
/// This function serves as the boundary between fxlang program evaluation and the battle engine.
pub fn run_function(
    context: &mut EvaluationContext,
    function_name: &str,
    args: VecDeque<Value>,
    event: BattleEvent,
    event_state: &EventState,
    effect_state: Option<DynamicEffectStateConnector>,
) -> Result<Option<Value>> {
    let context = FunctionContext::new(context, args, event, event_state, effect_state);
    match function_name {
        "ability_effect_state" => ability_effect_state(context),
        "ability_has_flag" => ability_has_flag(context).map(|val| Some(val)),
        "add_pseudo_weather" => add_pseudo_weather(context).map(|val| Some(val)),
        "add_secondary_effect_to_move" => add_secondary_effect_to_move(context).map(|()| None),
        "add_side_condition" => add_side_condition(context).map(|val| Some(val)),
        "add_slot_condition" => add_slot_condition(context).map(|val| Some(val)),
        "add_volatile" => add_volatile(context).map(|val| Some(val)),
        "adjacent_foes" => adjacent_foes(context).map(|val| Some(val)),
        "all_active_mons" => all_active_mons(context).map(|val| Some(val)),
        "all_active_mons_in_speed_order" => {
            all_active_mons_in_speed_order(context).map(|val| Some(val))
        }
        "all_active_mons_on_side" => all_active_mons_on_side(context).map(|val| Some(val)),
        "all_foes" => all_foes(context).map(|val| Some(val)),
        "all_mons_in_party" => all_mons_in_party(context).map(|val| Some(val)),
        "all_mons_on_side" => all_mons_on_side(context).map(|val| Some(val)),
        "all_types" => all_types(context).map(|val| Some(val)),
        "allies_and_self" => allies_and_self(context).map(|val| Some(val)),
        "any_mon_will_move_this_turn" => any_mon_will_move_this_turn(context).map(|val| Some(val)),
        "append" => append(context).map(|val| Some(val)),
        "apply_drain" => apply_drain(context).map(|()| None),
        "apply_recoil_damage" => apply_recoil_damage(context).map(|()| None),
        "base_species" => base_species(context).map(|val| Some(val)),
        "boost" => boost(context).map(|val| Some(val)),
        "boost_table" => boost_table(context).map(|val| Some(val)),
        "boostable_stats" => Ok(Some(boostable_stats())),
        "calculate_damage" => calculate_damage(context).map(|val| Some(val)),
        "calculate_confusion_damage" => calculate_confusion_damage(context).map(|val| Some(val)),
        "can_boost" => can_boost(context).map(|val| Some(val)),
        "can_escape" => can_escape(context).map(|val| Some(val)),
        "can_switch" => can_switch(context).map(|val| Some(val)),
        "cancel_move" => cancel_move(context).map(|val| Some(val)),
        "chance" => chance(context).map(|val| Some(val)),
        "check_immunity" => check_immunity(context).map(|val| Some(val)),
        "clamp_number" => clamp_number(context).map(|val| Some(val)),
        "clause_integer_value" => clause_integer_value(context),
        "clause_type_value" => clause_type_value(context),
        "clear_boosts" => clear_boosts(context).map(|()| None),
        "clear_negative_boosts" => clear_negative_boosts(context).map(|()| None),
        "clear_terrain" => clear_terrain(context).map(|val| Some(val)),
        "clear_weather" => clear_weather(context).map(|val| Some(val)),
        "clone_active_move" => clone_active_move(context).map(|val| Some(val)),
        "cure_status" => cure_status(context).map(|val| Some(val)),
        "damage" => damage(context).map(|val| Some(val)),
        "debug_log" => debug_log(context).map(|()| None),
        "decrease_friendship" => decrease_friendship(context).map(|()| None),
        "deduct_pp" => deduct_pp(context).map(|val| Some(val)),
        "direct_damage" => direct_damage(context).map(|()| None),
        "disable_move" => disable_move(context).map(|()| None),
        "discard_item" => discard_item(context).map(|val| Some(val)),
        "do_move" => do_move(context).map(|()| None),
        "do_not_animate_last_move" => do_not_animate_last_move(context).map(|()| None),
        "eat_item" => eat_item(context).map(|val| Some(val)),
        "eat_given_item" => eat_given_item(context).map(|val| Some(val)),
        "end_ability" => end_ability(context).map(|()| None),
        "end_dynamax" => end_dynamax(context).map(|()| None),
        "end_illusion" => end_illusion(context).map(|val| Some(val)),
        "end_item" => end_item(context).map(|()| None),
        "escape" => escape(context).map(|val| Some(val)),
        "faint" => faint(context).map(|()| None),
        "floor" => floor(context).map(|val| Some(val)),
        "force_fully_heal" => force_fully_heal(context).map(|()| None),
        "forme_change" => forme_change(context).map(|val| Some(val)),
        "get_all_moves" => get_all_moves(context).map(|val| Some(val)),
        "get_ability" => get_ability(context),
        "get_boost" => get_boost(context).map(|val| Some(val)),
        "get_item" => get_item(context),
        "get_move" => get_move(context),
        "get_move_targets" => get_move_targets(context).map(|val| Some(val)),
        "get_species" => get_species(context),
        "get_stat" => get_stat(context).map(|val| Some(val)),
        "has_ability" => has_ability(context).map(|val| Some(val)),
        "has_item" => has_item(context).map(|val| Some(val)),
        "has_move" => has_move(context).map(|val| Some(val)),
        "has_pseudo_weather" => has_pseudo_weather(context).map(|val| Some(val)),
        "has_side_condition" => has_side_condition(context).map(|val| Some(val)),
        "has_species_registered" => has_species_registered(context).map(|val| Some(val)),
        "has_type" => has_type(context).map(|val| Some(val)),
        "has_type_before_forced_types" => {
            has_type_before_forced_types(context).map(|val| Some(val))
        }
        "has_volatile" => has_volatile(context).map(|val| Some(val)),
        "heal" => heal(context).map(|val| Some(val)),
        "hit_effect" => hit_effect().map(|val| Some(val)),
        "increase_friendship" => increase_friendship(context).map(|()| None),
        "index" => index(context),
        "is_adjacent" => is_adjacent(context).map(|val| Some(val)),
        "is_ally" => is_ally(context).map(|val| Some(val)),
        "item_has_flag" => item_has_flag(context).map(|val| Some(val)),
        "last_attack" => last_attack(context),
        "log" => log(context).map(|()| None),
        "log_ability" => log_ability(context).map(|()| None),
        "log_activate" => log_activate(context).map(|()| None),
        "log_animate_move" => log_animate_move(context).map(|()| None),
        "log_announce_item" => log_announce_item(context).map(|()| None),
        "log_block" => log_block(context).map(|()| None),
        "log_cant" => log_cant(context).map(|()| None),
        "log_custom_effect" => log_custom_effect(context).map(|()| None),
        "log_end" => log_end(context).map(|()| None),
        "log_fail" => log_fail(context).map(|()| None),
        "log_fail_heal" => log_fail_heal(context).map(|()| None),
        "log_fail_unboost" => log_fail_unboost(context).map(|()| None),
        "log_field_activate" => log_field_activate(context).map(|()| None),
        "log_field_start" => log_field_start(context).map(|()| None),
        "log_field_end" => log_field_end(context).map(|()| None),
        "log_immune" => log_immune(context).map(|()| None),
        "log_ohko" => log_ohko(context).map(|()| None),
        "log_prepare_move" => log_prepare_move(context).map(|()| None),
        "log_side_end" => log_side_end(context).map(|()| None),
        "log_side_start" => log_side_start(context).map(|()| None),
        "log_single_move" => log_single_move(context).map(|()| None),
        "log_single_turn" => log_single_turn(context).map(|()| None),
        "log_start" => log_start(context).map(|()| None),
        "log_status" => log_status(context).map(|()| None),
        "log_weather" => log_weather(context).map(|()| None),
        "max" => max(context).map(|val| Some(val)),
        "max_move" => max_move(context),
        "min" => min(context).map(|val| Some(val)),
        "modify_move_type" => modify_move_type(context).map(|_| None),
        "mon_ability_suppressed_by_this_effect" => {
            mon_ability_suppressed_by_this_effect(context).map(|val| Some(val))
        }
        "mon_at_target_location" => mon_at_target_location(context),
        "mon_in_position" => mon_in_position(context),
        "move_at_move_slot_index" => move_at_move_slot_index(context),
        "move_crit_target" => move_crit_target(context).map(|val| Some(val)),
        "move_has_flag" => move_has_flag(context).map(|val| Some(val)),
        "move_hit_data_has_flag_against_target" => {
            move_hit_data_has_flag_against_target(context).map(|val| Some(val))
        }
        "move_hit_target" => move_hit_target(context).map(|val| Some(val)),
        "move_makes_contact" => move_makes_contact(context).map(|val| Some(val)),
        "move_slot" => move_slot(context).map(|val| Some(val)),
        "move_slot_at_index" => move_slot_at_index(context),
        "move_slot_index" => move_slot_index(context),
        "new_active_move" => new_active_move(context).map(|val| Some(val)),
        "new_active_move_from_local_data" => {
            new_active_move_from_local_data(context).map(|val| Some(val))
        }
        "new_object" => Ok(Some(new_object(context))),
        "object_keys" => object_keys(context).map(|val| Some(val)),
        "object_increment" => object_increment(context).map(|val| Some(val)),
        "object_get" => object_get(context),
        "object_set" => object_set(context).map(|val| Some(val)),
        "object_value" => object_value(context),
        "overwrite_move_slot" => overwrite_move_slot(context).map(|()| None),
        "pending_move_action_this_turn" => pending_move_action_this_turn(context),
        "plural" => plural(context).map(|val| Some(val)),
        "prepare_direct_move" => prepare_direct_move(context).map(|val| Some(val)),
        "primal_reversion" => primal_reversion(context).map(|val| Some(val)),
        "random" => random(context).map(|val| Some(val)),
        "random_target" => random_target(context),
        "received_attack" => received_attack(context).map(|val| Some(val)),
        "remove" => remove(context).map(|val| Some(val)),
        "remove_move_flag" => remove_move_flag(context).map(|()| None),
        "remove_pseudo_weather" => remove_pseudo_weather(context).map(|val| Some(val)),
        "remove_side_condition" => remove_side_condition(context).map(|val| Some(val)),
        "remove_slot_condition" => remove_slot_condition(context).map(|val| Some(val)),
        "remove_volatile" => remove_volatile(context).map(|val| Some(val)),
        "restore_pp" => restore_pp(context).map(|val| Some(val)),
        "reverse" => reverse(context).map(|val| Some(val)),
        "revive" => revive(context).map(|val| Some(val)),
        "run_event" => run_event(context).map(|val| Some(val)),
        "run_event_for_each_active_mon" => run_event_for_each_active_mon(context).map(|()| None),
        "run_event_for_mon" => run_event_for_mon(context).map(|val| Some(val)),
        "run_event_on_mon_ability" => run_event_on_mon_ability(context),
        "run_event_on_mon_item" => run_event_on_mon_item(context),
        "run_event_on_mon_volatile" => run_event_on_mon_volatile(context),
        "run_event_on_move" => run_event_on_move(context),
        "sample" => sample(context),
        "save_move_hit_data_flag_against_target" => {
            save_move_hit_data_flag_against_target(context).map(|()| None)
        }
        "secondary_hit_effect" => secondary_hit_effect().map(|val| Some(val)),
        "set_ability" => set_ability(context).map(|val| Some(val)),
        "set_boost" => set_boost(context).map(|val| Some(val)),
        "set_friendship" => set_friendship(context).map(|()| None),
        "set_hp" => set_hp(context).map(|val| Some(val)),
        "set_illusion" => set_illusion(context).map(|val| Some(val)),
        "set_item" => set_item(context).map(|val| Some(val)),
        "set_upgraded_to_max_move" => set_upgraded_to_max_move(context).map(|_| None),
        "set_pp" => set_pp(context).map(|val| Some(val)),
        "set_status" => set_status(context).map(|val| Some(val)),
        "set_types" => set_types(context).map(|val| Some(val)),
        "set_terrain" => set_terrain(context).map(|val| Some(val)),
        "set_weather" => set_weather(context).map(|val| Some(val)),
        "side_condition_effect_state" => side_condition_effect_state(context),
        "skip_effect_callback" => skip_effect_callback(context).map(|()| None),
        "special_item_data" => special_item_data(context).map(|val| Some(val)),
        "species_has_flag" => species_has_flag(context).map(|val| Some(val)),
        "start_ability" => start_ability(context).map(|()| None),
        "start_item" => start_item(context).map(|()| None),
        "status_effect_state" => status_effect_state(context),
        "swap_boosts" => swap_boosts(context).map(|()| None),
        "take_item" => take_item(context),
        "target_location_of_mon" => target_location_of_mon(context).map(|val| Some(val)),
        "transform_into" => transform_into(context).map(|val| Some(val)),
        "type_chart_effectiveness" => type_chart_effectiveness(context).map(|val| Some(val)),
        "type_chart_immunity" => type_chart_immunity(context).map(|val| Some(val)),
        "type_effectiveness" => type_effectiveness(context).map(|val| Some(val)),
        "type_has_no_effect_against" => type_has_no_effect_against(context).map(|val| Some(val)),
        "type_is_weak_against" => type_is_weak_against(context).map(|val| Some(val)),
        "type_modifier" => type_modifier(context).map(|val| Some(val)),
        "type_modifier_against_target" => type_modifier_against_target(context),
        "use_active_move" => use_active_move(context).map(|val| Some(val)),
        "use_given_item" => use_given_item(context).map(|val| Some(val)),
        "use_item" => use_item(context).map(|val| Some(val)),
        "use_move" => use_move(context).map(|val| Some(val)),
        "valid_target" => valid_target(context).map(|val| Some(val)),
        "value_from_local_data" => value_from_local_data(context),
        "volatile_status_state" => volatile_status_state(context),
        "will_move_this_turn" => will_move_this_turn(context).map(|val| Some(val)),
        _ => Err(general_error(format!(
            "undefined function: {function_name}"
        ))),
    }
}

struct FunctionContext<'eval, 'effect, 'context, 'battle, 'data> {
    context: &'eval mut EvaluationContext<'effect, 'context, 'battle, 'data>,
    args: VecDeque<Value>,
    event: BattleEvent,
    event_state: &'eval EventState,
    effect_state: Option<DynamicEffectStateConnector>,
    flags: HashMap<String, bool>,
}

impl<'eval, 'effect, 'context, 'battle, 'data>
    FunctionContext<'eval, 'effect, 'context, 'battle, 'data>
{
    fn new(
        context: &'eval mut EvaluationContext<'effect, 'context, 'battle, 'data>,
        args: VecDeque<Value>,
        event: BattleEvent,
        event_state: &'eval EventState,
        effect_state: Option<DynamicEffectStateConnector>,
    ) -> Self {
        Self {
            context,
            args,
            event,
            event_state,
            effect_state,
            flags: HashMap::default(),
        }
    }

    fn evaluation_context(&self) -> &EvaluationContext<'effect, 'context, 'battle, 'data> {
        self.context
    }

    fn evaluation_context_mut(
        &mut self,
    ) -> &mut EvaluationContext<'effect, 'context, 'battle, 'data> {
        self.context
    }

    #[allow(unused)]
    fn event(&self) -> BattleEvent {
        self.event
    }

    fn event_state(&self) -> &EventState {
        self.event_state
    }

    fn effect_state(&self) -> Option<DynamicEffectStateConnector> {
        self.effect_state.clone()
    }

    fn applied_effect_handle(&mut self) -> Result<Option<AppliedEffectHandle>> {
        let effect_state = match self.effect_state() {
            Some(effect_state) => effect_state,
            None => return Ok(None),
        };
        Ok(Some(AppliedEffectHandle::new(
            self.evaluation_context().effect_handle().clone(),
            effect_state.applied_effect_location(),
        )))
    }

    fn front(&self) -> Option<&Value> {
        self.args.front()
    }

    fn pop_front(&mut self) -> Option<Value> {
        self.args.pop_front()
    }

    fn has_flag_internal(&mut self, flag: &str) -> bool {
        match self
            .args
            .iter()
            .enumerate()
            .find(|(_, arg)| (*arg).clone().string().is_ok_and(|arg| arg == flag))
        {
            Some((i, _)) => {
                self.args.remove(i);
                true
            }
            None => false,
        }
    }

    fn has_flag(&mut self, flag: &str) -> bool {
        if self.flags.get(flag).cloned().is_some_and(|val| val) {
            // Still check the flag value so that it is removed, in case it was forcefully set but
            // is still set by the function call.
            self.has_flag_internal(flag);
            return true;
        }
        let val = self.has_flag_internal(flag);
        self.flags.insert(flag.to_owned(), val);
        val
    }

    fn set_flag(&mut self, flag: &str, val: bool) {
        self.flags.insert(flag.to_owned(), val);
    }

    fn boosts_from_rest_of_args(&mut self) -> Result<BoostTable> {
        let mut args = VecDeque::new();
        core::mem::swap(&mut args, &mut self.args);
        let has_boost_table = args
            .front()
            .is_some_and(|val| val.value_type() == ValueType::BoostTable);
        if has_boost_table {
            return args
                .pop_front()
                .wrap_expectation("missing boost table")?
                .boost_table()
                .wrap_error_with_message("invalid boost table");
        }
        let boosts = args
            .into_iter()
            .map(|boost| StatBoost::from_str(&boost.string()?))
            .map(|res| res.wrap_error_with_message("invalid boost"))
            .collect::<Result<Vec<_>>>()?;
        Ok(BoostTable::from_iter(
            boosts.into_iter().map(|boost| (boost.0, boost.1)),
        ))
    }

    fn from_effect(&mut self) -> bool {
        self.has_flag("from_effect")
    }

    fn link(&mut self) -> bool {
        self.has_flag("link")
    }

    fn link_handle(&mut self) -> Result<Option<AppliedEffectHandle>> {
        if self.link() {
            self.applied_effect_handle()
        } else {
            Ok(None)
        }
    }

    fn no_effect(&mut self) -> bool {
        self.has_flag("no_effect")
    }

    fn set_no_effect(&mut self, val: bool) {
        self.set_flag("no_effect", val)
    }

    fn no_events(&mut self) -> bool {
        self.has_flag("no_events")
    }

    fn no_source(&mut self) -> bool {
        self.has_flag("no_source")
    }

    fn no_source_effect(&mut self) -> bool {
        self.has_flag("no_source_effect")
    }

    fn residual(&mut self) -> bool {
        self.has_flag("residual")
    }

    fn silent(&mut self) -> bool {
        self.has_flag("silent")
    }

    fn use_effect_as_source_effect(&mut self) -> bool {
        self.has_flag("use_effect_as_source_effect")
    }

    fn set_use_effect_as_source_effect(&mut self, val: bool) {
        self.set_flag("use_effect_as_source_effect", val)
    }

    fn use_effect_state_source(&mut self) -> bool {
        self.has_flag("use_effect_state_source")
    }

    fn use_effect_state_source_as_source(&mut self) -> bool {
        self.has_flag("use_effect_state_source_as_source")
    }

    fn use_effect_state_source_effect(&mut self) -> bool {
        self.has_flag("use_effect_state_source_effect")
    }

    fn use_effect_state_target(&mut self) -> bool {
        self.has_flag("use_effect_state_target")
    }

    fn use_effect_state_target_as_source(&mut self) -> bool {
        self.has_flag("use_effect_state_target_as_source")
    }

    fn use_source(&mut self) -> bool {
        self.has_flag("use_source")
    }

    fn use_source_effect(&mut self) -> bool {
        self.has_flag("use_source_effect")
    }

    fn set_use_source_effect(&mut self, val: bool) {
        self.set_flag("use_source_effect", val)
    }

    fn use_target_as_source(&mut self) -> bool {
        self.has_flag("use_target_as_source")
    }

    fn set_use_target_as_source(&mut self, val: bool) {
        self.set_flag("use_target_as_source", val)
    }

    fn with_source(&mut self) -> bool {
        self.has_flag("with_source")
    }

    fn set_with_source(&mut self, val: bool) {
        self.set_flag("with_source", val)
    }

    fn with_source_effect(&mut self) -> bool {
        self.has_flag("with_source_effect")
    }

    fn set_with_source_effect(&mut self, val: bool) {
        self.set_flag("with_source_effect", val)
    }

    fn with_target(&mut self) -> bool {
        self.has_flag("with_target")
    }

    fn set_with_target(&mut self, val: bool) {
        self.set_flag("with_target", val)
    }

    fn source_handle(&mut self) -> Option<MonHandle> {
        if self.no_source() {
            None
        } else if self.use_effect_state_source_as_source() {
            self.effect_state()?
                .get_mut(self.battle_context_mut())
                .ok()?
                .source()
        } else if self.use_effect_state_target_as_source() {
            self.effect_state()?
                .get_mut(self.battle_context_mut())
                .ok()?
                .target()
        } else if self.use_target_as_source() {
            self.evaluation_context().target_handle()
        } else {
            self.evaluation_context().source_handle()
        }
    }

    fn target_handle(&mut self) -> Option<MonHandle> {
        if self.use_effect_state_source() {
            self.effect_state()?
                .get_mut(self.battle_context_mut())
                .ok()?
                .source()
        } else if self.use_effect_state_target() {
            self.effect_state()?
                .get_mut(self.battle_context_mut())
                .ok()?
                .target()
        } else if self.use_source() {
            self.evaluation_context().source_handle()
        } else {
            self.evaluation_context().target_handle()
        }
    }

    fn target_handle_positional(&mut self) -> Result<MonHandle> {
        match self.front().map(|val| val.value_type()) {
            Some(ValueType::Mon) => self
                .pop_front()
                .wrap_expectation("missing target mon")?
                .mon_handle()
                .wrap_error_with_message("invalid target mon"),
            _ => self
                .target_handle()
                .wrap_expectation("effect has no target mon"),
        }
    }

    fn target_side_index(&mut self) -> Option<usize> {
        if let Some(target) = self.target_handle() {
            Some(self.evaluation_context().mon(target).ok()?.side)
        } else {
            self.evaluation_context().side_index()
        }
    }

    fn target_side_index_positional(&mut self) -> Result<usize> {
        match self.front().map(|val| val.value_type()) {
            Some(ValueType::Side) => self
                .pop_front()
                .wrap_expectation("missing target side")?
                .side_index()
                .wrap_error_with_message("invalid target side"),
            _ => self
                .target_side_index()
                .wrap_expectation("effect has no target mon"),
        }
    }

    fn effect_handle(&mut self) -> Result<EffectHandle> {
        if self.use_effect_state_source_effect() {
            self.effect_state()
                .wrap_expectation("effect has no effect state")?
                .get_mut(self.battle_context_mut())?
                .source_effect()
                .wrap_expectation("effect state has no source effect")
        } else if self.use_source_effect() {
            self.evaluation_context()
                .source_effect_handle()
                .wrap_expectation("effect has no source effect")
                .cloned()
        } else {
            Ok(self.evaluation_context().effect_handle().clone())
        }
    }

    fn effect_handle_positional(&mut self) -> Result<EffectHandle> {
        match self.front().map(|val| val.value_type()) {
            Some(ValueType::Effect) => self
                .pop_front()
                .wrap_expectation("missing effect")?
                .effect_handle()
                .wrap_error_with_message("invalid effect"),
            _ => self.effect_handle(),
        }
    }

    fn source_effect_handle(&mut self) -> Result<Option<EffectHandle>> {
        if self.no_source_effect() {
            Ok(None)
        } else if self.use_effect_as_source_effect() {
            Ok(Some(self.evaluation_context().effect_handle().clone()))
        } else if self.use_source_effect() {
            // The source effect is being used as the effect handle, so there is no source effect.
            Ok(None)
        } else {
            Ok(self.evaluation_context().source_effect_handle().cloned())
        }
    }

    fn battle_context_mut<'function>(
        &'function mut self,
    ) -> &'function mut Context<'battle, 'data> {
        self.evaluation_context_mut().battle_context_mut()
    }

    fn source_active_move_context<'function>(
        &'function mut self,
    ) -> Result<Option<ActiveMoveContext<'function, 'function, 'function, 'function, 'battle, 'data>>>
    {
        self.evaluation_context_mut().source_active_move_context()
    }

    fn mon_context<'function>(
        &'function mut self,
        mon_handle: MonHandle,
    ) -> Result<MonContext<'function, 'function, 'function, 'battle, 'data>> {
        self.evaluation_context_mut()
            .battle_context_mut()
            .mon_context(mon_handle)
    }

    fn target_context<'function>(
        &'function mut self,
    ) -> Result<MonContext<'function, 'function, 'function, 'battle, 'data>> {
        let target_handle = self
            .target_handle()
            .wrap_expectation("effect has no target")?;
        self.mon_context(target_handle)
    }

    fn effect_context<'function>(
        &'function mut self,
    ) -> Result<EffectContext<'function, 'battle, 'data>> {
        let effect_handle = self.effect_handle()?;
        let source_effect_handle = self.source_effect_handle()?;
        self.evaluation_context_mut()
            .battle_context_mut()
            .effect_context(effect_handle, source_effect_handle)
    }

    fn forward_to_applying_effect_context<'function>(
        &'function mut self,
    ) -> Result<ApplyingEffectContext<'function, 'function, 'battle, 'data>> {
        let target_handle = self
            .target_handle()
            .wrap_expectation("effect has no target")?;
        self.forward_to_applying_effect_context_with_target(target_handle)
    }

    fn forward_to_applying_effect_context_with_target<'function>(
        &'function mut self,
        target_handle: MonHandle,
    ) -> Result<ApplyingEffectContext<'function, 'function, 'battle, 'data>> {
        let source_handle = self.source_handle();
        let effect_handle = self.effect_handle()?;
        let source_effect_handle = self.source_effect_handle()?;
        self.applying_effect_context(
            effect_handle,
            source_handle,
            target_handle,
            source_effect_handle,
        )
    }

    fn forward_to_applying_effect_context_with_effect_and_target<'function>(
        &'function mut self,
        effect_handle: EffectHandle,
        target_handle: MonHandle,
    ) -> Result<ApplyingEffectContext<'function, 'function, 'battle, 'data>> {
        let source_handle = self.source_handle();
        let source_effect_handle = self.source_effect_handle()?;
        self.applying_effect_context(
            effect_handle,
            source_handle,
            target_handle,
            source_effect_handle,
        )
    }

    fn forward_to_applying_effect_context_with_source_and_target<'function>(
        &'function mut self,
        source_handle: Option<MonHandle>,
        target_handle: MonHandle,
    ) -> Result<ApplyingEffectContext<'function, 'function, 'battle, 'data>> {
        let effect_handle = self.effect_handle()?;
        let source_effect_handle = self.source_effect_handle()?;
        self.applying_effect_context(
            effect_handle,
            source_handle,
            target_handle,
            source_effect_handle,
        )
    }

    fn applying_effect_context<'function>(
        &'function mut self,
        effect_handle: EffectHandle,
        source_handle: Option<MonHandle>,
        target_handle: MonHandle,
        source_effect_handle: Option<EffectHandle>,
    ) -> Result<ApplyingEffectContext<'function, 'function, 'battle, 'data>> {
        self.evaluation_context_mut()
            .battle_context_mut()
            .applying_effect_context(
                effect_handle,
                source_handle,
                target_handle,
                source_effect_handle,
            )
    }

    fn forward_to_side_effect<'function>(
        &'function mut self,
        side: usize,
    ) -> Result<SideEffectContext<'function, 'function, 'battle, 'data>> {
        let source_handle = self.source_handle();
        let effect_handle = self.effect_handle()?;
        let source_effect_handle = self.source_effect_handle()?;
        self.evaluation_context_mut()
            .battle_context_mut()
            .side_effect_context(effect_handle, side, source_handle, source_effect_handle)
    }

    fn forward_to_field_effect<'function>(
        &'function mut self,
    ) -> Result<FieldEffectContext<'function, 'function, 'battle, 'data>> {
        let source_handle = self.source_handle();
        let effect_handle = self.effect_handle()?;
        let source_effect_handle = self.source_effect_handle()?;
        self.evaluation_context_mut()
            .battle_context_mut()
            .field_effect_context(effect_handle, source_handle, source_effect_handle)
    }
}

fn debug_log(mut context: FunctionContext) -> Result<()> {
    let mut event = battle_log_entry!("fxlang_debug");
    let mut i = 0;
    while let Some(arg) = context.pop_front() {
        event.set(format!("arg{i}"), format!("{arg:?}"));
        i += 1;
    }
    context.battle_context_mut().battle_mut().log(event);
    Ok(())
}

fn log_internal(mut context: FunctionContext, title: String) -> Result<()> {
    let mut event = UncommittedBattleLogEntry::new(title);
    while let Some(arg) = context.pop_front() {
        let entry = arg.string().wrap_error_with_message("invalid log entry")?;
        match entry.split_once(':') {
            None => event.extend(&entry.as_str()),
            Some((a, b)) => event.extend(&(a, b)),
        }
    }
    context.battle_context_mut().battle_mut().log(event);
    Ok(())
}

fn log(mut context: FunctionContext) -> Result<()> {
    let title = context
        .pop_front()
        .wrap_expectation("missing log title")?
        .string()
        .wrap_error_with_message("invalid title")?;
    log_internal(context, title)
}

#[derive(Default)]
struct LogEffectActivationBaseContext {
    include_side: bool,
    additional: Vec<String>,
}

fn log_effect_activation_base(
    mut context: FunctionContext,
    header: &str,
    activation_base_context: LogEffectActivationBaseContext,
) -> Result<()> {
    let effect = if !context.no_effect() {
        Some(context.effect_handle_positional()?)
    } else {
        None
    };
    let target = if context.with_target() {
        Some(context.target_handle_positional()?)
    } else {
        None
    };
    let mut activation = core_battle_logs::EffectActivationContext {
        effect,
        side: if activation_base_context.include_side {
            context.target_side_index()
        } else {
            None
        },
        target,
        ignore_active_move_source_effect: true,
        ignore_source_effect_equal_to_effect: true,
        source_effect: if context.with_source_effect() {
            context.set_with_source(true);
            context.source_effect_handle()?
        } else {
            None
        },
        source: if context.with_source() {
            context.source_handle()
        } else {
            None
        },
        additional: activation_base_context.additional,
        ..Default::default()
    };

    while let Some(arg) = context.pop_front() {
        let entry = arg.string().wrap_error_with_message("invalid log entry")?;
        activation.additional.push(entry);
    }
    core_battle_logs::effect_activation(context.battle_context_mut(), header.to_owned(), activation)
}

fn log_custom_effect(mut context: FunctionContext) -> Result<()> {
    let header = context
        .pop_front()
        .wrap_expectation("missing header")?
        .string()
        .wrap_error_with_message("invalid header")?;
    log_effect_activation_base(context, &header, LogEffectActivationBaseContext::default())
}

fn log_ability(mut context: FunctionContext) -> Result<()> {
    context.set_with_target(true);
    log_effect_activation_base(
        context,
        "ability",
        LogEffectActivationBaseContext::default(),
    )
}

fn log_announce_item(mut context: FunctionContext) -> Result<()> {
    let target = context.target_handle_positional()?;
    core_battle_logs::item(&mut context.forward_to_applying_effect_context_with_target(target)?)
}

fn log_activate(context: FunctionContext) -> Result<()> {
    log_effect_activation_base(
        context,
        "activate",
        LogEffectActivationBaseContext::default(),
    )
}

fn log_block(mut context: FunctionContext) -> Result<()> {
    context.set_use_source_effect(true);
    context.set_with_source_effect(true);
    context.set_use_effect_as_source_effect(true);
    context.set_use_target_as_source(true);
    log_effect_activation_base(context, "block", LogEffectActivationBaseContext::default())
}

fn log_field_activate(context: FunctionContext) -> Result<()> {
    log_effect_activation_base(
        context,
        "fieldactivate",
        LogEffectActivationBaseContext::default(),
    )
}

fn log_single_turn(context: FunctionContext) -> Result<()> {
    log_effect_activation_base(
        context,
        "singleturn",
        LogEffectActivationBaseContext::default(),
    )
}

fn log_single_move(context: FunctionContext) -> Result<()> {
    log_effect_activation_base(
        context,
        "singlemove",
        LogEffectActivationBaseContext::default(),
    )
}

fn log_animate_move(mut context: FunctionContext) -> Result<()> {
    let source_effect = context.source_effect_handle()?;
    let user_handle = context
        .pop_front()
        .wrap_expectation("missing user")?
        .mon_handle()
        .wrap_error_with_message("invalid user")?;
    let move_name = context
        .pop_front()
        .wrap_expectation("missing move name")?
        .string()
        .wrap_error_with_message("invalid move name")?;
    let target_handle = match context.pop_front() {
        Some(Value::Undefined) | None => None,
        Some(value) => Some(
            value
                .mon_handle()
                .wrap_error_with_message("invalid target")?,
        ),
    };
    core_battle_logs::use_move(
        &mut context.mon_context(user_handle)?,
        &move_name,
        target_handle,
        source_effect.as_ref(),
        true,
    )
}

fn log_start(mut context: FunctionContext) -> Result<()> {
    context.set_with_target(context.evaluation_context().target_handle().is_some());
    log_effect_activation_base(context, "start", LogEffectActivationBaseContext::default())
}

fn log_end(mut context: FunctionContext) -> Result<()> {
    context.set_with_target(context.evaluation_context().target_handle().is_some());
    log_effect_activation_base(context, "end", LogEffectActivationBaseContext::default())
}

fn log_side_start(context: FunctionContext) -> Result<()> {
    log_effect_activation_base(
        context,
        "sidestart",
        LogEffectActivationBaseContext {
            include_side: true,
            ..Default::default()
        },
    )
}

fn log_side_end(context: FunctionContext) -> Result<()> {
    log_effect_activation_base(
        context,
        "sideend",
        LogEffectActivationBaseContext {
            include_side: true,
            ..Default::default()
        },
    )
}

fn log_field_start(context: FunctionContext) -> Result<()> {
    log_effect_activation_base(
        context,
        "fieldstart",
        LogEffectActivationBaseContext::default(),
    )
}

fn log_field_end(context: FunctionContext) -> Result<()> {
    log_effect_activation_base(
        context,
        "fieldend",
        LogEffectActivationBaseContext::default(),
    )
}

fn log_prepare_move(mut context: FunctionContext) -> Result<()> {
    let target = match context.pop_front() {
        Some(value) => Some(
            value
                .mon_handle()
                .wrap_error_with_message("invalid target")?,
        ),
        None => None,
    };
    let mut context = context
        .source_active_move_context()?
        .wrap_expectation("source effect is not an active move")?;
    let mut event = battle_log_entry!(
        "prepare",
        ("mon", Mon::position_details(context.as_mon_context())?),
        ("move", context.active_move().data.name.to_owned())
    );
    if let Some(target) = target {
        event.set(
            "target",
            Mon::position_details(&mut context.as_battle_context_mut().mon_context(target)?)?,
        );
    }
    context.battle_mut().log(event);
    Ok(())
}

fn log_cant(mut context: FunctionContext) -> Result<()> {
    let effect = context.effect_handle()?;
    let source = if context.with_source() {
        context.source_handle()
    } else {
        None
    };
    core_battle_logs::cant(&mut context.target_context()?, effect, source)
}

fn log_status(mut context: FunctionContext) -> Result<()> {
    let status = context
        .pop_front()
        .wrap_expectation("missing status")?
        .string()
        .wrap_error_with_message("invalid status")?;

    context.set_no_effect(true);
    context.set_with_target(true);
    context.set_with_source_effect(true);
    log_effect_activation_base(
        context,
        "status",
        LogEffectActivationBaseContext {
            additional: Vec::from_iter([format!("status:{status}")]),
            ..Default::default()
        },
    )
}

fn log_weather(mut context: FunctionContext) -> Result<()> {
    let (title, mut additional) = match context.pop_front() {
        Some(value) => (
            "weather",
            Vec::from_iter([format!(
                "weather:{}",
                value.string().wrap_error_with_message("invalid weather")?
            )]),
        ),
        None => ("clearweather", Vec::default()),
    };
    if context.residual() {
        additional.push("residual".to_owned());
    }

    context.set_no_effect(true);
    log_effect_activation_base(
        context,
        title,
        LogEffectActivationBaseContext {
            additional,
            ..Default::default()
        },
    )
}

fn log_fail(mut context: FunctionContext) -> Result<()> {
    let effect_handle = if context.from_effect() {
        Some(context.effect_handle()?)
    } else {
        None
    };
    let mon_handle = context.target_handle_positional()?;
    let what = match context.pop_front() {
        Some(what) => Some(
            what.effect_handle()
                .wrap_error_with_message("invalid effect")?,
        ),
        None => None,
    };
    core_battle_logs::fail(&mut context.mon_context(mon_handle)?, what, effect_handle)
}

fn log_fail_unboost(mut context: FunctionContext) -> Result<()> {
    let effect_handle = if context.from_effect() {
        Some(context.effect_handle()?)
    } else {
        None
    };
    let mon_handle = context.target_handle_positional()?;
    let mut boosts = Vec::new();
    while let Some(val) = context.pop_front() {
        if val.is_list() {
            for val in val.list()? {
                boosts.push(
                    val.boost()
                        .wrap_error_with_message("invalid boost in list")?,
                );
            }
        } else {
            boosts.push(val.boost().wrap_error_with_message("invalid boost")?);
        }
    }
    core_battle_logs::fail_unboost(
        &mut context.mon_context(mon_handle)?,
        &boosts,
        effect_handle,
    )
}

fn log_immune(mut context: FunctionContext) -> Result<()> {
    let mon_handle = context.target_handle_positional()?;
    let effect = if context.from_effect() {
        Some(context.effect_handle()?)
    } else {
        None
    };
    core_battle_logs::immune(&mut context.mon_context(mon_handle)?, effect)
}

fn log_fail_heal(mut context: FunctionContext) -> Result<()> {
    let mon_handle = context.target_handle_positional()?;
    core_battle_logs::fail_heal(&mut context.mon_context(mon_handle)?)
}

fn log_ohko(mut context: FunctionContext) -> Result<()> {
    let mon_handle = context.target_handle_positional()?;
    core_battle_logs::ohko(&mut context.mon_context(mon_handle)?)
}

fn random(mut context: FunctionContext) -> Result<Value> {
    let a = context
        .pop_front()
        .map(|val| val.integer_u64().ok())
        .flatten();
    let b = context
        .pop_front()
        .map(|val| val.integer_u64().ok())
        .flatten();
    let val = match (a, b) {
        (None, None) => context.battle_context_mut().battle_mut().prng.next(),
        (Some(max), None) => rand_util::range(
            context.battle_context_mut().battle_mut().prng.as_mut(),
            0,
            max,
        ),
        (Some(min), Some(max)) => rand_util::range(
            context.battle_context_mut().battle_mut().prng.as_mut(),
            min,
            max,
        ),
        _ => return Err(general_error("invalid random arguments")),
    };
    Ok(Value::UFraction(val.into()))
}

fn chance(mut context: FunctionContext) -> Result<Value> {
    let a = context
        .pop_front()
        .map(|val| val.integer_u64().ok())
        .flatten();
    let b = context
        .pop_front()
        .map(|val| val.integer_u64().ok())
        .flatten();
    let val = match (a, b) {
        (None, None) => return Err(general_error("chance requires at least one argument")),
        (Some(den), None) => rand_util::chance(
            context.battle_context_mut().battle_mut().prng.as_mut(),
            1,
            den,
        ),
        (Some(num), Some(den)) => rand_util::chance(
            context.battle_context_mut().battle_mut().prng.as_mut(),
            num,
            den,
        ),
        _ => return Err(general_error("invalid chance arguments")),
    };
    Ok(Value::Boolean(val))
}

fn sample(mut context: FunctionContext) -> Result<Option<Value>> {
    let list = context
        .pop_front()
        .wrap_expectation("missing list")?
        .list()
        .wrap_error_with_message("invalid list")?;
    Ok(rand_util::sample_slice(
        context.battle_context_mut().battle_mut().prng.as_mut(),
        list.as_slice(),
    )
    .cloned())
}

fn damage(mut context: FunctionContext) -> Result<Value> {
    let target_handle = context.target_handle_positional()?;

    let amount = context
        .pop_front()
        .wrap_expectation("missing damage amount")?
        .integer_u16()
        .wrap_error_with_message("invalid damage amount")?;

    let damaging_effect = context.effect_handle_positional()?;

    core_battle_actions::damage(
        &mut context.forward_to_applying_effect_context_with_effect_and_target(
            damaging_effect,
            target_handle,
        )?,
        amount,
    )
    .map(|damage| Value::UFraction(damage.into()))
}

fn direct_damage(mut context: FunctionContext) -> Result<()> {
    let source_handle = context.source_handle();

    let target_handle = context.target_handle_positional()?;

    let amount = context
        .pop_front()
        .wrap_expectation("missing damage amount")?
        .integer_u16()
        .wrap_error_with_message("invalid damage amount")?;

    let damaging_effect = context.effect_handle_positional()?;

    core_battle_actions::direct_damage(
        &mut context.mon_context(target_handle)?,
        amount,
        source_handle,
        Some(&damaging_effect),
    )?;
    Ok(())
}

fn has_ability(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;

    let abilities = context
        .pop_front()
        .wrap_expectation("missing ability or abilities")?;
    let abilities = if abilities.is_list() {
        abilities
            .list()
            .wrap_error_with_message("invalid ability list")?
            .into_iter()
            .map(|val| Ok(Id::from(val.string()?)))
            .collect::<Result<Vec<_>>>()
            .wrap_error_with_message("invalid ability list")?
    } else {
        Vec::from_iter([Id::from(
            abilities
                .string()
                .wrap_error_with_message("invalid ability")?,
        )])
    };
    let mut context = context.mon_context(mon_handle)?;
    Ok(Value::Boolean(
        abilities
            .into_iter()
            .any(|ability| Mon::has_ability(&mut context, &ability)),
    ))
}

fn has_item(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    let item = context
        .pop_front()
        .wrap_expectation("missing item id")?
        .string()
        .map(|ability| Id::from(ability))
        .wrap_error_with_message("invalid item id")?;
    Ok(Value::Boolean(Mon::has_item(
        &mut context.mon_context(mon_handle)?,
        &item,
    )))
}

fn has_volatile(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    let volatile = context
        .pop_front()
        .wrap_expectation("missing volatile id")?
        .string()
        .map(|ability| Id::from(ability))
        .wrap_error_with_message("invalid volatile id")?;
    Ok(Value::Boolean(Mon::has_volatile(
        &mut context.mon_context(mon_handle)?,
        &volatile,
    )))
}

fn cure_status(mut context: FunctionContext) -> Result<Value> {
    let silent = context.silent();
    let no_effect = context.no_effect();
    let mon_handle = context.target_handle_positional()?;
    let mut context = context.forward_to_applying_effect_context_with_target(mon_handle)?;
    core_battle_actions::cure_status(&mut context, silent, !no_effect)
        .map(|val| Value::Boolean(val))
}

fn move_has_flag(mut context: FunctionContext) -> Result<Value> {
    let move_id = context
        .pop_front()
        .wrap_expectation("missing move")?
        .move_id(context.evaluation_context_mut())
        .wrap_error_with_message("invalid move")?;
    let move_flag = context
        .pop_front()
        .wrap_expectation("missing move flag")?
        .string()
        .wrap_error_with_message("invalid move flag")?;
    let move_flag = MoveFlag::from_str(&move_flag).map_err(general_error)?;
    Ok(Value::Boolean(
        context
            .evaluation_context()
            .battle_context()
            .battle()
            .dex
            .moves
            .get_by_id(&move_id)?
            .data
            .flags
            .contains(&move_flag),
    ))
}

fn item_has_flag(mut context: FunctionContext) -> Result<Value> {
    let item_id = context
        .pop_front()
        .wrap_expectation("missing item")?
        .item_id()
        .wrap_error_with_message("invalid item")?;
    let item_flag = context
        .pop_front()
        .wrap_expectation("missing item flag")?
        .string()
        .wrap_error_with_message("invalid item flag")?;
    let item_flag = ItemFlag::from_str(&item_flag).map_err(general_error)?;
    Ok(Value::Boolean(
        context
            .evaluation_context()
            .battle_context()
            .battle()
            .dex
            .items
            .get_by_id(&item_id)?
            .data
            .flags
            .contains(&item_flag),
    ))
}

fn ability_has_flag(mut context: FunctionContext) -> Result<Value> {
    let ability_id = context
        .pop_front()
        .wrap_expectation("missing ability")?
        .ability_id()
        .wrap_error_with_message("invalid ability")?;
    let ability_flag = context
        .pop_front()
        .wrap_expectation("missing ability flag")?
        .string()
        .wrap_error_with_message("invalid ability flag")?;
    let ability_flag = AbilityFlag::from_str(&ability_flag).map_err(general_error)?;
    Ok(Value::Boolean(
        context
            .evaluation_context()
            .battle_context()
            .battle()
            .dex
            .abilities
            .get_by_id(&ability_id)?
            .data
            .flags
            .contains(&ability_flag),
    ))
}

fn species_has_flag(mut context: FunctionContext) -> Result<Value> {
    let species_id = context
        .pop_front()
        .wrap_expectation("missing species")?
        .species_id()
        .wrap_error_with_message("invalid species")?;
    let species_flag = context
        .pop_front()
        .wrap_expectation("missing species flag")?
        .string()
        .wrap_error_with_message("invalid species flag")?;
    let species_flag = SpeciesFlag::from_str(&species_flag).map_err(general_error)?;
    Ok(Value::Boolean(
        context
            .evaluation_context()
            .battle_context()
            .battle()
            .dex
            .species
            .get_by_id(&species_id)?
            .data
            .flags
            .contains(&species_flag),
    ))
}

fn remove_move_flag(mut context: FunctionContext) -> Result<()> {
    let active_move = context
        .pop_front()
        .wrap_expectation("missing move")?
        .active_move()
        .wrap_error_with_message("invalid move")?;
    let move_flag = context
        .pop_front()
        .wrap_expectation("missing move flag")?
        .string()
        .wrap_error_with_message("invalid move flag")?;
    let move_flag = MoveFlag::from_str(&move_flag).map_err(general_error)?;
    context
        .evaluation_context_mut()
        .active_move_mut(active_move)?
        .data
        .flags
        .remove(&move_flag);
    Ok(())
}

fn add_volatile(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    let volatile = context
        .pop_front()
        .wrap_expectation("missing volatile id")?
        .string()
        .wrap_error_with_message("invalid volatile")?;
    let volatile = Id::from(volatile);
    let link_to = context.link_handle()?;

    core_battle_actions::add_volatile(
        &mut context.forward_to_applying_effect_context_with_target(mon_handle)?,
        &volatile,
        false,
        link_to.as_ref(),
    )
    .map(|val| Value::Boolean(val))
}

fn remove_volatile(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    let volatile = context
        .pop_front()
        .wrap_expectation("missing volatile id")?
        .string()
        .wrap_error_with_message("invalid volatile")?;

    let no_events = context.no_events();
    let volatile = Id::from(volatile);
    let mut context = context.forward_to_applying_effect_context_with_target(mon_handle)?;
    core_battle_actions::remove_volatile(&mut context, &volatile, no_events)
        .map(|val| Value::Boolean(val))
}

fn run_event(mut context: FunctionContext) -> Result<Value> {
    let event = context
        .pop_front()
        .wrap_expectation("missing event")?
        .string()
        .wrap_error_with_message("invalid event")?;
    let event = BattleEvent::from_str(&event).map_err(general_error)?;

    match context.evaluation_context_mut() {
        EvaluationContext::ApplyingEffect(context) => Ok(Value::Boolean(
            core_battle_effects::run_event_for_applying_effect(
                context,
                event,
                VariableInput::default(),
            ),
        )),
        EvaluationContext::PlayerEffect(context) => Ok(Value::Boolean(
            core_battle_effects::run_event_for_player_effect(
                context,
                event,
                VariableInput::default(),
            ),
        )),
        EvaluationContext::SideEffect(context) => Ok(Value::Boolean(
            core_battle_effects::run_event_for_side_effect(
                context,
                event,
                VariableInput::default(),
            ),
        )),
        EvaluationContext::FieldEffect(context) => Ok(Value::Boolean(
            core_battle_effects::run_event_for_field_effect(
                context,
                event,
                VariableInput::default(),
            ),
        )),
        EvaluationContext::Effect(_) => {
            Err(general_error("effect must have a target to run an event"))
        }
    }
}

fn run_event_for_mon(mut context: FunctionContext) -> Result<Value> {
    let event = context
        .pop_front()
        .wrap_expectation("missing event")?
        .string()
        .wrap_error_with_message("invalid event")?;
    let event = BattleEvent::from_str(&event).map_err(general_error)?;
    Ok(Value::Boolean(core_battle_effects::run_event_for_mon(
        &mut context.target_context()?,
        event,
        VariableInput::default(),
    )))
}

fn run_event_for_each_active_mon(mut context: FunctionContext) -> Result<()> {
    let event = context
        .pop_front()
        .wrap_expectation("missing event")?
        .string()
        .wrap_error_with_message("invalid event")?;
    let event = BattleEvent::from_str(&event).map_err(general_error)?;
    core_battle_effects::run_event_for_each_active_mon_with_effect(
        &mut context.effect_context()?,
        event,
    )
}

fn run_event_on_mon_ability(mut context: FunctionContext) -> Result<Option<Value>> {
    let event = context
        .pop_front()
        .wrap_expectation("missing event")?
        .string()
        .wrap_error_with_message("invalid event")?;
    let event = BattleEvent::from_str(&event).map_err(general_error)?;
    Ok(core_battle_effects::run_mon_ability_event(
        &mut context.forward_to_applying_effect_context()?,
        event,
        VariableInput::default(),
    ))
}

fn run_event_on_mon_item(mut context: FunctionContext) -> Result<Option<Value>> {
    let event = context
        .pop_front()
        .wrap_expectation("missing event")?
        .string()
        .wrap_error_with_message("invalid event")?;
    let event = BattleEvent::from_str(&event).map_err(general_error)?;
    Ok(core_battle_effects::run_mon_item_event(
        &mut context.forward_to_applying_effect_context()?,
        event,
        VariableInput::default(),
    ))
}

fn run_event_on_mon_volatile(mut context: FunctionContext) -> Result<Option<Value>> {
    let status = context
        .pop_front()
        .wrap_expectation("missing volatile")?
        .string()
        .wrap_error_with_message("invalid volatile")?;
    let status = Id::from(status);
    let event = context
        .pop_front()
        .wrap_expectation("missing event")?
        .string()
        .wrap_error_with_message("invalid event")?;
    let event = BattleEvent::from_str(&event).map_err(general_error)?;
    Ok(core_battle_effects::run_mon_volatile_event(
        &mut context.forward_to_applying_effect_context()?,
        event,
        VariableInput::default(),
        &status,
    ))
}

fn run_event_on_move(mut context: FunctionContext) -> Result<Option<Value>> {
    let event = context
        .pop_front()
        .wrap_expectation("missing event")?
        .string()
        .wrap_error_with_message("invalid event")?;
    let event = BattleEvent::from_str(&event).map_err(general_error)?;

    let target_handle = context.target_handle();
    let source_handle = context.source_handle();
    let side_index = context.target_side_index();

    let mut context = context
        .source_active_move_context()?
        .wrap_expectation("source effect is not an active move")?;

    let user_handle = context.mon_handle();

    let target = match (target_handle, side_index) {
        (Some(target_handle), _) => {
            if target_handle == user_handle {
                core_battle_effects::MoveTargetForEvent::UserWithTarget(source_handle)
            } else {
                core_battle_effects::MoveTargetForEvent::Mon(target_handle)
            }
        }
        (None, Some(side_index)) => core_battle_effects::MoveTargetForEvent::Side(side_index),
        (None, None) => core_battle_effects::MoveTargetForEvent::None,
    };

    Ok(core_battle_effects::run_active_move_event(
        &mut context,
        event,
        target,
        VariableInput::default(),
    ))
}

fn do_not_animate_last_move(mut context: FunctionContext) -> Result<()> {
    core_battle_logs::do_not_animate_last_move(context.battle_context_mut());
    Ok(())
}

fn calculate_damage(mut context: FunctionContext) -> Result<Value> {
    let target_handle = context.target_handle_positional()?;
    let mut context = context
        .source_active_move_context()?
        .wrap_expectation("source effect is not an active move")?;
    match core_battle_actions::calculate_damage(&mut context.target_context(target_handle)?)? {
        MoveOutcomeOnTarget::Damage(damage) => Ok(Value::UFraction(damage.into())),
        MoveOutcomeOnTarget::Success | MoveOutcomeOnTarget::Unknown => {
            Ok(Value::UFraction(0u64.into()))
        }
        _ => Ok(Value::Boolean(false)),
    }
}

fn calculate_confusion_damage(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    let base_power = context
        .pop_front()
        .wrap_expectation("missing base power")?
        .integer_u32()
        .wrap_error_with_message("invalid base power")?;
    core_battle_actions::calculate_confusion_damage(
        &mut context.mon_context(mon_handle)?,
        base_power,
    )
    .map(|value| Value::UFraction(value.into()))
}

fn max(mut context: FunctionContext) -> Result<Value> {
    let mut first = context
        .pop_front()
        .wrap_expectation("max requires at least one argument")?;
    while let Some(second) = context.pop_front() {
        if MaybeReferenceValueForOperation::from(&second)
            .greater_than(MaybeReferenceValueForOperation::from(&first))?
            .boolean()
            .unwrap_or(false)
        {
            first = second;
        }
    }
    Ok(first)
}

fn min(mut context: FunctionContext) -> Result<Value> {
    let mut first = context
        .pop_front()
        .wrap_expectation("min requires at least one argument")?;
    while let Some(second) = context.pop_front() {
        if MaybeReferenceValueForOperation::from(&second)
            .less_than(MaybeReferenceValueForOperation::from(&first))?
            .boolean()
            .unwrap_or(false)
        {
            first = second;
        }
    }
    Ok(first)
}

fn floor(mut context: FunctionContext) -> Result<Value> {
    let value = match context.pop_front().wrap_expectation("missing number")? {
        Value::Fraction(number) => Value::Fraction(number.floor().into()),
        Value::UFraction(number) => Value::UFraction(number.floor().into()),
        _ => {
            return Err(general_error("invalid number"));
        }
    };
    Ok(value)
}

fn clamp_number(mut context: FunctionContext) -> Result<Value> {
    let number = context.pop_front().wrap_expectation("missing number")?;
    let min = context.pop_front().wrap_expectation("missing minimum")?;
    let max = context.pop_front().wrap_expectation("missing maximum")?;

    if MaybeReferenceValueForOperation::from(&min)
        .greater_than_or_equal(MaybeReferenceValueForOperation::from(&max))?
        .boolean()
        .unwrap_or(false)
    {
        return Err(general_error("invalid range"));
    }

    if MaybeReferenceValueForOperation::from(&number)
        .greater_than(MaybeReferenceValueForOperation::from(&max))?
        .boolean()
        .unwrap_or(false)
    {
        Ok(max)
    } else if MaybeReferenceValueForOperation::from(&number)
        .less_than(MaybeReferenceValueForOperation::from(&min))?
        .boolean()
        .unwrap_or(false)
    {
        Ok(min)
    } else {
        Ok(number)
    }
}

fn heal(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    let damage = context
        .pop_front()
        .wrap_expectation("missing damage")?
        .integer_u16()
        .wrap_error_with_message("invalid damage")?;
    core_battle_actions::heal(
        &mut context.forward_to_applying_effect_context_with_target(mon_handle)?,
        damage,
    )
    .map(|val| Value::UFraction(val.into()))
}

fn revive(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    let hp = context
        .pop_front()
        .wrap_expectation("missing hp")?
        .integer_u16()
        .wrap_error_with_message("invalid hp")?;
    core_battle_actions::revive(
        &mut context.forward_to_applying_effect_context_with_target(mon_handle)?,
        hp,
    )
    .map(|val| Value::UFraction(val.into()))
}

fn apply_drain(mut context: FunctionContext) -> Result<()> {
    let target_handle = context.target_handle_positional()?;
    let source_handle = context
        .pop_front()
        .wrap_expectation("missing source")?
        .mon_handle()
        .wrap_error_with_message("invalid source")?;
    let damage = context
        .pop_front()
        .wrap_expectation("missing damage")?
        .integer_u16()
        .wrap_error_with_message("invalid damage")?;

    core_battle_actions::apply_drain(
        &mut context.forward_to_applying_effect_context_with_source_and_target(
            Some(source_handle),
            target_handle,
        )?,
        damage,
    )
}

fn apply_recoil_damage(mut context: FunctionContext) -> Result<()> {
    let damage = context
        .pop_front()
        .wrap_expectation("missing damage")?
        .integer_u64()
        .wrap_error_with_message("invalid damage")?;
    let mut context = context
        .source_active_move_context()?
        .wrap_expectation("source effect is not an active move")?;
    core_battle_actions::apply_recoil_damage(&mut context, damage)
}

fn set_status(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    let status = context
        .pop_front()
        .wrap_expectation("missing status id")?
        .string()
        .wrap_error_with_message("invalid status")?;
    let status = Id::from(status);

    core_battle_actions::try_set_status(
        &mut context.forward_to_applying_effect_context_with_target(mon_handle)?,
        status,
        false,
    )
    .map(|val| Value::Boolean(val.success()))
}

fn is_ally(mut context: FunctionContext) -> Result<Value> {
    let left_mon_handle = context
        .pop_front()
        .wrap_expectation("missing first mon")?
        .mon_handle()
        .wrap_error_with_message("invalid first mon")?;
    let right_mon_handle = context
        .pop_front()
        .wrap_expectation("missing second mon")?
        .mon_handle()
        .wrap_error_with_message("invalid second mon")?;
    Ok(Value::Boolean(
        context
            .evaluation_context()
            .mon(left_mon_handle)?
            .is_ally(context.evaluation_context().mon(right_mon_handle)?),
    ))
}

fn boostable_stats() -> Value {
    Value::List(Vec::from_iter(
        BoostOrderIterator::new().map(|boost| Value::Boost(boost)),
    ))
}

fn get_boost(mut context: FunctionContext) -> Result<Value> {
    let boosts = context
        .pop_front()
        .wrap_expectation("missing boosts")?
        .boost_table()
        .wrap_error_with_message("invalid boosts")?;
    let boost = context
        .pop_front()
        .wrap_expectation("missing boost")?
        .boost()
        .wrap_error_with_message("invalid boost")?;
    Ok(Value::Fraction(boosts.get(boost).into()))
}

fn set_boost(mut context: FunctionContext) -> Result<Value> {
    let mut boosts = context
        .pop_front()
        .wrap_expectation("missing boosts")?
        .boost_table()
        .wrap_error_with_message("invalid boosts")?;
    let boost = context
        .pop_front()
        .wrap_expectation("missing boost")?
        .boost()
        .wrap_error_with_message("invalid boost")?;
    let value = context
        .pop_front()
        .wrap_expectation("missing boost value")?
        .integer_i8()
        .wrap_error_with_message("invalid boost value")?;
    boosts.set(boost, value);
    Ok(Value::BoostTable(boosts))
}

fn has_type(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    let typ = context
        .pop_front()
        .wrap_expectation("missing type")?
        .mon_type()
        .wrap_error_with_message("invalid type")?;
    Ok(Value::Boolean(Mon::has_type(
        &mut context.mon_context(mon_handle)?,
        typ,
    )))
}

fn has_type_before_forced_types(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    let typ = context
        .pop_front()
        .wrap_expectation("missing type")?
        .mon_type()
        .wrap_error_with_message("invalid type")?;
    Ok(Value::Boolean(Mon::has_type_before_forced_types(
        &mut context.mon_context(mon_handle)?,
        typ,
    )))
}

fn mon_in_position(mut context: FunctionContext) -> Result<Option<Value>> {
    let side_index = context.target_side_index_positional()?;
    let position = context
        .pop_front()
        .wrap_expectation("missing position")?
        .integer_usize()
        .wrap_error_with_message("invalid position")?;
    Ok(Side::mon_in_position(
        &mut context.battle_context_mut().side_context(side_index)?,
        position,
    )?
    .map(|mon| Value::Mon(mon)))
}

fn disable_move(mut context: FunctionContext) -> Result<()> {
    let mon_handle = context.target_handle_positional()?;
    let move_id = context
        .pop_front()
        .wrap_expectation("missing move")?
        .string()
        .wrap_error_with_message("invalid move")?;
    let move_id = Id::from(move_id);
    Mon::disable_move(&mut context.mon_context(mon_handle)?, &move_id)
}

fn volatile_status_state(mut context: FunctionContext) -> Result<Option<Value>> {
    let mon_handle = context.target_handle_positional()?;
    let volatile_id = context
        .pop_front()
        .wrap_expectation("missing volatile")?
        .string()
        .wrap_error_with_message("invalid volatile")?;
    let volatile_id = Id::from(volatile_id);
    let effect_state = MonVolatileStatusEffectStateConnector::new(mon_handle, volatile_id);
    if effect_state.exists(context.battle_context_mut())? {
        Ok(Some(Value::EffectState(effect_state.make_dynamic())))
    } else {
        Ok(None)
    }
}

fn status_effect_state(mut context: FunctionContext) -> Result<Option<Value>> {
    let mon_handle = context.target_handle_positional()?;
    let effect_state = MonStatusEffectStateConnector::new(mon_handle);
    if effect_state.exists(context.battle_context_mut())? {
        Ok(Some(Value::EffectState(effect_state.make_dynamic())))
    } else {
        Ok(None)
    }
}

fn ability_effect_state(mut context: FunctionContext) -> Result<Option<Value>> {
    let mon_handle = context.target_handle_positional()?;
    let effect_state = MonAbilityEffectStateConnector::new(mon_handle);
    if effect_state.exists(context.battle_context_mut())? {
        Ok(Some(Value::EffectState(effect_state.make_dynamic())))
    } else {
        Ok(None)
    }
}

fn side_condition_effect_state(mut context: FunctionContext) -> Result<Option<Value>> {
    let side = context.target_side_index_positional()?;
    let condition_id = context
        .pop_front()
        .wrap_expectation("missing condition")?
        .string()
        .wrap_error_with_message("invalid condition")?;
    let condition_id = Id::from(condition_id);
    let effect_state = SideConditionEffectStateConnector::new(side, condition_id);
    if effect_state.exists(context.battle_context_mut())? {
        Ok(Some(Value::EffectState(effect_state.make_dynamic())))
    } else {
        Ok(None)
    }
}

struct StatBoost(Boost, i8);

impl FromStr for StatBoost {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (boost, amount) = s.split_once(':').wrap_expectation("invalid boost")?;
        let boost = Boost::from_str(boost).map_err(general_error)?;
        let amount =
            i8::from_str_radix(amount, 10).wrap_error_with_message("invalid boost amount")?;
        Ok(Self(boost, amount))
    }
}

fn can_boost(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    let boosts = context.boosts_from_rest_of_args()?;

    core_battle_actions::can_boost(&mut context.mon_context(mon_handle)?, boosts)
        .map(|val| Value::Boolean(val))
}

fn boost(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;

    // Create the context early to parse flags.
    context.forward_to_applying_effect_context_with_target(mon_handle)?;

    let boosts = context.boosts_from_rest_of_args()?;

    core_battle_actions::boost(
        &mut context.forward_to_applying_effect_context_with_target(mon_handle)?,
        boosts,
        false,
        false,
    )
    .map(|val| Value::Boolean(val))
}

fn boost_table(mut context: FunctionContext) -> Result<Value> {
    Ok(Value::BoostTable(context.boosts_from_rest_of_args()?))
}

fn can_switch(mut context: FunctionContext) -> Result<Value> {
    let player_index = context
        .pop_front()
        .wrap_expectation("missing player")?
        .player_index()
        .wrap_error_with_message("invalid player")?;
    Ok(Value::Boolean(Player::can_switch(
        &mut context.battle_context_mut().player_context(player_index)?,
    )))
}

fn has_move(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    let move_id = context
        .pop_front()
        .wrap_expectation("missing move")?
        .string()
        .wrap_error_with_message("invalid move")?;
    Ok(Value::Boolean(
        context
            .evaluation_context()
            .mon(mon_handle)?
            .move_slot_index(&Id::from(move_id))
            .is_some(),
    ))
}

fn move_slot_index(mut context: FunctionContext) -> Result<Option<Value>> {
    let mon_handle = context.target_handle_positional()?;
    let move_id = context
        .pop_front()
        .wrap_expectation("missing move")?
        .string()
        .wrap_error_with_message("invalid move")?;
    match context
        .evaluation_context()
        .mon(mon_handle)?
        .move_slot_index(&Id::from(move_id))
    {
        Some(index) => Ok(Some(Value::UFraction(
            TryInto::<u64>::try_into(index)
                .wrap_error_with_message("integer overflow")?
                .into(),
        ))),
        None => Ok(None),
    }
}

fn move_slot_at_index(mut context: FunctionContext) -> Result<Option<Value>> {
    let mon_handle = context.target_handle_positional()?;
    let index = context
        .pop_front()
        .wrap_expectation("missing index")?
        .integer_usize()
        .wrap_error_with_message("invalid index")?;
    Ok(context
        .evaluation_context()
        .mon(mon_handle)?
        .volatile_state
        .move_slots
        .get(index)
        .cloned()
        .map(|move_slot| Value::MoveSlot(move_slot)))
}

fn move_slot(mut context: FunctionContext) -> Result<Value> {
    let active_move_handle = context
        .pop_front()
        .wrap_expectation("missing active move")?
        .active_move()
        .wrap_error_with_message("invalid active move")?;
    let active_move = context
        .evaluation_context()
        .active_move(active_move_handle)?;
    let move_slot = MoveSlot::new_simulated(
        active_move.id().clone(),
        active_move.data.name.clone(),
        active_move.data.pp,
        active_move.data.pp,
        active_move.data.target,
    );
    Ok(Value::MoveSlot(move_slot))
}

fn overwrite_move_slot(mut context: FunctionContext) -> Result<()> {
    let mon_handle = context.target_handle_positional()?;
    let index = context
        .pop_front()
        .wrap_expectation("missing index")?
        .integer_usize()
        .wrap_error_with_message("invalid index")?;
    let move_slot = context
        .pop_front()
        .wrap_expectation("missing move slot")?
        .move_slot()
        .wrap_error_with_message("invalid move slot")?;

    let override_base_slot = context.has_flag("override_base_slot");

    context
        .mon_context(mon_handle)?
        .mon_mut()
        .overwrite_move_slot(index, move_slot, override_base_slot)
}

fn move_hit_target(mut context: FunctionContext) -> Result<Value> {
    let active_move_handle = context
        .pop_front()
        .wrap_expectation("missing active move")?
        .active_move()
        .wrap_error_with_message("invalid active move")?;
    let mon_handle = context.target_handle_positional()?;
    Ok(Value::Boolean(
        context
            .evaluation_context()
            .active_move(active_move_handle)?
            .hit_data(mon_handle)
            .is_some(),
    ))
}

fn move_crit_target(mut context: FunctionContext) -> Result<Value> {
    let active_move_handle = context
        .pop_front()
        .wrap_expectation("missing active move")?
        .active_move()
        .wrap_error_with_message("invalid active move")?;
    let mon_handle = context.target_handle_positional()?;
    Ok(Value::Boolean(
        context
            .evaluation_context()
            .active_move(active_move_handle)?
            .hit_data(mon_handle)
            .map(|hit_data| hit_data.crit)
            .unwrap_or(false),
    ))
}

fn type_modifier_against_target(mut context: FunctionContext) -> Result<Option<Value>> {
    let active_move_handle = context
        .pop_front()
        .wrap_expectation("missing active move")?
        .active_move()
        .wrap_error_with_message("invalid active move")?;
    let mon_handle = context.target_handle_positional()?;
    Ok(context
        .evaluation_context()
        .active_move(active_move_handle)?
        .hit_data(mon_handle)
        .map(|hit_data| Value::Fraction(hit_data.type_modifier.into())))
}

fn save_move_hit_data_flag_against_target(mut context: FunctionContext) -> Result<()> {
    let active_move_handle = context
        .pop_front()
        .wrap_expectation("missing active move")?
        .active_move()
        .wrap_error_with_message("invalid active move")?;
    let mon_handle = context.target_handle_positional()?;
    let flag = context
        .pop_front()
        .wrap_expectation("missing flag")?
        .string()
        .wrap_error_with_message("invalid flag")?;
    let flag = Id::from(flag);
    context
        .evaluation_context_mut()
        .active_move_mut(active_move_handle)?
        .hit_data_mut(mon_handle)
        .flags
        .insert(flag);
    Ok(())
}

fn move_hit_data_has_flag_against_target(mut context: FunctionContext) -> Result<Value> {
    let active_move_handle = context
        .pop_front()
        .wrap_expectation("missing active move")?
        .active_move()
        .wrap_error_with_message("invalid active move")?;
    let mon_handle = context.target_handle_positional()?;
    let flag = context
        .pop_front()
        .wrap_expectation("missing flag")?
        .string()
        .wrap_error_with_message("invalid flag")?;
    let flag = Id::from(flag);
    Ok(Value::Boolean(
        context
            .evaluation_context()
            .active_move(active_move_handle)?
            .hit_data(mon_handle)
            .map(|hit_data| hit_data.flags.contains(&flag))
            .unwrap_or(false),
    ))
}

fn all_active_mons(context: FunctionContext) -> Result<Value> {
    Ok(Value::List(
        context
            .evaluation_context()
            .battle_context()
            .battle()
            .all_active_mon_handles()
            .map(|mon_handle| Value::Mon(mon_handle))
            .collect(),
    ))
}

fn all_active_mons_in_speed_order(mut context: FunctionContext) -> Result<Value> {
    Ok(Value::List(
        CoreBattle::all_active_mon_handles_in_speed_order(context.battle_context_mut())?
            .into_iter()
            .map(|mon_handle| Value::Mon(mon_handle))
            .collect(),
    ))
}

fn all_active_mons_on_side(mut context: FunctionContext) -> Result<Value> {
    let side = context.target_side_index_positional()?;
    Ok(Value::List(
        context
            .evaluation_context()
            .battle_context()
            .battle()
            .active_mon_handles_on_side(side)
            .map(|mon_handle| Value::Mon(mon_handle))
            .collect(),
    ))
}

fn all_mons_on_side(mut context: FunctionContext) -> Result<Value> {
    let side = context.target_side_index_positional()?;
    Ok(Value::List(
        context
            .evaluation_context()
            .battle_context()
            .battle()
            .all_mon_handles_on_side(side)
            .map(|mon_handle| Value::Mon(mon_handle))
            .collect(),
    ))
}

fn all_mons_in_party(mut context: FunctionContext) -> Result<Value> {
    let player = context
        .pop_front()
        .wrap_expectation("missing player")?
        .player_index()
        .wrap_error_with_message("invalid player")?;
    Ok(Value::List(
        context
            .evaluation_context()
            .battle_context()
            .battle()
            .player(player)?
            .mon_handles()
            .map(|mon_handle| Value::Mon(*mon_handle))
            .collect(),
    ))
}

fn adjacent_foes(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    Ok(Value::List(
        Mon::adjacent_foes(&mut context.mon_context(mon_handle)?)?
            .map(|mon| Value::Mon(mon))
            .collect(),
    ))
}

fn all_foes(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    Ok(Value::List(
        Mon::active_foes(&mut context.mon_context(mon_handle)?)
            .map(|mon| Value::Mon(mon))
            .collect(),
    ))
}

fn allies_and_self(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    Ok(Value::List(
        Mon::active_allies_and_self(&mut context.mon_context(mon_handle)?)
            .map(|mon| Value::Mon(mon))
            .collect(),
    ))
}

fn clear_boosts(mut context: FunctionContext) -> Result<()> {
    let mon_handle = context.target_handle_positional()?;
    context.mon_context(mon_handle)?.mon_mut().clear_boosts();
    Ok(())
}

fn clear_negative_boosts(mut context: FunctionContext) -> Result<()> {
    let mon_handle = context.target_handle_positional()?;
    core_battle_actions::clear_negative_boosts(
        &mut context.forward_to_applying_effect_context_with_target(mon_handle)?,
    )
}

fn random_target(mut context: FunctionContext) -> Result<Option<Value>> {
    let mon_handle = context.target_handle_positional()?;
    let move_target = match context.pop_front() {
        Some(value) => value
            .move_target()
            .wrap_error_with_message("invalid move target")?,
        None => MoveTarget::Normal,
    };
    Ok(
        CoreBattle::random_target(context.battle_context_mut(), mon_handle, move_target)?
            .map(|mon| Value::Mon(mon)),
    )
}

fn clone_active_move(mut context: FunctionContext) -> Result<Value> {
    let active_move = context
        .pop_front()
        .wrap_expectation("missing move")?
        .active_move()
        .wrap_error_with_message("invalid move")?;
    let active_move = context
        .evaluation_context()
        .active_move(active_move)?
        .clone_for_battle();
    let mon_handle = context
        .pop_front()
        .wrap_expectation("missing user")?
        .mon_handle()
        .wrap_error_with_message("invalid user")?;
    let active_move_handle =
        CoreBattle::register_active_move(context.battle_context_mut(), active_move, mon_handle)?;
    Ok(Value::ActiveMove(active_move_handle))
}

fn new_active_move(mut context: FunctionContext) -> Result<Value> {
    let move_id = context
        .pop_front()
        .wrap_expectation("missing move")?
        .move_id(context.evaluation_context_mut())
        .wrap_error_with_message("invalid move")?;
    let mon_handle = context
        .pop_front()
        .wrap_expectation("missing user")?
        .mon_handle()
        .wrap_error_with_message("invalid user")?;
    let active_move_handle =
        CoreBattle::register_active_move_by_id(context.battle_context_mut(), &move_id, mon_handle)?;
    Ok(Value::ActiveMove(active_move_handle))
}

fn new_active_move_from_local_data(mut context: FunctionContext) -> Result<Value> {
    let effect_handle = context
        .pop_front()
        .wrap_expectation("missing effect")?
        .effect_handle()
        .wrap_error_with_message("invalid effect")?;
    let move_id = context
        .pop_front()
        .wrap_expectation("missing move")?
        .string()
        .wrap_error_with_message("invalid move")?;
    let move_id = Id::from(move_id);
    let move_data = CoreBattle::get_effect_by_handle(
        context.evaluation_context().battle_context(),
        &effect_handle,
    )?
    .fxlang_effect()
    .wrap_expectation("effect does not have local data")?
    .local_data
    .moves
    .get(&move_id)
    .wrap_expectation_with_format(format_args!(
        "move {move_id} does not exist in the effect's local data"
    ))?
    .clone();
    let active_move = Move::new_unlinked(move_id, move_data);
    let mon_handle = context
        .pop_front()
        .wrap_expectation("missing user")?
        .mon_handle()
        .wrap_error_with_message("invalid user")?;
    let active_move_handle =
        CoreBattle::register_active_move(context.battle_context_mut(), active_move, mon_handle)?;
    Ok(Value::ActiveMove(active_move_handle))
}

fn use_active_move(mut context: FunctionContext) -> Result<Value> {
    let indirect = context.has_flag("indirect");
    let mon_handle = context.target_handle_positional()?;
    let active_move_handle = context
        .pop_front()
        .wrap_expectation("missing active move")?
        .active_move()
        .wrap_error_with_message("invalid active move")?;
    let target_handle = match context.pop_front() {
        Some(value) => {
            if value.is_undefined() {
                None
            } else {
                Some(
                    value
                        .mon_handle()
                        .wrap_error_with_message("invalid target")?,
                )
            }
        }
        None => None,
    };
    let source_effect = context.source_effect_handle()?;
    core_battle_actions::use_active_move(
        &mut context.mon_context(mon_handle)?,
        active_move_handle,
        target_handle,
        source_effect.as_ref(),
        true,
        !indirect,
    )
    .map(|val| Value::Boolean(val))
}

fn use_move(mut context: FunctionContext) -> Result<Value> {
    let source_effect = context.source_effect_handle()?;
    let indirect = context.has_flag("indirect");
    let mon_handle = context.target_handle_positional()?;
    let move_id = context
        .pop_front()
        .wrap_expectation("missing move")?
        .move_id(context.evaluation_context_mut())
        .wrap_error_with_message("invalid move")?;
    let target_handle = match context.pop_front() {
        Some(value) => {
            if value.is_undefined() {
                None
            } else {
                Some(
                    value
                        .mon_handle()
                        .wrap_error_with_message("invalid target")?,
                )
            }
        }
        None => None,
    };
    core_battle_actions::use_move(
        &mut context.mon_context(mon_handle)?,
        &move_id,
        target_handle,
        source_effect.as_ref(),
        !indirect,
    )
    .map(|val| Value::Boolean(val))
}

fn do_move(mut context: FunctionContext) -> Result<()> {
    let mon_handle = context.target_handle_positional()?;
    let move_id = context
        .pop_front()
        .wrap_expectation("missing move")?
        .move_id(context.evaluation_context_mut())
        .wrap_error_with_message("invalid move")?;
    let target_position = match context.pop_front() {
        Some(value) => Some(
            value
                .integer_isize()
                .wrap_error_with_message("invalid target position")?,
        ),
        None => None,
    };
    let target = match context.pop_front() {
        Some(value) => Some(
            value
                .mon_handle()
                .wrap_error_with_message("invalid target mon")?,
        ),
        None => None,
    };
    core_battle_actions::do_move_by_id(
        &mut context.mon_context(mon_handle)?,
        &move_id,
        target_position,
        target,
    )
}

fn mon_at_target_location(mut context: FunctionContext) -> Result<Option<Value>> {
    let mon_handle = context.target_handle_positional()?;
    let relative_location = context
        .pop_front()
        .wrap_expectation("missing relative location")?
        .integer_isize()
        .wrap_error_with_message("invalid relative location")?;
    Mon::get_target(&mut context.mon_context(mon_handle)?, relative_location)
        .map(|mon| Some(Value::Mon(mon?)))
}

fn target_location_of_mon(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    let target_handle = context
        .pop_front()
        .wrap_expectation("missing target")?
        .mon_handle()
        .wrap_error_with_message("invalid target")?;
    Ok(Value::Fraction(
        TryInto::<i64>::try_into(Mon::get_target_location(
            &mut context.mon_context(mon_handle)?,
            target_handle,
        )?)
        .wrap_error_with_message("integer overflow")?
        .into(),
    ))
}

fn effect_handle_exists_or_none(
    context: FunctionContext,
    effect_handle: EffectHandle,
) -> Option<Value> {
    CoreBattle::get_effect_by_handle(
        context.evaluation_context().battle_context(),
        &effect_handle,
    )
    .is_ok()
    .then(|| Value::Effect(effect_handle))
}

fn get_move(mut context: FunctionContext) -> Result<Option<Value>> {
    let move_id = context
        .pop_front()
        .wrap_expectation("missing move id")?
        .move_id(context.evaluation_context_mut())
        .wrap_error_with_message("invalid move id")?;
    Ok(effect_handle_exists_or_none(
        context,
        EffectHandle::InactiveMove(move_id),
    ))
}

fn get_ability(mut context: FunctionContext) -> Result<Option<Value>> {
    let ability_id = context
        .pop_front()
        .wrap_expectation("missing ability id")?
        .ability_id()
        .wrap_error_with_message("invalid ability id")?;
    Ok(effect_handle_exists_or_none(
        context,
        EffectHandle::Ability(ability_id),
    ))
}

fn get_item(mut context: FunctionContext) -> Result<Option<Value>> {
    let item_id = context
        .pop_front()
        .wrap_expectation("missing item id")?
        .item_id()
        .wrap_error_with_message("invalid item id")?;
    Ok(effect_handle_exists_or_none(
        context,
        EffectHandle::Item(item_id),
    ))
}

fn get_species(mut context: FunctionContext) -> Result<Option<Value>> {
    let species_id = context
        .pop_front()
        .wrap_expectation("missing item id")?
        .species_id()
        .wrap_error_with_message("invalid species id")?;
    Ok(effect_handle_exists_or_none(
        context,
        EffectHandle::Species(species_id),
    ))
}

fn get_all_moves(mut context: FunctionContext) -> Result<Value> {
    let mut with_flags = HashSet::default();
    let mut without_flags = HashSet::default();
    while let Some(arg) = context.pop_front() {
        match arg
            .string()
            .wrap_error_with_message("invalid filter")?
            .split_once(':')
        {
            Some(("with_flag", flag)) => {
                with_flags.insert(MoveFlag::from_str(flag).map_err(general_error)?)
            }
            Some(("without_flag", flag)) => {
                without_flags.insert(MoveFlag::from_str(flag).map_err(general_error)?)
            }
            _ => return Err(general_error("invalid filter")),
        };
    }

    let mut moves = context
        .evaluation_context()
        .battle_context()
        .battle()
        .dex
        .all_move_ids(&|move_data| {
            with_flags.is_subset(&move_data.flags)
                && without_flags.intersection(&move_data.flags).count() == 0
        })?;
    // This sort must be stable for RNG stability.
    moves.sort();
    Ok(Value::List(
        moves
            .into_iter()
            .map(|id| Value::Effect(EffectHandle::InactiveMove(id)))
            .collect(),
    ))
}

fn move_at_move_slot_index(mut context: FunctionContext) -> Result<Option<Value>> {
    let mon_handle = context.target_handle_positional()?;
    let index = context
        .pop_front()
        .wrap_expectation("missing index")?
        .integer_usize()
        .wrap_error_with_message("invalid index")?;
    let context = context.mon_context(mon_handle)?;
    Ok(context
        .mon()
        .volatile_state
        .move_slots
        .get(index)
        .map(|move_slot| Value::Effect(EffectHandle::InactiveMove(move_slot.id.clone()))))
}

fn set_types(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    let typ = context
        .pop_front()
        .wrap_expectation("missing type")?
        .mon_type()
        .wrap_error_with_message("invalid type")?;
    let mut context = context.forward_to_applying_effect_context_with_target(mon_handle)?;
    core_battle_actions::set_types(&mut context, Vec::from_iter([typ]))
        .map(|val| Value::Boolean(val))
}

fn set_weather(mut context: FunctionContext) -> Result<Value> {
    let weather = context
        .pop_front()
        .wrap_expectation("missing weather")?
        .string()
        .wrap_error_with_message("invalid weather")?;
    let weather = Id::from(weather);
    core_battle_actions::set_weather(&mut context.forward_to_field_effect()?, &weather)
        .map(Value::Boolean)
}

fn set_terrain(mut context: FunctionContext) -> Result<Value> {
    let terrain = context
        .pop_front()
        .wrap_expectation("missing terrain")?
        .string()
        .wrap_error_with_message("invalid terrain")?;
    let terrain = Id::from(terrain);
    core_battle_actions::set_terrain(&mut context.forward_to_field_effect()?, &terrain)
        .map(Value::Boolean)
}

fn clear_weather(mut context: FunctionContext) -> Result<Value> {
    core_battle_actions::clear_weather(&mut context.forward_to_field_effect()?).map(Value::Boolean)
}

fn clear_terrain(mut context: FunctionContext) -> Result<Value> {
    core_battle_actions::clear_terrain(&mut context.forward_to_field_effect()?).map(Value::Boolean)
}

fn transform_into(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    let transform_into_handle = context
        .pop_front()
        .wrap_expectation("missing transformation target")?
        .mon_handle()
        .wrap_error_with_message("invalid transformation target")?;

    core_battle_actions::transform_into(
        &mut context.forward_to_applying_effect_context_with_target(mon_handle)?,
        transform_into_handle,
    )
    .map(|val| Value::Boolean(val))
}

fn set_illusion(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    let transform_into_handle = context
        .pop_front()
        .wrap_expectation("missing illusion target")?
        .mon_handle()
        .wrap_error_with_message("invalid illusion target")?;

    core_battle_actions::set_illusion(
        &mut context.forward_to_applying_effect_context_with_target(mon_handle)?,
        transform_into_handle,
    )
    .map(|val| Value::Boolean(val))
}

fn end_illusion(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;

    core_battle_actions::end_illusion(
        &mut context.forward_to_applying_effect_context_with_target(mon_handle)?,
    )
    .map(|val| Value::Boolean(val))
}

fn can_escape(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    Mon::can_escape(&mut context.mon_context(mon_handle)?).map(|val| Value::Boolean(val))
}

fn escape(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    core_battle_actions::try_escape(&mut context.mon_context(mon_handle)?, true)
        .map(|val| Value::Boolean(val))
}

fn hit_effect() -> Result<Value> {
    Ok(Value::HitEffect(HitEffect::default()))
}

fn secondary_hit_effect() -> Result<Value> {
    Ok(Value::SecondaryHitEffect(SecondaryEffectData::default()))
}

fn all_types(context: FunctionContext) -> Result<Value> {
    let mut types = context
        .evaluation_context()
        .battle_context()
        .battle()
        .dex
        .type_chart()
        .types
        .keys()
        .cloned()
        .collect::<Vec<_>>();
    types.sort();
    let types = types.into_iter().map(|typ| Value::Type(typ)).collect();
    Ok(Value::List(types))
}

fn type_is_weak_against(mut context: FunctionContext) -> Result<Value> {
    let offense = context
        .pop_front()
        .wrap_expectation("missing offensive type")?
        .mon_type()
        .wrap_error_with_message("invalid offensive type")?;
    let defense = context
        .pop_front()
        .wrap_expectation("missing defensive type")?
        .mon_type()
        .wrap_error_with_message("invalid defensive type")?;
    Ok(Value::Boolean(
        context
            .evaluation_context()
            .battle_context()
            .battle()
            .dex
            .type_chart()
            .types
            .get(&offense)
            .map(|types| {
                types
                    .get(&defense)
                    .is_some_and(|effectiveness| effectiveness == &TypeEffectiveness::Weak)
            })
            .unwrap_or(false),
    ))
}

fn type_has_no_effect_against(mut context: FunctionContext) -> Result<Value> {
    let offense = context
        .pop_front()
        .wrap_expectation("missing offensive type")?
        .mon_type()
        .wrap_error_with_message("invalid offensive type")?;
    let defense = context
        .pop_front()
        .wrap_expectation("missing defensive type")?
        .mon_type()
        .wrap_error_with_message("invalid defensive type")?;
    Ok(Value::Boolean(
        context
            .evaluation_context()
            .battle_context()
            .battle()
            .dex
            .type_chart()
            .types
            .get(&offense)
            .map(|types| {
                types
                    .get(&defense)
                    .is_some_and(|effectiveness| effectiveness == &TypeEffectiveness::None)
            })
            .unwrap_or(false),
    ))
}

fn append(mut context: FunctionContext) -> Result<Value> {
    let mut list = context
        .pop_front()
        .wrap_expectation("missing list")?
        .list()
        .wrap_error_with_message("invalid list")?;
    let value = context.pop_front().wrap_expectation("missing value")?;
    list.push(value);
    Ok(Value::List(list))
}

fn remove(mut context: FunctionContext) -> Result<Value> {
    let list = context
        .pop_front()
        .wrap_expectation("missing list")?
        .list()
        .wrap_error_with_message("invalid list")?;
    let value = context.pop_front().wrap_expectation("missing value")?;
    let list = list
        .into_iter()
        .filter(|element| {
            MaybeReferenceValueForOperation::from(element)
                .not_equal(MaybeReferenceValueForOperation::from(&value))
                .is_ok_and(|val| val.boolean().is_some_and(|val| val))
        })
        .collect();
    Ok(Value::List(list))
}

fn index(mut context: FunctionContext) -> Result<Option<Value>> {
    let list = context
        .pop_front()
        .wrap_expectation("missing list")?
        .list()
        .wrap_error_with_message("invalid list")?;
    let index = context
        .pop_front()
        .wrap_expectation("missing index")?
        .integer_usize()
        .wrap_error_with_message("invalid index")?;
    Ok(list.get(index).cloned())
}

fn reverse(mut context: FunctionContext) -> Result<Value> {
    let mut list = context
        .pop_front()
        .wrap_expectation("missing list")?
        .list()
        .wrap_error_with_message("invalid list")?;
    list.reverse();
    Ok(Value::List(list))
}

fn any_mon_will_move_this_turn(context: FunctionContext) -> Result<Value> {
    Ok(Value::Boolean(
        context
            .evaluation_context()
            .battle_context()
            .battle()
            .queue
            .any_move_this_turn(),
    ))
}

fn pending_move_action_this_turn(mut context: FunctionContext) -> Result<Option<Value>> {
    let mon_handle = context.target_handle_positional()?;
    Ok(context
        .evaluation_context()
        .battle_context()
        .battle()
        .queue
        .pending_move_this_turn(mon_handle)
        .map(|move_action| {
            Value::Object(HashMap::from_iter([
                ("id".to_owned(), Value::String(move_action.id.to_string())),
                (
                    "effective_id".to_owned(),
                    Value::String(move_action.effective_move_id().to_string()),
                ),
            ]))
        }))
}

fn will_move_this_turn(context: FunctionContext) -> Result<Value> {
    pending_move_action_this_turn(context).map(|val| Value::Boolean(val.is_some()))
}

fn remove_side_condition(mut context: FunctionContext) -> Result<Value> {
    let side = context.target_side_index_positional()?;
    let condition = context
        .pop_front()
        .wrap_expectation("missing side condition")?
        .string()
        .wrap_error_with_message("invalid side condition")?;
    let condition = Id::from(condition);
    Ok(Value::Boolean(core_battle_actions::remove_side_condition(
        &mut context.forward_to_side_effect(side)?,
        &condition,
    )?))
}

fn faint(mut context: FunctionContext) -> Result<()> {
    let mon_handle = context.target_handle_positional()?;
    let source = context.source_handle();
    let effect = context.effect_handle()?;
    core_battle_actions::faint(&mut context.mon_context(mon_handle)?, source, Some(&effect))
}

fn prepare_direct_move(mut context: FunctionContext) -> Result<Value> {
    let targets = context
        .pop_front()
        .wrap_expectation("missing targets list")?
        .mons_list()
        .wrap_error_with_message("invalid targets list")?;
    let mut context = context
        .source_active_move_context()?
        .wrap_expectation("context is not an active move")?;
    let targets = core_battle_actions::prepare_direct_move(&mut context, &targets)?;
    Ok(Value::List(
        targets
            .into_iter()
            .map(|target| Value::Mon(target))
            .collect(),
    ))
}

fn check_immunity(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    let effect_id = context
        .pop_front()
        .wrap_expectation("missing effect")?
        .effect_id()
        .wrap_error_with_message("invalid effect")?;
    let effect_handle = context
        .battle_context_mut()
        .battle_mut()
        .get_effect_handle_by_id(&effect_id)?
        .clone();

    core_battle_actions::check_immunity(
        &mut context
            .forward_to_applying_effect_context_with_effect_and_target(effect_handle, mon_handle)?,
    )
    .map(|val| Value::Boolean(val))
}

fn modify_move_type(mut context: FunctionContext) -> Result<()> {
    let target = context.source_handle();
    let mut context = context
        .source_active_move_context()?
        .wrap_expectation("context is not an active move")?;
    core_battle_actions::modify_move_type(&mut context, target)?;
    Ok(())
}

fn max_move(mut context: FunctionContext) -> Result<Option<Value>> {
    let target = context.target_handle_positional()?;
    let move_handle = context
        .pop_front()
        .wrap_expectation("missing move")?
        .active_move()
        .wrap_error_with_message("invalid move")?;
    let mut context = context.mon_context(target)?;
    let move_data = context
        .as_battle_context()
        .active_move(move_handle)?
        .data
        .clone();
    core_battle_actions::max_move(&mut context, &move_data)
        .map(|move_id| move_id.map(|val| Value::String(val.to_string())))
}

fn set_hp(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    let hp = context
        .pop_front()
        .wrap_expectation("missing hp")?
        .integer_u16()
        .wrap_error_with_message("invalid hp")?;
    Ok(Value::Fraction(
        Mon::set_hp(&mut context.mon_context(mon_handle)?, hp)?.into(),
    ))
}

fn deduct_pp(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    let move_id = context
        .pop_front()
        .wrap_expectation("missing move")?
        .move_id(context.evaluation_context_mut())
        .wrap_error_with_message("invalid move")?;
    let pp = context
        .pop_front()
        .wrap_expectation("missing pp")?
        .integer_u8()
        .wrap_error_with_message("invalid pp")?;
    core_battle_actions::deduct_pp(
        &mut context.forward_to_applying_effect_context_with_target(mon_handle)?,
        &move_id,
        pp,
    )
    .map(|val| Value::UFraction(val.into()))
}

fn restore_pp(mut context: FunctionContext) -> Result<Value> {
    let silent = context.silent();
    let mon_handle = context.target_handle_positional()?;
    let move_id = context
        .pop_front()
        .wrap_expectation("missing move")?
        .move_id(context.evaluation_context_mut())
        .wrap_error_with_message("invalid move")?;
    let pp = context
        .pop_front()
        .wrap_expectation("missing pp")?
        .integer_u8()
        .wrap_error_with_message("invalid pp")?;
    core_battle_actions::restore_pp(
        &mut context.forward_to_applying_effect_context_with_target(mon_handle)?,
        &move_id,
        pp,
        silent,
    )
    .map(|val| Value::UFraction(val.into()))
}

fn set_pp(mut context: FunctionContext) -> Result<Value> {
    let mon_handle = context.target_handle_positional()?;
    let move_id = context
        .pop_front()
        .wrap_expectation("missing move")?
        .move_id(context.evaluation_context_mut())
        .wrap_error_with_message("invalid move")?;
    let pp = context
        .pop_front()
        .wrap_expectation("missing pp")?
        .integer_u8()
        .wrap_error_with_message("invalid pp")?;
    core_battle_actions::set_pp(
        &mut context.forward_to_applying_effect_context_with_target(mon_handle)?,
        &move_id,
        pp,
    )
    .map(|val| Value::UFraction(val.into()))
}

fn add_slot_condition(mut context: FunctionContext) -> Result<Value> {
    let side_index = context.target_side_index_positional()?;
    let slot = context
        .pop_front()
        .wrap_expectation("missing slot")?
        .integer_usize()
        .wrap_error_with_message("invalid slot")?;
    let condition = context
        .pop_front()
        .wrap_expectation("missing condition id")?
        .string()
        .wrap_error_with_message("invalid condition id")?;
    let condition = Id::from(condition);

    let mut context = context.forward_to_side_effect(side_index)?;
    let value = core_battle_actions::add_slot_condition(&mut context, slot, &condition);
    value.map(|val| Value::Boolean(val))
}
fn remove_slot_condition(mut context: FunctionContext) -> Result<Value> {
    let side_index = context.target_side_index_positional()?;
    let slot = context
        .pop_front()
        .wrap_expectation("missing slot")?
        .integer_usize()
        .wrap_error_with_message("invalid slot")?;
    let condition = context
        .pop_front()
        .wrap_expectation("missing condition id")?
        .string()
        .wrap_error_with_message("invalid condition id")?;
    let condition = Id::from(condition);

    let mut context = context.forward_to_side_effect(side_index)?;
    let value = core_battle_actions::remove_slot_condition(&mut context, slot, &condition);
    value.map(|val| Value::Boolean(val))
}

fn add_side_condition(mut context: FunctionContext) -> Result<Value> {
    let side_index = context.target_side_index_positional()?;
    let condition = context
        .pop_front()
        .wrap_expectation("missing condition id")?
        .string()
        .wrap_error_with_message("invalid condition id")?;
    let condition = Id::from(condition);

    let mut context = context.forward_to_side_effect(side_index)?;
    let value = core_battle_actions::add_side_condition(&mut context, &condition);
    value.map(|val| Value::Boolean(val))
}

fn is_adjacent(mut context: FunctionContext) -> Result<Value> {
    let mon = context.target_handle_positional()?;
    let other = context
        .pop_front()
        .wrap_expectation("missing second mon")?
        .mon_handle()
        .wrap_error_with_message("invalid second mon")?;
    Mon::is_adjacent(&mut context.mon_context(mon)?, other).map(|val| Value::Boolean(val))
}

fn cancel_move(mut context: FunctionContext) -> Result<Value> {
    let mon = context.target_handle_positional()?;
    Ok(Value::Boolean(
        context
            .battle_context_mut()
            .battle_mut()
            .queue
            .cancel_move(mon),
    ))
}

fn take_item(mut context: FunctionContext) -> Result<Option<Value>> {
    let mon = context.target_handle_positional()?;
    let dry_run = context.has_flag("dry_run");
    let silent = context.silent();
    Ok(core_battle_actions::take_item(
        &mut context.forward_to_applying_effect_context_with_target(mon)?,
        dry_run,
        silent,
    )?
    .map(|val| Value::String(val.to_string())))
}

fn set_item(mut context: FunctionContext) -> Result<Value> {
    let mon = context.target_handle_positional()?;
    let item = context
        .pop_front()
        .wrap_expectation("missing item")?
        .string()
        .wrap_error_with_message("invalid item")?;
    let item = Id::from(item);
    let dry_run = context.has_flag("dry_run");

    Ok(Value::Boolean(core_battle_actions::set_item(
        &mut context.forward_to_applying_effect_context_with_target(mon)?,
        &item,
        dry_run,
    )?))
}

fn eat_item(mut context: FunctionContext) -> Result<Value> {
    let mon = context.target_handle_positional()?;
    core_battle_actions::eat_item(&mut context.forward_to_applying_effect_context_with_target(mon)?)
        .map(|val| Value::Boolean(val))
}

fn eat_given_item(mut context: FunctionContext) -> Result<Value> {
    let mon = context.target_handle_positional()?;
    let item = context
        .pop_front()
        .wrap_expectation("missing item")?
        .string()
        .wrap_error_with_message("invalid item")?;
    let item = Id::from(item);
    core_battle_actions::eat_given_item(
        &mut context.forward_to_applying_effect_context_with_target(mon)?,
        &item,
    )
    .map(|val| Value::Boolean(val))
}

fn use_item(mut context: FunctionContext) -> Result<Value> {
    let mon = context.target_handle_positional()?;
    core_battle_actions::use_item(&mut context.forward_to_applying_effect_context_with_target(mon)?)
        .map(|val| Value::Boolean(val))
}

fn use_given_item(mut context: FunctionContext) -> Result<Value> {
    let mon = context.target_handle_positional()?;
    let item = context
        .pop_front()
        .wrap_expectation("missing item")?
        .string()
        .wrap_error_with_message("invalid item")?;
    let item = Id::from(item);
    core_battle_actions::use_given_item(
        &mut context.forward_to_applying_effect_context_with_target(mon)?,
        &item,
    )
    .map(|val| Value::Boolean(val))
}

fn discard_item(mut context: FunctionContext) -> Result<Value> {
    let silent = context.has_flag("silent");
    let mon = context.target_handle_positional()?;
    core_battle_actions::discard_item(
        &mut context.forward_to_applying_effect_context_with_target(mon)?,
        silent,
    )
    .map(|val| Value::Boolean(val))
}

fn valid_target(mut context: FunctionContext) -> Result<Value> {
    let mon = context.target_handle_positional()?;
    let target = context
        .pop_front()
        .wrap_expectation("missing target")?
        .mon_handle()
        .wrap_error_with_message("invalid target")?;
    let move_target = context
        .pop_front()
        .wrap_expectation("missing move target")?
        .move_target()
        .wrap_error_with_message("invalid move target")?;
    let mut context = context.mon_context(mon)?;
    let target_location = Mon::get_target_location(&mut context, target)?;
    CoreBattle::valid_target(&mut context, move_target, target_location)
        .map(|val| Value::Boolean(val))
}

fn set_ability(mut context: FunctionContext) -> Result<Value> {
    let silent = context.silent();
    let dry_run = context.has_flag("dry_run");
    let mon = context.target_handle_positional()?;
    let ability_id = context
        .pop_front()
        .wrap_expectation("missing ability")?
        .ability_id()
        .wrap_error_with_message("invalid ability")?;
    core_battle_actions::set_ability(
        &mut context.forward_to_applying_effect_context_with_target(mon)?,
        &ability_id,
        dry_run,
        false,
        silent,
    )
    .map(|val| Value::Boolean(val))
}

fn received_attack(mut context: FunctionContext) -> Result<Value> {
    let has_damage = context.has_flag("has_damage");
    let this_turn = context.has_flag("this_turn");
    let target = context.target_handle_positional()?;
    let source = context
        .pop_front()
        .wrap_expectation("missing source")?
        .mon_handle()
        .wrap_error_with_message("invalid source")?;
    let turn = context
        .evaluation_context()
        .battle_context()
        .battle()
        .turn();
    Ok(Value::Boolean(
        context
            .evaluation_context()
            .mon(target)?
            .volatile_state
            .received_attacks
            .iter()
            .any(|entry| {
                entry.source == source
                    && (!has_damage || entry.damage > 0)
                    && (!this_turn || entry.turn == turn)
            }),
    ))
}

fn last_attack(mut context: FunctionContext) -> Result<Option<Value>> {
    let has_damage = context.has_flag("has_damage");
    let different_side = context.has_flag("different_side");
    let target = context.target_handle_positional()?;
    let target = context.evaluation_context().mon(target)?;
    let side = target.side;
    target
        .volatile_state
        .received_attacks
        .iter()
        .filter(|attack| !has_damage || attack.damage > 0)
        .filter(|attack| !different_side || attack.source_side != side)
        .last()
        .map(|attack| {
            Ok(Value::Object(HashMap::from_iter([
                ("damage".to_owned(), Value::UFraction(attack.damage.into())),
                ("side".to_owned(), Value::Side(attack.source_side)),
                (
                    "slot".to_owned(),
                    Value::UFraction(
                        TryInto::<u64>::try_into(attack.source_position)
                            .wrap_error_with_message("integer overflow")?
                            .into(),
                    ),
                ),
                ("source".to_owned(), Value::Mon(attack.source)),
            ])))
        })
        .transpose()
}

fn add_secondary_effect_to_move(mut context: FunctionContext) -> Result<()> {
    let active_move = context
        .pop_front()
        .wrap_expectation("missing move")?
        .active_move()
        .wrap_error_with_message("invalid move")?;
    let secondary_effect = context
        .pop_front()
        .wrap_expectation("missing target effect")?
        .secondary_hit_effect()
        .wrap_error_with_message("invalid target effect")?;
    context
        .evaluation_context_mut()
        .active_move_mut(active_move)?
        .data
        .secondary_effects
        .push(secondary_effect);
    Ok(())
}

fn move_makes_contact(mut context: FunctionContext) -> Result<Value> {
    let active_move = context
        .pop_front()
        .wrap_expectation("missing move")?
        .active_move()
        .wrap_error_with_message("invalid move")?;
    Ok(Value::Boolean(mon_states::move_makes_contact(
        &mut context
            .evaluation_context_mut()
            .active_move_context(active_move)?,
    )))
}

fn type_effectiveness(mut context: FunctionContext) -> Result<Value> {
    let target = context.target_handle_positional()?;
    core_battle_actions::type_effectiveness(
        &mut context.forward_to_applying_effect_context_with_target(target)?,
    )
    .map(|val| Value::Fraction(val.into()))
}

fn type_modifier(mut context: FunctionContext) -> Result<Value> {
    let target = context.target_handle_positional()?;
    core_battle_actions::type_modifier(
        &mut context.forward_to_applying_effect_context_with_target(target)?,
    )
    .map(|val| Value::Fraction(val.into()))
}

fn type_chart_effectiveness(mut context: FunctionContext) -> Result<Value> {
    let offense = context
        .pop_front()
        .wrap_expectation("missing offensive type")?
        .mon_type()
        .wrap_error_with_message("invalid offensive type")?;
    let defense = context
        .pop_front()
        .wrap_expectation("missing defensive types")?
        .types_list()
        .wrap_error_with_message("invalid defensive types")?;
    Ok(Value::Fraction(
        defense
            .into_iter()
            .map(|defense| {
                context
                    .evaluation_context()
                    .battle_context()
                    .battle()
                    .check_type_effectiveness(offense, defense)
            })
            .sum::<i8>()
            .into(),
    ))
}

fn type_chart_immunity(mut context: FunctionContext) -> Result<Value> {
    let offense = context
        .pop_front()
        .wrap_expectation("missing offensive type")?
        .mon_type()
        .wrap_error_with_message("invalid offensive type")?;
    let defense = context
        .pop_front()
        .wrap_expectation("missing defensive types")?
        .types_list()
        .wrap_error_with_message("invalid defensive types")?;
    Ok(Value::Boolean(
        context
            .evaluation_context()
            .battle_context()
            .battle()
            .check_type_immunity(offense, &defense),
    ))
}

fn forme_change(mut context: FunctionContext) -> Result<Value> {
    let permanent = context.has_flag("permanent");
    let target = context.target_handle_positional()?;
    let forme = context
        .pop_front()
        .wrap_expectation("missing forme")?
        .string()
        .wrap_error_with_message("invalid forme")?;
    let forme = Id::from(forme);
    core_battle_actions::forme_change(
        &mut context.forward_to_applying_effect_context_with_target(target)?,
        &forme,
        if permanent {
            core_battle_actions::FormeChangeType::Permanent
        } else {
            core_battle_actions::FormeChangeType::Temporary
        },
    )
    .map(|val| Value::Boolean(val))
}

fn primal_reversion(mut context: FunctionContext) -> Result<Value> {
    let target = context.target_handle_positional()?;
    let forme = context
        .pop_front()
        .wrap_expectation("missing forme")?
        .string()
        .wrap_error_with_message("invalid forme")?;
    let forme = Id::from(forme);
    core_battle_actions::primal_reversion(
        &mut context.forward_to_applying_effect_context_with_target(target)?,
        &forme,
    )
    .map(|val| Value::Boolean(val))
}

fn end_dynamax(mut context: FunctionContext) -> Result<()> {
    let target = context.target_handle_positional()?;
    core_battle_actions::end_dynamax(
        &mut context.forward_to_applying_effect_context_with_target(target)?,
    )
}

fn increase_friendship(mut context: FunctionContext) -> Result<()> {
    let mon_handle = context.target_handle_positional()?;
    let delta_1 = context
        .pop_front()
        .wrap_expectation("missing level 1 delta")?
        .integer_u8()
        .wrap_error_with_message("invalid level 1 delta")?;
    let delta_2 = context
        .pop_front()
        .wrap_expectation("missing level 2 delta")?
        .integer_u8()
        .wrap_error_with_message("invalid level 2 delta")?;
    let delta_3 = context
        .pop_front()
        .wrap_expectation("missing level 3 delta")?
        .integer_u8()
        .wrap_error_with_message("invalid level 3 delta")?;
    Mon::increase_friendship(
        &mut context.mon_context(mon_handle)?,
        [delta_1, delta_2, delta_3],
    );
    Ok(())
}

fn decrease_friendship(mut context: FunctionContext) -> Result<()> {
    let mon_handle = context.target_handle_positional()?;
    let delta_1 = context
        .pop_front()
        .wrap_expectation("missing level 1 delta")?
        .integer_u8()
        .wrap_error_with_message("invalid level 1 delta")?;
    let delta_2 = context
        .pop_front()
        .wrap_expectation("missing level 2 delta")?
        .integer_u8()
        .wrap_error_with_message("invalid level 2 delta")?;
    let delta_3 = context
        .pop_front()
        .wrap_expectation("missing level 3 delta")?
        .integer_u8()
        .wrap_error_with_message("invalid level 3 delta")?;
    Mon::decrease_friendship(
        &mut context.mon_context(mon_handle)?,
        [delta_1, delta_2, delta_3],
    );
    Ok(())
}

fn set_friendship(mut context: FunctionContext) -> Result<()> {
    let mon_handle = context.target_handle_positional()?;
    let friendship = context
        .pop_front()
        .wrap_expectation("missing friendship")?
        .integer_u8()
        .wrap_error_with_message("invalid friendship")?;
    Mon::set_friendship(&mut context.mon_context(mon_handle)?, friendship);
    Ok(())
}

fn has_side_condition(mut context: FunctionContext) -> Result<Value> {
    let side_index = context.target_side_index_positional()?;
    let condition = context
        .pop_front()
        .wrap_expectation("missing condition")?
        .string()
        .wrap_error_with_message("invalid condition")?;
    let condition = Id::from(condition);
    Ok(Value::Boolean(Side::has_condition(
        &context.battle_context_mut().side_context(side_index)?,
        &condition,
    )))
}

fn clause_integer_value(mut context: FunctionContext) -> Result<Option<Value>> {
    let clause = context
        .pop_front()
        .wrap_expectation("missing clause")?
        .clause_id()
        .wrap_error_with_message("invalid clause")?;
    Ok(context
        .evaluation_context()
        .battle_context()
        .battle()
        .format
        .rules
        .numeric_value::<u64>(&clause)
        .map(|val| Value::UFraction(val.into())))
}

fn clause_type_value(mut context: FunctionContext) -> Result<Option<Value>> {
    let clause = context
        .pop_front()
        .wrap_expectation("missing clause")?
        .clause_id()
        .wrap_error_with_message("invalid clause")?;
    match context
        .evaluation_context()
        .battle_context()
        .battle()
        .format
        .rules
        .value(&clause)
    {
        Some(val) => Ok(Some(Value::Type(
            Type::from_str(val).map_err(general_error)?,
        ))),
        None => Ok(None),
    }
}

fn new_object(_: FunctionContext) -> Value {
    Value::Object(HashMap::default())
}

fn object_keys(mut context: FunctionContext) -> Result<Value> {
    let object = context
        .pop_front()
        .wrap_expectation("missing object")?
        .object()
        .wrap_error_with_message("invalid object")?;
    Ok(Value::List(
        object
            .keys()
            .cloned()
            .map(|key| Value::String(key))
            .collect(),
    ))
}

fn object_value(mut context: FunctionContext) -> Result<Option<Value>> {
    let object = context
        .pop_front()
        .wrap_expectation("missing object")?
        .object()
        .wrap_error_with_message("invalid object")?;
    let key = context
        .pop_front()
        .wrap_expectation("missing key")?
        .string()
        .wrap_error_with_message("invalid key")?;
    Ok(object.get(&key).cloned())
}

fn object_increment(mut context: FunctionContext) -> Result<Value> {
    let mut object = context
        .pop_front()
        .wrap_expectation("missing object")?
        .object()
        .wrap_error_with_message("invalid object")?;
    let key = context
        .pop_front()
        .wrap_expectation("missing key")?
        .string()
        .wrap_error_with_message("invalid key")?;
    let value = match object.get(&key).cloned() {
        Some(val) => val
            .integer_u64()
            .wrap_error_with_message("cannot increment a non-integer value")?,
        None => 0,
    };
    let value = value + 1;
    object.insert(key, Value::UFraction(value.into()));
    Ok(Value::Object(object))
}

fn object_get(mut context: FunctionContext) -> Result<Option<Value>> {
    let object = context
        .pop_front()
        .wrap_expectation("missing object")?
        .object()
        .wrap_error_with_message("invalid object")?;
    let key = context
        .pop_front()
        .wrap_expectation("missing key")?
        .string()
        .wrap_error_with_message("invalid key")?;
    Ok(object.get(&key).cloned())
}

fn object_set(mut context: FunctionContext) -> Result<Value> {
    let mut object = context
        .pop_front()
        .wrap_expectation("missing object")?
        .object()
        .wrap_error_with_message("invalid object")?;
    let key = context
        .pop_front()
        .wrap_expectation("missing key")?
        .string()
        .wrap_error_with_message("invalid key")?;
    let value = context.pop_front().wrap_expectation("missing value")?;
    object.insert(key, value);
    Ok(Value::Object(object))
}

fn plural(mut context: FunctionContext) -> Result<Value> {
    if context
        .pop_front()
        .wrap_expectation("missing number")?
        .fraction_u64()
        .is_ok_and(|val| val == 1)
    {
        Ok(Value::String("".to_owned()))
    } else {
        Ok(Value::String("s".to_owned()))
    }
}

fn add_pseudo_weather(mut context: FunctionContext) -> Result<Value> {
    let pseudo_weather = context
        .pop_front()
        .wrap_expectation("missing pseudo weather id")?
        .string()
        .wrap_error_with_message("invalid pseudo weather")?;
    let pseudo_weather = Id::from(pseudo_weather);

    core_battle_actions::add_pseudo_weather(
        &mut context.forward_to_field_effect()?,
        &pseudo_weather,
    )
    .map(|val| Value::Boolean(val))
}

fn remove_pseudo_weather(mut context: FunctionContext) -> Result<Value> {
    let pseudo_weather = context
        .pop_front()
        .wrap_expectation("missing pseudo weather id")?
        .string()
        .wrap_error_with_message("invalid pseudo weather")?;
    let pseudo_weather = Id::from(pseudo_weather);

    core_battle_actions::remove_pseudo_weather(
        &mut context.forward_to_field_effect()?,
        &pseudo_weather,
    )
    .map(|val| Value::Boolean(val))
}

fn has_pseudo_weather(mut context: FunctionContext) -> Result<Value> {
    let pseudo_weather = context
        .pop_front()
        .wrap_expectation("missing pseudo weather id")?
        .string()
        .map(|ability| Id::from(ability))
        .wrap_error_with_message("invalid pseudo weather id")?;
    Ok(Value::Boolean(
        context
            .context
            .battle_context()
            .battle()
            .field
            .pseudo_weathers
            .contains_key(&pseudo_weather),
    ))
}

fn start_ability(mut context: FunctionContext) -> Result<()> {
    let target_handle = context.target_handle_positional()?;
    let silent = context.silent();
    core_battle_actions::start_ability(
        &mut context.forward_to_applying_effect_context_with_target(target_handle)?,
        silent,
    )
}

fn end_ability(mut context: FunctionContext) -> Result<()> {
    let target_handle = context.target_handle_positional()?;
    let silent = context.silent();
    core_battle_actions::end_ability(
        &mut context.forward_to_applying_effect_context_with_target(target_handle)?,
        silent,
    )
}

fn start_item(mut context: FunctionContext) -> Result<()> {
    let target_handle = context.target_handle_positional()?;
    let silent = context.silent();
    core_battle_actions::start_item(
        &mut context.forward_to_applying_effect_context_with_target(target_handle)?,
        silent,
    )
}

fn end_item(mut context: FunctionContext) -> Result<()> {
    let target_handle = context.target_handle_positional()?;
    let silent = context.silent();
    core_battle_actions::end_item(
        &mut context.forward_to_applying_effect_context_with_target(target_handle)?,
        silent,
    )
}

fn mon_ability_suppressed_by_this_effect(mut context: FunctionContext) -> Result<Value> {
    let target_handle = context.target_handle_positional()?;
    let effect_state = context
        .effect_state()
        .wrap_expectation("this effect has no effect state")?;

    let mut context = context.context.mon_context(target_handle)?;

    let started = effect_state
        .get_mut(context.as_battle_context_mut())?
        .started();

    effect_state
        .get_mut(context.as_battle_context_mut())?
        .set_started(false);
    let ability_without_this_effect = mon_states::effective_ability(&mut context);

    effect_state
        .get_mut(context.as_battle_context_mut())?
        .set_started(true);
    let ability_with_this_effect = mon_states::effective_ability(&mut context);

    effect_state
        .get_mut(context.as_battle_context_mut())?
        .set_started(started);

    Ok(Value::Boolean(
        ability_without_this_effect.is_some() && ability_with_this_effect.is_none(),
    ))
}

fn get_stat(mut context: FunctionContext) -> Result<Value> {
    let unboosted = context.has_flag("unboosted");
    let unmodified = context.has_flag("unmodified");
    let target_handle = context.target_handle_positional()?;
    let stat = context
        .pop_front()
        .wrap_expectation("missing stat")?
        .stat()
        .wrap_error_with_message("invalid stat")?;

    Mon::get_stat(
        &mut context.mon_context(target_handle)?,
        stat,
        unboosted,
        unmodified,
    )
    .map(|val| Value::UFraction(val.into()))
}

fn special_item_data(mut context: FunctionContext) -> Result<Value> {
    let item_id = context
        .pop_front()
        .wrap_expectation("missing item")?
        .item_id()
        .wrap_error_with_message("invalid item")?;
    Ok(Value::SpecialItemData(
        context
            .evaluation_context()
            .battle_context()
            .battle()
            .dex
            .items
            .get_by_id(&item_id)?
            .data
            .special_data
            .clone(),
    ))
}

fn swap_boosts(mut context: FunctionContext) -> Result<()> {
    let target = context.target_handle_positional()?;
    let source = context
        .pop_front()
        .wrap_expectation("missing source")?
        .mon_handle()
        .wrap_error_with_message("invalid source")?;
    let mut boosts = Vec::default();
    while let Some(val) = context.pop_front() {
        boosts.push(val.boost().wrap_error_with_message("invalid boost")?);
    }

    core_battle_actions::swap_boosts(
        &mut context
            .forward_to_applying_effect_context_with_source_and_target(Some(source), target)?,
        &boosts,
    )
}

fn base_species(mut context: FunctionContext) -> Result<Value> {
    let mon = context.target_handle_positional()?;
    let context = context.mon_context(mon)?;
    Mon::base_species_of_species(&context).map(|id| Value::String(id.to_string()))
}

fn skip_effect_callback(mut context: FunctionContext) -> Result<()> {
    let effect = context
        .pop_front()
        .wrap_expectation("missing effect")?
        .effect_id()
        .wrap_error_with_message("invalid effect")?;
    if let Some(effect) = context
        .battle_context_mut()
        .battle_mut()
        .resolve_effect_id(&effect)
    {
        context.event_state().skip_effect(effect);
    }

    Ok(())
}

fn value_from_local_data(mut context: FunctionContext) -> Result<Option<Value>> {
    let effect_handle = context.effect_handle_positional()?;
    let key = context
        .pop_front()
        .wrap_expectation("missing key")?
        .string()
        .wrap_error_with_message("invalid key")?;
    Ok(CoreBattle::get_effect_by_handle(
        context.evaluation_context().battle_context(),
        &effect_handle,
    )?
    .fxlang_effect()
    .wrap_expectation("effect does not have local data")?
    .local_data
    .values
    .get(&key)
    .map(|val| Value::String(val.clone())))
}

fn has_species_registered(mut context: FunctionContext) -> Result<Value> {
    let player = context
        .pop_front()
        .wrap_expectation("missing player")?
        .player_index()
        .wrap_error_with_message("invalid player")?;
    let species = context
        .pop_front()
        .wrap_expectation("missing species")?
        .species_id()
        .wrap_error_with_message("invalid species")?;
    Ok(Value::Boolean(Player::has_species_registered(
        &context.battle_context_mut().player_context(player)?,
        &species,
    )))
}

fn force_fully_heal(mut context: FunctionContext) -> Result<()> {
    let target = context.target_handle_positional()?;
    Mon::force_fully_heal(&mut context.mon_context(target)?)
}

fn set_upgraded_to_max_move(mut context: FunctionContext) -> Result<()> {
    let active_move = context
        .pop_front()
        .wrap_expectation("missing move")?
        .active_move()
        .wrap_error_with_message("invalid move")?;
    let base_move = context
        .pop_front()
        .wrap_expectation("missing base move")?
        .move_id(context.evaluation_context_mut())
        .wrap_error_with_message("invalid base move")?;
    context
        .battle_context_mut()
        .active_move_mut(active_move)?
        .upgraded = Some(UpgradedMoveSource::MaxMove { base_move });
    Ok(())
}

fn get_move_targets(mut context: FunctionContext) -> Result<Value> {
    let active_move = context
        .pop_front()
        .wrap_expectation("missing move")?
        .active_move()
        .wrap_error_with_message("invalid move")?;
    let mon_handle = context
        .pop_front()
        .wrap_expectation("missing user")?
        .mon_handle()
        .wrap_error_with_message("invalid user")?;
    let target = context
        .pop_front()
        .map(|val| val.mon_handle().wrap_error_with_message("invalid target"))
        .transpose()?;
    core_battle_actions::get_move_targets(
        &mut context
            .mon_context(mon_handle)?
            .active_move_context(active_move)?,
        target,
    )
    .map(|targets| {
        Value::List(
            targets
                .into_iter()
                .map(|target| Value::Mon(target))
                .collect(),
        )
    })
}
