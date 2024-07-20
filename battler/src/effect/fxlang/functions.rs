use std::{
    collections::VecDeque,
    str::FromStr,
};

use ahash::HashSetExt;

use crate::{
    battle::{
        core_battle_actions,
        core_battle_effects,
        core_battle_logs,
        Boost,
        BoostOrderIterator,
        BoostTable,
        CoreBattle,
        Mon,
        MonContext,
        MoveOutcomeOnTarget,
        MoveSlot,
        Player,
        Side,
    },
    battler_error,
    common::{
        Error,
        FastHashSet,
        Id,
        Identifiable,
        WrapResultError,
    },
    effect::{
        fxlang::{
            BattleEvent,
            EvaluationContext,
            MaybeReferenceValueForOperation,
            Value,
            VariableInput,
        },
        Effect,
        EffectHandle,
    },
    log::Event,
    log_event,
    moves::{
        Move,
        MoveFlags,
        MoveTarget,
    },
    rng::rand_util,
};

/// Runs an fxlang function.
///
/// This function serves as the boundary between fxlang program evaluation and the battle engine.
pub fn run_function(
    context: &mut EvaluationContext,
    function_name: &str,
    args: VecDeque<Value>,
) -> Result<Option<Value>, Error> {
    match function_name {
        "add_volatile" => add_volatile(context, args).map(|val| Some(val)),
        "all_active_mons" => all_active_mons(context).map(|val| Some(val)),
        "apply_drain" => apply_drain(context, args).map(|()| None),
        "apply_recoil_damage" => apply_recoil_damage(context, args).map(|()| None),
        "boost" => boost(context, args).map(|val| Some(val)),
        "boostable_stats" => Ok(Some(boostable_stats())),
        "calculate_damage" => calculate_damage(context, args).map(|val| Some(val)),
        "calculate_confusion_damage" => {
            calculate_confusion_damage(context, args).map(|val| Some(val))
        }
        "can_escape" => can_escape(context, args).map(|val| Some(val)),
        "can_switch" => can_switch(context, args).map(|val| Some(val)),
        "chance" => chance(context, args).map(|val| Some(val)),
        "clear_boosts" => clear_boosts(context, args).map(|()| None),
        "cure_status" => cure_status(context, args).map(|()| None),
        "damage" => damage(context, args).map(|val| Some(val)),
        "debug_log" => debug_log(context, args).map(|()| None),
        "direct_damage" => direct_damage(context, args).map(|()| None),
        "disable_move" => disable_move(context, args).map(|()| None),
        "do_not_animate_last_move" => do_not_animate_last_move(context).map(|()| None),
        "escape" => escape(context, args).map(|val| Some(val)),
        "floor" => floor(args).map(|val| Some(val)),
        "get_all_moves" => get_all_moves(context, args).map(|val| Some(val)),
        "get_boost" => get_boost(args).map(|val| Some(val)),
        "get_move" => get_move(context, args).map(|val| Some(val)),
        "has_ability" => has_ability(context, args).map(|val| Some(val)),
        "has_move" => has_move(context, args).map(|val| Some(val)),
        "has_type" => has_type(context, args).map(|val| Some(val)),
        "has_volatile" => has_volatile(context, args).map(|val| Some(val)),
        "heal" => heal(context, args).map(|()| None),
        "is_ally" => is_ally(context, args).map(|val| Some(val)),
        "is_boolean" => is_boolean(args).map(|val| Some(val)),
        "is_defined" => is_defined(args).map(|val| Some(val)),
        "is_undefined" => is_undefined(args).map(|val| Some(val)),
        "log" => log(context, args).map(|()| None),
        "log_ability" => log_ability(context).map(|()| None),
        "log_activate" => log_activate(context, args).map(|()| None),
        "log_cant" => log_cant(&mut context.target_context()?, args).map(|()| None),
        "log_end" => log_end(context, args).map(|()| None),
        "log_fail" => log_fail(context, args).map(|()| None),
        "log_ohko" => log_ohko(context, args).map(|()| None),
        "log_prepare_move" => log_prepare_move(context).map(|()| None),
        "log_side_end" => log_side_end(context, args).map(|()| None),
        "log_side_start" => log_side_start(context, args).map(|()| None),
        "log_start" => log_start(context, args).map(|()| None),
        "log_status" => log_status(context, args).map(|()| None),
        "log_weather" => log_weather(context, args).map(|()| None),
        "max" => max(args).map(|val| Some(val)),
        "mon_at_target_location" => mon_at_target_location(context, args),
        "mon_in_position" => mon_in_position(context, args),
        "mons_per_side" => mons_per_side(context).map(|val| Some(val)),
        "move_at_move_slot_index" => move_at_move_slot_index(context, args),
        "move_crit_target" => move_crit_target(context, args).map(|val| Some(val)),
        "move_has_flag" => move_has_flag(context, args).map(|val| Some(val)),
        "move_slot" => move_slot(context, args).map(|val| Some(val)),
        "move_slot_index" => move_slot_index(context, args),
        "new_active_move_from_local_data" => {
            new_active_move_from_local_data(context, args).map(|val| Some(val))
        }
        "overwrite_move_slot" => overwrite_move_slot(context, args).map(|()| None),
        "random" => random(context, args).map(|val| Some(val)),
        "random_target" => random_target(context, args),
        "remove_volatile" => remove_volatile(context, args).map(|val| Some(val)),
        "run_event" => run_event(context, args).map(|val| Some(val)),
        "run_event_on_mon_item" => run_event_on_mon_item(context, args).map(|()| None),
        "run_event_on_move" => run_event_on_move(context, args).map(|()| None),
        "trap" => trap_mon(context, args).map(|()| None),
        "sample" => sample(context, args),
        "set_boost" => set_boost(args).map(|val| Some(val)),
        "set_status" => set_status(context, args).map(|val| Some(val)),
        "set_types" => set_types(context, args).map(|val| Some(val)),
        "set_weather" => set_weather(context, args).map(|val| Some(val)),
        "target_location_of_mon" => target_location_of_mon(context, args).map(|val| Some(val)),
        "transform_into" => transform_into(context, args).map(|val| Some(val)),
        "use_active_move" => use_active_move(context, args).map(|val| Some(val)),
        "use_move" => use_move(context, args).map(|val| Some(val)),
        "volatile_effect_state" => volatile_effect_state(context, args),
        _ => Err(battler_error!("undefined function: {function_name}")),
    }
}

fn has_special_string_flag(args: &mut VecDeque<Value>, flag: &str) -> bool {
    match args
        .iter()
        .enumerate()
        .find(|(_, arg)| (*arg).clone().string().is_ok_and(|arg| arg == flag))
    {
        Some((i, _)) => {
            args.remove(i);
            true
        }
        None => false,
    }
}

fn should_use_source_effect(args: &mut VecDeque<Value>) -> bool {
    has_special_string_flag(args, "use_source")
}

fn debug_log(context: &mut EvaluationContext, args: VecDeque<Value>) -> Result<(), Error> {
    let mut event = log_event!("fxlang_debug");
    for (i, arg) in args.into_iter().enumerate() {
        event.set(format!("arg{i}"), format!("{arg:?}"));
    }
    context.battle_context_mut().battle_mut().log(event);
    Ok(())
}

fn log_internal(
    context: &mut EvaluationContext,
    title: String,
    args: VecDeque<Value>,
) -> Result<(), Error> {
    let mut event = Event::new(title);
    for arg in args {
        let entry = arg.string().wrap_error_with_message("invalid log entry")?;
        match entry.split_once(':') {
            None => event.extend(&entry.as_str()),
            Some((a, b)) => event.extend(&(a, b)),
        }
    }
    context.battle_context_mut().battle_mut().log(event);
    Ok(())
}

fn log(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let title = args
        .pop_front()
        .wrap_error_with_message("missing log title")?
        .string()
        .wrap_error_with_message("invalid title")?;
    log_internal(context, title, args)
}

fn add_effect_to_args(
    context: &mut EvaluationContext,
    args: &mut VecDeque<Value>,
) -> Result<(), Error> {
    match context.effect_context_mut().effect() {
        Effect::ActiveMove(active_move, _) => {
            args.push_front(Value::String(format!("move:{}", active_move.data.name)))
        }
        Effect::Ability(ability) => {
            args.push_front(Value::String(format!("ability:{}", ability.data.name)))
        }
        Effect::Item(item) => args.push_front(Value::String(format!("item:{}", item.data.name))),
        Effect::Condition(condition) => args.push_front(Value::String(format!(
            "{}:{}",
            condition.non_empty_condition_type_name(),
            condition.data.name
        ))),
        Effect::MoveCondition(condition) => {
            args.push_front(Value::String(format!("move:{}", condition.data.name)))
        }
        _ => (),
    }
    Ok(())
}

fn log_ability(context: &mut EvaluationContext) -> Result<(), Error> {
    core_battle_logs::ability(&mut context.target_context()?)
}

fn log_activate(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let with_target = has_special_string_flag(&mut args, "with_target");
    let with_source = has_special_string_flag(&mut args, "with_source");

    let no_effect = has_special_string_flag(&mut args, "no_effect");
    if !no_effect {
        add_effect_to_args(context, &mut args)?;
    }

    if with_target {
        args.push_front(Value::String(format!(
            "mon:{}",
            Mon::position_details(&context.target_context()?)?
        )));
    }
    if with_source {
        args.push_back(Value::String(format!(
            "of:{}",
            Mon::position_details(
                &context
                    .source_context()?
                    .wrap_error_with_message("effect has no source")?
            )?
        )));
    }

    log_internal(context, "activate".to_owned(), args)
}

fn log_start(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let no_effect = has_special_string_flag(&mut args, "no_effect");
    let with_source_effect = has_special_string_flag(&mut args, "with_source_effect");

    if with_source_effect {
        args.push_back(Value::String(format!(
            "from:{}",
            context
                .source_effect_context()?
                .wrap_error_with_message("effect has no source effect")?
                .effect()
                .full_name()
        )));

        if context.effect_handle().is_ability() {
            if let Some(source_context) = context.source_context()? {
                args.push_back(Value::String(format!(
                    "of:{}",
                    Mon::position_details(&source_context)?
                )));
            }
        }
    }

    if !no_effect {
        add_effect_to_args(context, &mut args)?;
    }

    args.push_front(Value::String(format!(
        "mon:{}",
        Mon::position_details(&mut context.target_context()?)?
    )));

    log_internal(context, "start".to_owned(), args)
}

fn log_end(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let no_effect = has_special_string_flag(&mut args, "no_effect");
    if !no_effect {
        add_effect_to_args(context, &mut args)?;
    }

    args.push_front(Value::String(format!(
        "mon:{}",
        Mon::position_details(&mut context.target_context()?)?
    )));

    log_internal(context, "end".to_owned(), args)
}

fn log_side_start(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let side_index = context
        .side_index()
        .wrap_error_with_message("context has no side index")?;

    let no_effect = has_special_string_flag(&mut args, "no_effect");
    if !no_effect {
        add_effect_to_args(context, &mut args)?;
    }

    args.push_front(Value::String(format!("side:{side_index}")));

    log_internal(context, "sidestart".to_owned(), args)
}

fn log_side_end(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let side_index = context
        .side_index()
        .wrap_error_with_message("context has no side index")?;

    let no_effect = has_special_string_flag(&mut args, "no_effect");
    if !no_effect {
        add_effect_to_args(context, &mut args)?;
    }

    args.push_front(Value::String(format!("side:{side_index}")));

    log_internal(context, "sideend".to_owned(), args)
}

fn log_prepare_move(context: &mut EvaluationContext) -> Result<(), Error> {
    let mut context = context
        .source_active_move_context()?
        .wrap_error_with_message("source effect is not an active move")?;
    let event = log_event!(
        "prepare",
        ("mon", Mon::position_details(context.as_mon_context())?),
        ("move", context.active_move().data.name.to_owned())
    );
    context.battle_mut().log(event);
    Ok(())
}

fn log_cant(context: &mut MonContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let reason = args
        .pop_front()
        .wrap_error_with_message("missing reason")?
        .string()?;
    core_battle_logs::cant(context, &reason, None)
}

fn log_status(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let status = args
        .pop_front()
        .wrap_error_with_message("missing status name")?
        .string()
        .wrap_error_with_message("invalid status name")?;

    let with_source_effect = has_special_string_flag(&mut args, "with_source_effect");

    let mut event = log_event!(
        "status",
        ("mon", Mon::position_details(&context.target_context()?)?),
        ("status", status)
    );
    if with_source_effect {
        event.set(
            "from",
            context
                .source_effect_context()?
                .wrap_error_with_message("effect has no source effect")?
                .effect()
                .full_name(),
        );
        if context.effect_handle().is_ability() {
            if let Some(source_context) = context.source_context()? {
                event.set("of", Mon::position_details(&source_context)?);
            }
        }
    }
    context.battle_context_mut().battle_mut().log(event);
    Ok(())
}

fn log_weather(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let weather = match args.pop_front() {
        Some(value) => value.string().wrap_error_with_message("invalid weather")?,
        None => "Clear".to_owned(),
    };

    let with_source_effect = has_special_string_flag(&mut args, "with_source_effect");
    let residual = has_special_string_flag(&mut args, "residual");

    let mut event = log_event!("weather", ("weather", weather));
    if with_source_effect {
        let source_effect_context = context
            .source_effect_context()?
            .wrap_error_with_message("effect has no source effect")?;
        event.set("from", source_effect_context.effect().full_name());
        if source_effect_context.effect_handle().is_ability() {
            if let Some(source_context) = context.source_context()? {
                event.set("of", Mon::position_details(&source_context)?);
            }
        }
    }

    if residual {
        event.add_flag("residual");
    }

    context.battle_context_mut().battle_mut().log(event);
    Ok(())
}

fn log_fail(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    core_battle_logs::fail(&mut context.mon_context(mon_handle)?)
}

fn log_ohko(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    core_battle_logs::ohko(&mut context.mon_context(mon_handle)?)
}

fn random(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<Value, Error> {
    let a = args.pop_front().map(|val| val.integer_u64().ok()).flatten();
    let b = args.pop_front().map(|val| val.integer_u64().ok()).flatten();
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
        _ => return Err(battler_error!("invalid random arguments")),
    };
    Ok(Value::U64(val))
}

fn chance(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<Value, Error> {
    let a = args.pop_front().map(|val| val.integer_u64().ok()).flatten();
    let b = args.pop_front().map(|val| val.integer_u64().ok()).flatten();
    let val = match (a, b) {
        (None, None) => return Err(battler_error!("chance requires at least one argument")),
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
        _ => return Err(battler_error!("invalid chance arguments")),
    };
    Ok(Value::Boolean(val))
}

fn sample(
    context: &mut EvaluationContext,
    mut args: VecDeque<Value>,
) -> Result<Option<Value>, Error> {
    let list = args
        .pop_front()
        .wrap_error_with_message("missing list")?
        .list()
        .wrap_error_with_message("invalid list")?;
    Ok(rand_util::sample_slice(
        context.battle_context_mut().battle_mut().prng.as_mut(),
        list.as_slice(),
    )
    .cloned())
}

fn damage(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<Value, Error> {
    let no_source = has_special_string_flag(&mut args, "no_source");
    let source_handle = if no_source {
        None
    } else {
        context.source_handle()
    };

    let mut target_handle = context.target_handle();
    if let Some(value) = args.front().cloned() {
        if let Ok(value) = value.mon_handle() {
            args.pop_front();
            target_handle = Some(value);
        }
    }
    let target_handle = target_handle.wrap_error_with_message("missing target")?;

    let amount = args
        .pop_front()
        .wrap_error_with_message("missing damage amount")?
        .integer_u16()
        .wrap_error_with_message("invalid damage amount")?;

    let mut damaging_effect = context.effect_handle().clone();
    if let Some(value) = args.pop_front() {
        damaging_effect = value
            .effect_handle()
            .wrap_error_with_message("invalid damaging effect")?;
    }

    let source_effect_handle = context.source_effect_handle().cloned();
    core_battle_actions::damage(
        &mut context.battle_context_mut().applying_effect_context(
            damaging_effect,
            source_handle,
            target_handle,
            source_effect_handle,
        )?,
        amount,
    )
    .map(|damage| Value::U64(damage as u64))
}

fn direct_damage(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let no_source = has_special_string_flag(&mut args, "no_source");
    let source_handle = if no_source {
        None
    } else {
        context.source_handle()
    };

    let mut target_handle = context.target_handle();
    if let Some(value) = args.front().cloned() {
        if let Ok(value) = value.mon_handle() {
            args.pop_front();
            target_handle = Some(value);
        }
    }
    let target_handle = target_handle.wrap_error_with_message("missing target")?;

    let amount = args
        .pop_front()
        .wrap_error_with_message("missing damage amount")?
        .integer_u16()
        .wrap_error_with_message("invalid damage amount")?;

    let mut damaging_effect = context.effect_handle().clone();
    if let Some(value) = args.pop_front() {
        damaging_effect = value
            .effect_handle()
            .wrap_error_with_message("invalid damaging effect")?;
    }

    core_battle_actions::direct_damage(
        &mut context.mon_context(target_handle)?,
        amount,
        source_handle,
        Some(&damaging_effect),
    )?;
    Ok(())
}

fn has_ability(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<Value, Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let ability = args
        .pop_front()
        .wrap_error_with_message("missing ability id")?
        .string()
        .map(|ability| Id::from(ability))
        .wrap_error_with_message("invalid ability id")?;
    Mon::has_ability(&mut context.mon_context(mon_handle)?, &ability).map(|val| Value::Boolean(val))
}

fn has_volatile(
    context: &mut EvaluationContext,
    mut args: VecDeque<Value>,
) -> Result<Value, Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let volatile = args
        .pop_front()
        .wrap_error_with_message("missing volatile id")?
        .string()
        .map(|ability| Id::from(ability))
        .wrap_error_with_message("invalid volatile id")?;
    Mon::has_volatile(&mut context.mon_context(mon_handle)?, &volatile)
        .map(|val| Value::Boolean(val))
}

fn cure_status(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let log_effect = has_special_string_flag(&mut args, "log_effect");
    let mut context = if should_use_source_effect(&mut args) {
        context.forward_source_effect_to_applying_effect(mon_handle)?
    } else {
        context.forward_effect_to_applying_effect(mon_handle)?
    };
    core_battle_actions::cure_status(&mut context, log_effect)?;
    Ok(())
}

fn move_has_flag(
    context: &mut EvaluationContext,
    mut args: VecDeque<Value>,
) -> Result<Value, Error> {
    let move_id = args
        .pop_front()
        .wrap_error_with_message("missing move")?
        .move_id(context)
        .wrap_error_with_message("invalid move")?;
    let move_flag = args
        .pop_front()
        .wrap_error_with_message("missing move flag")?
        .string()
        .wrap_error_with_message("invalid move flag")?;
    let move_flag = MoveFlags::from_str(&move_flag).wrap_error_with_message("invalid move flag")?;
    Ok(Value::Boolean(
        context
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

fn add_volatile(
    context: &mut EvaluationContext,
    mut args: VecDeque<Value>,
) -> Result<Value, Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let volatile = args
        .pop_front()
        .wrap_error_with_message("missing volatile id")?
        .string()
        .wrap_error_with_message("invalid volatile")?;
    let volatile = Id::from(volatile);
    let mut context = if should_use_source_effect(&mut args) {
        context.forward_source_effect_to_applying_effect(mon_handle)?
    } else {
        context.forward_effect_to_applying_effect(mon_handle)?
    };
    core_battle_actions::try_add_volatile(&mut context, &volatile, false)
        .map(|val| Value::Boolean(val))
}

fn remove_volatile(
    context: &mut EvaluationContext,
    mut args: VecDeque<Value>,
) -> Result<Value, Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let volatile = args
        .pop_front()
        .wrap_error_with_message("missing volatile id")?
        .string()
        .wrap_error_with_message("invalid volatile")?;

    let no_events = has_special_string_flag(&mut args, "no_events");
    let volatile = Id::from(volatile);
    let mut context = if should_use_source_effect(&mut args) {
        context.forward_source_effect_to_applying_effect(mon_handle)?
    } else {
        context.forward_effect_to_applying_effect(mon_handle)?
    };
    core_battle_actions::remove_volatile(&mut context, &volatile, no_events)
        .map(|val| Value::Boolean(val))
}

fn run_event(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<Value, Error> {
    let event = args
        .pop_front()
        .wrap_error_with_message("missing event")?
        .string()
        .wrap_error_with_message("invalid event")?;
    let event = BattleEvent::from_str(&event).wrap_error_with_message("invalid event")?;

    match context {
        EvaluationContext::ApplyingEffect(context) => Ok(Value::Boolean(
            core_battle_effects::run_event_for_applying_effect(
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
            Err(battler_error!("effect must have a target to run an event"))
        }
    }
}

fn run_event_on_mon_item(
    context: &mut EvaluationContext,
    mut args: VecDeque<Value>,
) -> Result<(), Error> {
    let event = args
        .pop_front()
        .wrap_error_with_message("missing event")?
        .string()
        .wrap_error_with_message("invalid event")?;
    let event = BattleEvent::from_str(&event).wrap_error_with_message("invalid event")?;
    core_battle_effects::run_mon_item_event(context.applying_effect_context_mut()?, event);
    Ok(())
}

fn run_event_on_move(
    context: &mut EvaluationContext,
    mut args: VecDeque<Value>,
) -> Result<(), Error> {
    let on_user = has_special_string_flag(&mut args, "on_user");
    let target = match (on_user, context.target_handle()) {
        (true, _) => core_battle_effects::MoveTargetForEvent::User,
        (_, Some(target_handle)) => core_battle_effects::MoveTargetForEvent::Mon(target_handle),
        (_, None) => match context.side_index() {
            Some(side_index) => core_battle_effects::MoveTargetForEvent::Side(side_index),
            None => core_battle_effects::MoveTargetForEvent::None,
        },
    };
    let mut context = context
        .source_active_move_context()?
        .wrap_error_with_message("source effect is not an active move")?;
    let event = args
        .pop_front()
        .wrap_error_with_message("missing event")?
        .string()
        .wrap_error_with_message("invalid event")?;
    let event = BattleEvent::from_str(&event).wrap_error_with_message("invalid event")?;
    core_battle_effects::run_active_move_event_expecting_void(
        &mut context,
        event,
        target,
        VariableInput::default(),
    );
    Ok(())
}

fn do_not_animate_last_move(context: &mut EvaluationContext) -> Result<(), Error> {
    core_battle_logs::do_not_animate_last_move(context.battle_context_mut());
    Ok(())
}

fn trap_mon(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("mising mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    core_battle_actions::trap_mon(&mut context.mon_context(mon_handle)?)
}

fn calculate_damage(
    context: &mut EvaluationContext,
    mut args: VecDeque<Value>,
) -> Result<Value, Error> {
    let mut context = context
        .source_active_move_context()?
        .wrap_error_with_message("source effect is not an active move")?;
    let target_handle = args
        .pop_front()
        .wrap_error_with_message("missing target")?
        .mon_handle()
        .wrap_error_with_message("invalid target")?;
    match core_battle_actions::calculate_damage(&mut context.target_context(target_handle)?)? {
        MoveOutcomeOnTarget::Damage(damage) => Ok(Value::U64(damage as u64)),
        MoveOutcomeOnTarget::Success | MoveOutcomeOnTarget::Unknown => Ok(Value::U64(0)),
        _ => Ok(Value::Boolean(false)),
    }
}

fn calculate_confusion_damage(
    context: &mut EvaluationContext,
    mut args: VecDeque<Value>,
) -> Result<Value, Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let base_power = args
        .pop_front()
        .wrap_error_with_message("missing base power")?
        .integer_u32()
        .wrap_error_with_message("invalid base power")?;
    core_battle_actions::calculate_confusion_damage(
        &mut context.mon_context(mon_handle)?,
        base_power,
    )
    .map(|value| Value::U64(value as u64))
}

fn max(mut args: VecDeque<Value>) -> Result<Value, Error> {
    let mut first = args
        .pop_front()
        .wrap_error_with_message("max requires at least one argument")?;
    while let Some(second) = args.pop_front() {
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

fn floor(mut args: VecDeque<Value>) -> Result<Value, Error> {
    let number = args
        .pop_front()
        .wrap_error_with_message("missing number")?
        .integer_u64()
        .wrap_error_with_message("invalid number")?;
    Ok(Value::U64(number))
}

fn heal(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let damage = args
        .pop_front()
        .wrap_error_with_message("missing damage")?
        .integer_u16()
        .wrap_error_with_message("invalid damage")?;
    let mut source_handle = context.source_handle();
    if let Some(source) = args.pop_front() {
        if let Ok(source) = source.mon_handle() {
            source_handle = Some(source);
        }
    }
    let effect = context.effect_handle().clone();
    core_battle_actions::heal(
        &mut context.mon_context(mon_handle)?,
        damage,
        source_handle,
        Some(&effect),
    )?;
    Ok(())
}

fn apply_drain(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let target_handle = args
        .pop_front()
        .wrap_error_with_message("missing target")?
        .mon_handle()
        .wrap_error_with_message("invalid target")?;
    let source_handle = args
        .pop_front()
        .wrap_error_with_message("missing source")?
        .mon_handle()
        .wrap_error_with_message("invalid source")?;
    let damage = args
        .pop_front()
        .wrap_error_with_message("missing damage")?
        .integer_u16()
        .wrap_error_with_message("invalid damage")?;
    core_battle_actions::apply_drain(
        &mut context
            .effect_context_mut()
            .applying_effect_context(Some(source_handle), target_handle)?,
        damage,
    )
}

fn apply_recoil_damage(
    context: &mut EvaluationContext,
    mut args: VecDeque<Value>,
) -> Result<(), Error> {
    let mut context = context
        .source_active_move_context()?
        .wrap_error_with_message("source effect is not an active move")?;
    let damage = args
        .pop_front()
        .wrap_error_with_message("missing damage")?
        .integer_u64()
        .wrap_error_with_message("invalid damage")?;
    core_battle_actions::apply_recoil_damage(&mut context, damage)
}

fn is_boolean(mut args: VecDeque<Value>) -> Result<Value, Error> {
    let value = args.pop_front().wrap_error_with_message("missing value")?;
    Ok(Value::Boolean(value.boolean().is_ok()))
}

fn is_defined(mut args: VecDeque<Value>) -> Result<Value, Error> {
    let value = args.pop_front().wrap_error_with_message("missing value")?;
    Ok(Value::Boolean(!value.is_undefined()))
}

fn is_undefined(mut args: VecDeque<Value>) -> Result<Value, Error> {
    let value = args.pop_front().wrap_error_with_message("missing value")?;
    Ok(Value::Boolean(value.is_undefined()))
}

fn set_status(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<Value, Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let status = args
        .pop_front()
        .wrap_error_with_message("missing status id")?
        .string()
        .wrap_error_with_message("invalid status")?;
    let status = Id::from(status);
    let mut context = if should_use_source_effect(&mut args) {
        context.forward_source_effect_to_applying_effect(mon_handle)?
    } else {
        context.forward_effect_to_applying_effect(mon_handle)?
    };
    core_battle_actions::try_set_status(&mut context, Some(status), false)
        .map(move |val| Value::Boolean(val.success()))
}

fn is_ally(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<Value, Error> {
    let left_mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing first mon")?
        .mon_handle()
        .wrap_error_with_message("invalid first mon")?;
    let right_mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing first mon")?
        .mon_handle()
        .wrap_error_with_message("invalid first mon")?;
    Ok(Value::Boolean(
        context
            .mon(left_mon_handle)?
            .is_ally(context.mon(right_mon_handle)?),
    ))
}

fn boostable_stats() -> Value {
    Value::List(Vec::from_iter(
        BoostOrderIterator::new().map(|boost| Value::Boost(boost)),
    ))
}

fn get_boost(mut args: VecDeque<Value>) -> Result<Value, Error> {
    let boosts = args
        .pop_front()
        .wrap_error_with_message("missing boosts")?
        .boost_table()
        .wrap_error_with_message("invalid boosts")?;
    let boost = args
        .pop_front()
        .wrap_error_with_message("missing boost")?
        .boost()
        .wrap_error_with_message("invalid boost")?;
    Ok(Value::I64(boosts.get(boost) as i64))
}

fn set_boost(mut args: VecDeque<Value>) -> Result<Value, Error> {
    let mut boosts = args
        .pop_front()
        .wrap_error_with_message("missing boosts")?
        .boost_table()
        .wrap_error_with_message("invalid boosts")?;
    let boost = args
        .pop_front()
        .wrap_error_with_message("missing boost")?
        .boost()
        .wrap_error_with_message("invalid boost")?;
    let value = args
        .pop_front()
        .wrap_error_with_message("missing boost value")?
        .integer_i8()
        .wrap_error_with_message("invalid boost value")?;
    boosts.set(boost, value);
    Ok(Value::BoostTable(boosts))
}

fn has_type(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<Value, Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let typ = args
        .pop_front()
        .wrap_error_with_message("missing type")?
        .mon_type()
        .wrap_error_with_message("invalid type")?;
    Mon::has_type(&mut context.mon_context(mon_handle)?, typ).map(|val| Value::Boolean(val))
}

fn mon_in_position(
    context: &mut EvaluationContext,
    mut args: VecDeque<Value>,
) -> Result<Option<Value>, Error> {
    let side_index = args
        .pop_front()
        .wrap_error_with_message("missing side")?
        .side_index()
        .wrap_error_with_message("invalid side")?;
    let position = args
        .pop_front()
        .wrap_error_with_message("missing position")?
        .integer_usize()
        .wrap_error_with_message("invalid position")?;
    Ok(Side::mon_in_position(
        &mut context.battle_context_mut().side_context(side_index)?,
        position,
    )?
    .map(|mon| Value::Mon(mon)))
}

fn disable_move(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let move_id = args
        .pop_front()
        .wrap_error_with_message("missing move")?
        .string()
        .wrap_error_with_message("invalid move")?;
    let move_id = Id::from(move_id);
    Mon::disable_move(&mut context.mon_context(mon_handle)?, &move_id)
}

fn volatile_effect_state(
    context: &mut EvaluationContext,
    mut args: VecDeque<Value>,
) -> Result<Option<Value>, Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let volatile_id = args
        .pop_front()
        .wrap_error_with_message("missing volatile")?
        .string()
        .wrap_error_with_message("invalid volatile")?;
    let volatile_id = Id::from(volatile_id);
    Ok(context
        .mon_context(mon_handle)?
        .mon()
        .volatiles
        .get(&volatile_id)
        .map(|effect_state| Value::from(effect_state.clone())))
}

struct StatBoost(Boost, i8);

impl FromStr for StatBoost {
    type Err = Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (boost, amount) = s.split_once(':').wrap_error_with_message("invalid boost")?;
        let boost = Boost::from_str(boost).wrap_error_with_message("invalid boost stat")?;
        let amount =
            i8::from_str_radix(amount, 10).wrap_error_with_message("invalid boost amount")?;
        Ok(Self(boost, amount))
    }
}

fn boosts_from_rest_of_args(args: VecDeque<Value>) -> Result<BoostTable, Error> {
    let boosts = args
        .into_iter()
        .map(|boost| StatBoost::from_str(&boost.string()?))
        .map(|res| res.wrap_error_with_message("invalid boost"))
        .collect::<Result<Vec<_>, Error>>()?;
    Ok(BoostTable::from_iter(
        boosts.into_iter().map(|boost| (boost.0, boost.1)),
    ))
}

fn boost(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<Value, Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let boosts = boosts_from_rest_of_args(args)?;
    core_battle_actions::boost(
        &mut context.mon_context(mon_handle)?,
        boosts,
        None,
        None,
        false,
        false,
    )
    .map(|val| Value::Boolean(val))
}

fn can_switch(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<Value, Error> {
    let player_index = args
        .pop_front()
        .wrap_error_with_message("missing player")?
        .player_index()
        .wrap_error_with_message("invalid player")?;
    Ok(Value::Boolean(Player::can_switch(
        &mut context.battle_context_mut().player_context(player_index)?,
    )))
}

fn has_move(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<Value, Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let move_id = args
        .pop_front()
        .wrap_error_with_message("missing move")?
        .string()
        .wrap_error_with_message("invalid move")?;
    Ok(Value::Boolean(
        context
            .mon(mon_handle)?
            .move_slot_index(&Id::from(move_id))
            .is_some(),
    ))
}

fn move_slot_index(
    context: &mut EvaluationContext,
    mut args: VecDeque<Value>,
) -> Result<Option<Value>, Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let move_id = args
        .pop_front()
        .wrap_error_with_message("missing move")?
        .string()
        .wrap_error_with_message("invalid move")?;
    match context.mon(mon_handle)?.move_slot_index(&Id::from(move_id)) {
        Some(index) => Ok(Some(Value::U64(
            index
                .try_into()
                .wrap_error_with_message("integer overflow")?,
        ))),
        None => Ok(None),
    }
}

fn move_slot(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<Value, Error> {
    let active_move_handle = args
        .pop_front()
        .wrap_error_with_message("missing active move")?
        .active_move()
        .wrap_error_with_message("invalid active move")?;
    let active_move = context.active_move(active_move_handle)?;
    let move_slot = MoveSlot::new_simulated(
        active_move.id().clone(),
        active_move.data.name.clone(),
        active_move.data.pp,
        active_move.data.pp,
        active_move.data.target,
    );
    Ok(Value::MoveSlot(move_slot))
}

fn overwrite_move_slot(
    context: &mut EvaluationContext,
    mut args: VecDeque<Value>,
) -> Result<(), Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let index = args
        .pop_front()
        .wrap_error_with_message("missing index")?
        .integer_usize()
        .wrap_error_with_message("invalid index")?;
    let move_slot = args
        .pop_front()
        .wrap_error_with_message("missing move slot")?
        .move_slot()
        .wrap_error_with_message("invalid move slot")?;

    context
        .mon_context(mon_handle)?
        .mon_mut()
        .overwrite_move_slot(index, move_slot)
}

fn mons_per_side(context: &mut EvaluationContext) -> Result<Value, Error> {
    Ok(Value::U64(
        context
            .battle_context()
            .battle()
            .max_side_length()
            .try_into()
            .wrap_error_with_message("integer overflow")?,
    ))
}

fn move_crit_target(
    context: &mut EvaluationContext,
    mut args: VecDeque<Value>,
) -> Result<Value, Error> {
    let active_move_handle = args
        .pop_front()
        .wrap_error_with_message("missing active move")?
        .active_move()
        .wrap_error_with_message("invalid active move")?;
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    Ok(Value::Boolean(
        context
            .active_move(active_move_handle)?
            .maybe_hit_data(mon_handle)
            .map(|hit_data| hit_data.crit)
            .unwrap_or(false),
    ))
}

fn all_active_mons(context: &mut EvaluationContext) -> Result<Value, Error> {
    Ok(Value::List(
        context
            .battle_context()
            .battle()
            .all_active_mon_handles()
            .map(|mon_handle| Value::Mon(mon_handle))
            .collect(),
    ))
}

fn clear_boosts(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    context.mon_context(mon_handle)?.mon_mut().clear_boosts();
    Ok(())
}

fn random_target(
    context: &mut EvaluationContext,
    mut args: VecDeque<Value>,
) -> Result<Option<Value>, Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let move_target = match args.pop_front() {
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

fn new_active_move_from_local_data(
    context: &mut EvaluationContext,
    mut args: VecDeque<Value>,
) -> Result<Value, Error> {
    let move_id = args
        .pop_front()
        .wrap_error_with_message("missing move")?
        .string()
        .wrap_error_with_message("invalid move")?;
    let move_id = Id::from(move_id);
    let move_data = context
        .effect_context()
        .effect()
        .fxlang_effect()
        .wrap_error_with_message("effect does not have local data")?
        .local_data
        .moves
        .get(&move_id)
        .wrap_error_with_format(format_args!(
            "move {move_id} does not exist in the effect's local data"
        ))?
        .clone();
    let active_move = Move::new_unlinked(move_id, move_data);
    let active_move_handle =
        core_battle_actions::register_active_move(context.battle_context_mut(), active_move)?;
    Ok(Value::ActiveMove(active_move_handle))
}

fn use_active_move(
    context: &mut EvaluationContext,
    mut args: VecDeque<Value>,
) -> Result<Value, Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let active_move_handle = args
        .pop_front()
        .wrap_error_with_message("missing active move")?
        .active_move()
        .wrap_error_with_message("invalid active move")?;
    let target_handle = match args.pop_front() {
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
    let source_effect = context.source_effect_handle().cloned();
    core_battle_actions::use_active_move(
        &mut context.mon_context(mon_handle)?,
        active_move_handle,
        target_handle,
        source_effect.as_ref(),
        true,
    )
    .map(|val| Value::Boolean(val))
}

fn use_move(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<Value, Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let move_id = args
        .pop_front()
        .wrap_error_with_message("missing move")?
        .move_id(context)
        .wrap_error_with_message("invalid move")?;
    let target_handle = match args.pop_front() {
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
    let source_effect = context.source_effect_handle().cloned();
    core_battle_actions::use_move(
        &mut context.mon_context(mon_handle)?,
        &move_id,
        target_handle,
        source_effect.as_ref(),
        true,
    )
    .map(|val| Value::Boolean(val))
}

fn mon_at_target_location(
    context: &mut EvaluationContext,
    mut args: VecDeque<Value>,
) -> Result<Option<Value>, Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let relative_location = args
        .pop_front()
        .wrap_error_with_message("missing relative location")?
        .integer_isize()
        .wrap_error_with_message("invalid relative location")?;
    Mon::get_target(&mut context.mon_context(mon_handle)?, relative_location)
        .map(|mon| Some(Value::Mon(mon?)))
}

fn target_location_of_mon(
    context: &mut EvaluationContext,
    mut args: VecDeque<Value>,
) -> Result<Value, Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let target_handle = args
        .pop_front()
        .wrap_error_with_message("missing target")?
        .mon_handle()
        .wrap_error_with_message("invalid target")?;
    Ok(Value::I64(
        Mon::get_target_location(&mut context.mon_context(mon_handle)?, target_handle)?
            .try_into()
            .wrap_error_with_message("integer overflow")?,
    ))
}

fn get_move(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<Value, Error> {
    let move_id = args
        .pop_front()
        .wrap_error_with_message("missing move id")?
        .move_id(context)
        .wrap_error_with_message("invalid move id")?;
    Ok(Value::Effect(EffectHandle::InactiveMove(move_id)))
}

fn get_all_moves(
    context: &mut EvaluationContext,
    mut args: VecDeque<Value>,
) -> Result<Value, Error> {
    let mut with_flags = FastHashSet::new();
    let mut without_flags = FastHashSet::new();
    while let Some(arg) = args.pop_front() {
        match arg
            .string()
            .wrap_error_with_message("invalid filter")?
            .split_once(':')
        {
            Some(("with_flag", flag)) => with_flags
                .insert(MoveFlags::from_str(flag).wrap_error_with_message("invalid move flag")?),
            Some(("without_flag", flag)) => without_flags
                .insert(MoveFlags::from_str(flag).wrap_error_with_message("invalid move flag")?),
            _ => return Err(battler_error!("invalid filter")),
        };
    }

    let mut moves = context
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

fn move_at_move_slot_index(
    context: &mut EvaluationContext,
    mut args: VecDeque<Value>,
) -> Result<Option<Value>, Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let index = args
        .pop_front()
        .wrap_error_with_message("missing index")?
        .integer_usize()
        .wrap_error_with_message("invalid index")?;
    let context = context.mon_context(mon_handle)?;
    Ok(context
        .mon()
        .move_slots
        .get(index)
        .map(|move_slot| Value::Effect(EffectHandle::InactiveMove(move_slot.id.clone()))))
}

fn set_types(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<Value, Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let typ = args
        .pop_front()
        .wrap_error_with_message("missing type")?
        .mon_type()
        .wrap_error_with_message("invalid type")?;
    let mut context = context.forward_effect_to_applying_effect(mon_handle)?;
    core_battle_actions::set_types(&mut context, Vec::from_iter([typ]))
        .map(|val| Value::Boolean(val))
}

fn set_weather(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<Value, Error> {
    let weather = args
        .pop_front()
        .wrap_error_with_message("missing weather")?
        .string()
        .wrap_error_with_message("invalid weather")?;
    let weather = Id::from(weather);
    core_battle_actions::set_weather(&mut context.forward_effect_to_field_effect()?, &weather)
        .map(Value::Boolean)
}

fn transform_into(
    context: &mut EvaluationContext,
    mut args: VecDeque<Value>,
) -> Result<Value, Error> {
    let with_source_effect = has_special_string_flag(&mut args, "with_source_effect");

    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let target_handle = args
        .pop_front()
        .wrap_error_with_message("missing target")?
        .mon_handle()
        .wrap_error_with_message("invalid target")?;

    let mut context = context.forward_effect_to_applying_effect(mon_handle)?;
    core_battle_actions::transform_into(&mut context, target_handle, with_source_effect)
        .map(|val| Value::Boolean(val))
}

fn can_escape(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<Value, Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    Mon::can_escape(&mut context.mon_context(mon_handle)?).map(|val| Value::Boolean(val))
}

fn escape(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<Value, Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    core_battle_actions::try_escape(&mut context.mon_context(mon_handle)?, true)
        .map(|val| Value::Boolean(val))
}
