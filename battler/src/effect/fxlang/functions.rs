use std::{
    collections::VecDeque,
    str::FromStr,
};

use crate::{
    battle::{
        core_battle_actions,
        core_battle_effects,
        core_battle_logs,
        ActiveMoveContext,
        ApplyingEffectContext,
        Context,
        Mon,
        MonContext,
        MoveOutcomeOnTarget,
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
        "activate" => activate(context, args).map(|()| None),
        "add_volatile" => {
            add_volatile(context.applying_effect_context_mut()?.as_mut(), args).map(|val| Some(val))
        }
        "apply_drain" => apply_drain(context, args).map(|()| None),
        "apply_recoil_damage" => {
            apply_recoil_damage(context.active_move_context_mut()?.as_mut(), args).map(|()| None)
        }
        "calculate_damage" => {
            calculate_damage(context.active_move_context_mut()?.as_mut(), args).map(|val| Some(val))
        }
        "calculate_confusion_damage" => {
            calculate_confusion_damage(context, args).map(|val| Some(val))
        }
        "cant" => log_cant(context.target_context_mut()?.as_mut(), args).map(|()| None),
        "chance" => chance(context.battle_context_mut(), args).map(|val| Some(val)),
        "cure_status" => {
            cure_status(context.applying_effect_context_mut()?.as_mut(), args).map(|()| None)
        }
        "damage" => damage(context, args).map(|()| None),
        "debug_log" => debug_log(context.battle_context_mut(), args).map(|()| None),
        "direct_damage" => direct_damage(context, args).map(|()| None),
        "do_not_animate_last_move" => {
            do_not_animate_last_move(context.battle_context_mut()).map(|()| None)
        }
        "end" => end(context.target_context_mut()?.as_mut(), args).map(|()| None),
        "fail" => fail(context, args).map(|()| None),
        "floor" => floor(args).map(|val| Some(val)),
        "has_ability" => has_ability(context, args).map(|val| Some(val)),
        "has_volatile" => has_volatile(context, args).map(|val| Some(val)),
        "heal" => heal(context, args).map(|()| None),
        "is_boolean" => is_boolean(args).map(|val| Some(val)),
        "log" => log(context.battle_context_mut(), args).map(|()| None),
        "log_status" => {
            log_status(context.applying_effect_context_mut()?.as_mut(), args).map(|()| None)
        }
        "max" => max(args).map(|val| Some(val)),
        "move_has_flag" => move_has_flag(context, args).map(|val| Some(val)),
        "ohko" => ohko(context, args).map(|()| None),
        "prepare_move" => prepare_move(context.active_move_context_mut()?.as_mut()).map(|()| None),
        "random" => random(context.battle_context_mut(), args).map(|val| Some(val)),
        "remove_volatile" => remove_volatile(context.applying_effect_context_mut()?.as_mut(), args)
            .map(|val| Some(val)),
        "run_event" => {
            run_event(context.applying_effect_context_mut()?.as_mut(), args).map(|val| Some(val))
        }
        "run_event_on_move" => {
            run_event_on_move(context.active_move_context_mut()?.as_mut(), args).map(|()| None)
        }
        "start" => start(context.target_context_mut()?.as_mut(), args).map(|()| None),
        "trap" => trap_mon(context, args).map(|()| None),
        "set_status" => {
            set_status(context.applying_effect_context_mut()?.as_mut(), args).map(|val| Some(val))
        }
        _ => Err(battler_error!("undefined function: {function_name}")),
    }
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

fn activate(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let mut with_target = false;
    let mut with_source = false;

    if let Some(value) = args.front().cloned() {
        if value.string().is_ok_and(|value| value == "with_target") {
            with_target = true;
            args.pop_front();
        }
    }
    if let Some(value) = args.front().cloned() {
        if value.string().is_ok_and(|value| value == "with_source") {
            with_source = true;
            args.pop_front();
        }
    }

    match context {
        EvaluationContext::ActiveMove(context) => {
            args.push_front(Value::String(format!(
                "move:{}",
                context.active_move().data.name
            )));
        }
        EvaluationContext::ApplyingEffect(context) => match context.effect() {
            Effect::Ability(ability) => {
                args.push_front(Value::String(format!("ability:{}", ability.data.name)))
            }
            _ => (),
        },
        _ => (),
    }

    if with_target {
        args.push_front(Value::String(format!(
            "mon:{}",
            Mon::position_details(context.target_context_mut()?.as_ref())?
        )));
    }
    if with_source {
        args.push_back(Value::String(format!(
            "of:{}",
            Mon::position_details(context.source_context_mut()?.as_ref())?
        )));
    }

    log_internal(context.battle_context_mut(), "activate".to_owned(), args)
}

fn start(context: &mut MonContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let status = args
        .pop_front()
        .wrap_error_with_message("missing status name")?
        .string()
        .wrap_error_with_message("invalid status name")?;
    args.push_front(Value::String(format!("what:{status}")));
    args.push_front(Value::String(format!(
        "mon:{}",
        Mon::position_details(context)?
    )));

    log_internal(context.as_battle_context_mut(), "start".to_owned(), args)
}

fn end(context: &mut MonContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let status = args
        .pop_front()
        .wrap_error_with_message("missing status name")?
        .string()
        .wrap_error_with_message("invalid status name")?;
    args.push_front(Value::String(format!("what:{status}")));
    args.push_front(Value::String(format!(
        "mon:{}",
        Mon::position_details(context)?
    )));

    log_internal(context.as_battle_context_mut(), "end".to_owned(), args)
}

fn prepare_move(context: &mut ActiveMoveContext) -> Result<(), Error> {
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

fn log_status(context: &mut ApplyingEffectContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let status = args
        .pop_front()
        .wrap_error_with_message("missing status name")?
        .string()
        .wrap_error_with_message("invalid status name")?;

    let mut with_effect = false;
    if let Some(value) = args.front().cloned() {
        if value.string().is_ok_and(|value| value == "with_effect") {
            with_effect = true;
            args.pop_front();
        }
    }

    let mut event = log_event!(
        "status",
        ("mon", Mon::position_details(&context.target_context()?)?),
        ("status", status)
    );
    if with_effect {
        event.set("from", context.effect().full_name());
        if context.effect_handle().is_ability() {
            if let Some(source_context) = context.source_context()? {
                event.set("of", Mon::position_details(&source_context)?);
            }
        }
    }
    context.battle_mut().log(event);
    Ok(())
}

fn fail(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    core_battle_logs::fail(context.mon_context_mut(mon_handle)?.as_mut())
}

fn ohko(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    core_battle_logs::ohko(context.mon_context_mut(mon_handle)?.as_mut())
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

fn damage(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let effect_handle = context.effect_handle();

    let mut source_handle = context.source_handle();
    if let Some(value) = args.front().cloned() {
        if value.string().is_ok_and(|value| value == "no_source") {
            source_handle = None;
            args.pop_front();
        }
    }

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
    let damaging_effect = match args.pop_front() {
        Some(value) => Some(
            value
                .effect_handle()
                .wrap_error_with_message("invalid damaging effect")?,
        ),
        None => effect_handle,
    };
    core_battle_actions::damage(
        context.mon_context_mut(target_handle)?.as_mut(),
        amount,
        source_handle,
        damaging_effect.as_ref(),
    )
}

fn direct_damage(context: &mut EvaluationContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let effect_handle = context.effect_handle();

    let mut source_handle = context.source_handle();
    if let Some(value) = args.front().cloned() {
        if value.string().is_ok_and(|value| value == "no_source") {
            source_handle = None;
            args.pop_front();
        }
    }

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
    let damaging_effect = match args.pop_front() {
        Some(value) => Some(
            value
                .effect_handle()
                .wrap_error_with_message("invalid damaging effect")?,
        ),
        None => effect_handle,
    };
    core_battle_actions::direct_damage(
        context.mon_context_mut(target_handle)?.as_mut(),
        amount,
        source_handle,
        damaging_effect.as_ref(),
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
    Mon::has_ability(context.mon_context_mut(mon_handle)?.as_mut(), &ability)
        .map(|val| Value::Boolean(val))
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
    Mon::has_volatile(context.mon_context_mut(mon_handle)?.as_mut(), &volatile)
        .map(|val| Value::Boolean(val))
}

fn cure_status(
    context: &mut ApplyingEffectContext,
    mut args: VecDeque<Value>,
) -> Result<(), Error> {
    let mon_handle = args
        .pop_front()
        .wrap_error_with_message("missing mon")?
        .mon_handle()
        .wrap_error_with_message("invalid mon")?;
    let log_effect = args
        .pop_front()
        .unwrap_or(Value::Boolean(false))
        .boolean()
        .wrap_error_with_message("second parameter must be a boolean")?;
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
    context: &mut ApplyingEffectContext,
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
    let mut context = context.change_target_context(mon_handle)?;
    core_battle_actions::try_add_volatile(&mut context, &volatile, false)
        .map(|val| Value::Boolean(val))
}

fn remove_volatile(
    context: &mut ApplyingEffectContext,
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
    let no_events = match args.pop_front() {
        Some(value) => value
            .boolean()
            .wrap_error_with_message("invalid no_events argument")?,
        _ => false,
    };
    let volatile = Id::from(volatile);
    let mut context = context.change_target_context(mon_handle)?;
    core_battle_actions::remove_volatile(&mut context, &volatile, no_events)
        .map(|val| Value::Boolean(val))
}

fn run_event(
    context: &mut ApplyingEffectContext,
    mut args: VecDeque<Value>,
) -> Result<Value, Error> {
    let event = args
        .pop_front()
        .wrap_error_with_message("missing event")?
        .string()
        .wrap_error_with_message("invalid event")?;
    let event = BattleEvent::from_str(&event).wrap_error_with_message("invalid event")?;
    Ok(Value::Boolean(
        core_battle_effects::run_event_for_applying_effect(
            context,
            event,
            VariableInput::default(),
        ),
    ))
}

fn run_event_on_move(
    context: &mut ActiveMoveContext,
    mut args: VecDeque<Value>,
) -> Result<(), Error> {
    let event = args
        .pop_front()
        .wrap_error_with_message("missing event")?
        .string()
        .wrap_error_with_message("invalid event")?;
    let event = BattleEvent::from_str(&event).wrap_error_with_message("invalid event")?;
    core_battle_effects::run_active_move_event_expecting_void(context, event);
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
    core_battle_actions::trap_mon(context.mon_context_mut(mon_handle)?.as_mut())
}

fn calculate_damage(
    context: &mut ActiveMoveContext,
    mut args: VecDeque<Value>,
) -> Result<Value, Error> {
    let target_handle = args
        .pop_front()
        .wrap_error_with_message("missing target")?
        .mon_handle()
        .wrap_error_with_message("invalid target")?;
    match core_battle_actions::calculate_damage(&mut context.target_context(target_handle)?)? {
        MoveOutcomeOnTarget::Damage(damage) => Ok(Value::U16(damage)),
        MoveOutcomeOnTarget::Success => Ok(Value::U16(0)),
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
        context.mon_context_mut(mon_handle)?.as_mut(),
        base_power,
    )
    .map(|value| Value::U16(value))
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
    let source_handle = context.source_handle();
    let effect = context.effect_handle();
    core_battle_actions::heal(
        context.mon_context_mut(mon_handle)?.as_mut(),
        damage,
        source_handle,
        effect.as_ref(),
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
    let effect_handle = context
        .effect_handle()
        .wrap_error_with_message("cannot drain outside of an effect")?;
    core_battle_actions::apply_drain(
        &mut context
            .battle_context_mut()
            .effect_context(&effect_handle)?
            .applying_effect_context(Some(source_handle), target_handle)?,
        damage,
    )
}

fn apply_recoil_damage(
    context: &mut ActiveMoveContext,
    mut args: VecDeque<Value>,
) -> Result<(), Error> {
    let damage = args
        .pop_front()
        .wrap_error_with_message("missing damage")?
        .integer_u64()
        .wrap_error_with_message("invalid damage")?;
    core_battle_actions::apply_recoil_damage(context, damage)
}

fn is_boolean(mut args: VecDeque<Value>) -> Result<Value, Error> {
    let value = args.pop_front().wrap_error_with_message("missing value")?;
    Ok(Value::Boolean(value.boolean().is_ok()))
}

fn set_status(
    context: &mut ApplyingEffectContext,
    mut args: VecDeque<Value>,
) -> Result<Value, Error> {
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
    let mut context = context.change_target_context(mon_handle)?;
    core_battle_actions::try_set_status(&mut context, Some(status), false)
        .map(|val| Value::Boolean(val))
}
