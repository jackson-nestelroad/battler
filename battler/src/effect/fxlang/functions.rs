use std::{
    collections::VecDeque,
    str::FromStr,
};

use crate::{
    battle::{
        core_battle_actions,
        core_battle_effects,
        core_battle_logs,
        Boost,
        Context,
        Mon,
        MonContext,
        MoveOutcomeOnTarget,
        Side,
    },
    battler_error,
    common::{
        Error,
        Id,
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
    },
    log::Event,
    log_event,
    moves::MoveFlags,
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
        "apply_drain" => apply_drain(context, args).map(|()| None),
        "apply_recoil_damage" => apply_recoil_damage(context, args).map(|()| None),
        "boostable_stats" => Ok(Some(boostable_stats())),
        "calculate_damage" => calculate_damage(context, args).map(|val| Some(val)),
        "calculate_confusion_damage" => {
            calculate_confusion_damage(context, args).map(|val| Some(val))
        }
        "chance" => chance(context.battle_context_mut(), args).map(|val| Some(val)),
        "cure_status" => cure_status(context, args).map(|()| None),
        "damage" => damage(context, args).map(|val| Some(val)),
        "debug_log" => debug_log(context.battle_context_mut(), args).map(|()| None),
        "direct_damage" => direct_damage(context, args).map(|()| None),
        "do_not_animate_last_move" => {
            do_not_animate_last_move(context.battle_context_mut()).map(|()| None)
        }
        "floor" => floor(args).map(|val| Some(val)),
        "get_boost" => get_boost(args).map(|val| Some(val)),
        "has_ability" => has_ability(context, args).map(|val| Some(val)),
        "has_type" => has_type(context, args).map(|val| Some(val)),
        "has_volatile" => has_volatile(context, args).map(|val| Some(val)),
        "heal" => heal(context, args).map(|()| None),
        "is_ally" => is_ally(context, args).map(|val| Some(val)),
        "is_boolean" => is_boolean(args).map(|val| Some(val)),
        "is_defined" => is_defined(args).map(|val| Some(val)),
        "log" => log(context.battle_context_mut(), args).map(|()| None),
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
        "max" => max(args).map(|val| Some(val)),
        "mon_in_position" => mon_in_position(context, args),
        "move_has_flag" => move_has_flag(context, args).map(|val| Some(val)),
        "random" => random(context.battle_context_mut(), args).map(|val| Some(val)),
        "remove_volatile" => remove_volatile(context, args).map(|val| Some(val)),
        "run_event" => run_event(context, args).map(|val| Some(val)),
        "run_event_on_move" => run_event_on_move(context, args).map(|()| None),
        "trap" => trap_mon(context, args).map(|()| None),
        "set_boost" => set_boost(args).map(|val| Some(val)),
        "set_status" => set_status(context, args).map(|val| Some(val)),
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

fn debug_log(context: &mut Context, args: VecDeque<Value>) -> Result<(), Error> {
    let mut event = log_event!("fxlang_debug");
    for (i, arg) in args.into_iter().enumerate() {
        event.set(format!("arg{i}"), format!("{arg:?}"));
    }
    context.battle_mut().log(event);
    Ok(())
}

fn log_internal(context: &mut Context, title: String, args: VecDeque<Value>) -> Result<(), Error> {
    let mut event = Event::new(title);
    for arg in args {
        let entry = arg.string().wrap_error_with_message("invalid log entry")?;
        match entry.split_once(':') {
            None => event.extend(&entry.as_str()),
            Some((a, b)) => event.extend(&(a, b)),
        }
    }
    context.battle_mut().log(event);
    Ok(())
}

fn log(context: &mut Context, mut args: VecDeque<Value>) -> Result<(), Error> {
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

    log_internal(context.battle_context_mut(), "activate".to_owned(), args)
}

fn log_start(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    add_effect_to_args(context, &mut args)?;

    args.push_front(Value::String(format!(
        "mon:{}",
        Mon::position_details(&mut context.target_context()?)?
    )));

    log_internal(context.battle_context_mut(), "start".to_owned(), args)
}

fn log_end(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    add_effect_to_args(context, &mut args)?;

    args.push_front(Value::String(format!(
        "mon:{}",
        Mon::position_details(&mut context.target_context()?)?
    )));

    log_internal(context.battle_context_mut(), "end".to_owned(), args)
}

fn log_side_start(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let side_index = context
        .side_index()
        .wrap_error_with_message("context has no side index")?;
    let condition = args
        .pop_front()
        .wrap_error_with_message("missing side condition name")?
        .string()
        .wrap_error_with_message("invalid side condition name")?;
    args.push_front(Value::String(format!("what:{condition}")));
    args.push_front(Value::String(format!("side:{side_index}")));

    log_internal(context.battle_context_mut(), "sidestart".to_owned(), args)
}

fn log_side_end(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let side_index = context
        .side_index()
        .wrap_error_with_message("context has no side index")?;
    let condition = args
        .pop_front()
        .wrap_error_with_message("missing side condition name")?
        .string()
        .wrap_error_with_message("invalid side condition name")?;
    args.push_front(Value::String(format!("what:{condition}")));
    args.push_front(Value::String(format!("side:{side_index}")));

    log_internal(context.battle_context_mut(), "sideend".to_owned(), args)
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

fn random(context: &mut Context, mut args: VecDeque<Value>) -> Result<Value, Error> {
    let a = args.pop_front().map(|val| val.integer_u64().ok()).flatten();
    let b = args.pop_front().map(|val| val.integer_u64().ok()).flatten();
    let val = match (a, b) {
        (None, None) => context.battle_mut().prng.next(),
        (Some(max), None) => rand_util::range(context.battle_mut().prng.as_mut(), 0, max),
        (Some(min), Some(max)) => rand_util::range(context.battle_mut().prng.as_mut(), min, max),
        _ => return Err(battler_error!("invalid random arguments")),
    };
    Ok(Value::U64(val))
}

fn chance(context: &mut Context, mut args: VecDeque<Value>) -> Result<Value, Error> {
    let a = args.pop_front().map(|val| val.integer_u64().ok()).flatten();
    let b = args.pop_front().map(|val| val.integer_u64().ok()).flatten();
    let val = match (a, b) {
        (None, None) => return Err(battler_error!("chance requires at least one argument")),
        (Some(den), None) => rand_util::chance(context.battle_mut().prng.as_mut(), 1, den),
        (Some(num), Some(den)) => rand_util::chance(context.battle_mut().prng.as_mut(), num, den),
        _ => return Err(battler_error!("invalid chance arguments")),
    };
    Ok(Value::Boolean(val))
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
    let mut context =
        context.maybe_source_applying_effect_context(should_use_source_effect(&mut args))?;
    let mut context = context.change_target_context(mon_handle)?;
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
    let mut context =
        context.maybe_source_applying_effect_context(should_use_source_effect(&mut args))?;
    let mut context = context.change_target_context(mon_handle)?;
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
    let mut context =
        context.maybe_source_applying_effect_context(should_use_source_effect(&mut args))?;
    let mut context = context.change_target_context(mon_handle)?;
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
    let mut context =
        context.maybe_source_applying_effect_context(should_use_source_effect(&mut args))?;
    Ok(Value::Boolean(
        core_battle_effects::run_event_for_applying_effect(
            context.as_mut(),
            event,
            VariableInput::default(),
        ),
    ))
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
    core_battle_effects::run_active_move_event_expecting_void(&mut context, event, target);
    Ok(())
}

fn do_not_animate_last_move(context: &mut Context) -> Result<(), Error> {
    core_battle_logs::do_not_animate_last_move(context);
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
        MoveOutcomeOnTarget::Success => Ok(Value::U64(0)),
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
        false,
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
    let mut context =
        context.maybe_source_applying_effect_context(should_use_source_effect(&mut args))?;
    let mut context = context.change_target_context(mon_handle)?;
    core_battle_actions::try_set_status(&mut context, Some(status), false)
        .map(|val| Value::Boolean(val))
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
        [
            Boost::Atk,
            Boost::Def,
            Boost::SpAtk,
            Boost::SpDef,
            Boost::Spe,
            Boost::Accuracy,
            Boost::Evasion,
        ]
        .map(|boost| Value::Boost(boost)),
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
