use std::collections::VecDeque;

use crate::{
    battle::{
        core_battle_actions,
        core_battle_logs,
        ActiveMoveContext,
        ApplyingEffectContext,
        Context,
        Mon,
        MonContext,
        MonHandle,
    },
    battler_error,
    common::{
        Error,
        Id,
        WrapResultError,
    },
    effect::{
        fxlang::{
            EvaluationContext,
            Value,
        },
        EffectHandle,
    },
    log::Event,
    log_event,
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
        "debug_log" => debug_log(context.battle_context_mut(), args).map(|()| None),
        "log" => log(context.battle_context_mut(), args).map(|()| None),
        "activate" => match context {
            EvaluationContext::ActiveMove(context) => activate_move(context, args).map(|()| None),
            _ => Err(battler_error!(
                "activate can only be called on an active move context"
            )),
        },
        "cant" => log_cant(context.target_context_mut()?.as_mut(), args).map(|()| None),
        "log_status" => {
            log_status(context.applying_effect_context_mut()?.as_mut(), args, false).map(|()| None)
        }
        "log_status_with_effect" => {
            log_status(context.applying_effect_context_mut()?.as_mut(), args, true).map(|()| None)
        }
        "random" => random(context.battle_context_mut(), args).map(|val| Some(val)),
        "chance" => chance(context.battle_context_mut(), args).map(|val| Some(val)),
        "damage" => {
            let source_handle = context.source_handle();
            let effect_handle = context.effect_handle();
            damage(
                context.target_context_mut()?.as_mut(),
                source_handle,
                effect_handle,
                args,
            )
            .map(|()| None)
        }
        "has_ability" => has_ability(context.battle_context_mut(), args).map(|val| Some(val)),
        _ => Err(battler_error!("undefined function: {function_name}")),
    }
}

fn debug_log(context: &mut Context, args: VecDeque<Value>) -> Result<(), Error> {
    let mut event = log_event!("fxlang_debug");
    for (i, arg) in args.into_iter().enumerate() {
        event.set(
            format!("arg{i}"),
            arg.string().unwrap_or("not a string".to_owned()),
        );
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

fn activate_move(context: &mut ActiveMoveContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    args.push_front(Value::String(format!(
        "move:{}",
        context.active_move().data.name
    )));
    log_internal(context.as_battle_context_mut(), "activate".to_owned(), args)
}

fn log_cant(context: &mut MonContext, mut args: VecDeque<Value>) -> Result<(), Error> {
    let reason = args
        .pop_front()
        .wrap_error_with_message("missing reason")?
        .string()?;
    core_battle_logs::cant(context, &reason, None)
}

fn log_status(
    context: &mut ApplyingEffectContext,
    mut args: VecDeque<Value>,
    log_effect: bool,
) -> Result<(), Error> {
    let status = args
        .pop_front()
        .wrap_error_with_message("missing status id")?
        .string()?;
    let mut event = log_event!(
        "status",
        ("mon", Mon::position_details(&context.target_context()?)?),
        ("status", status)
    );
    if log_effect {
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

fn damage(
    context: &mut MonContext,
    source_handle: Option<MonHandle>,
    effect_handle: Option<EffectHandle>,
    mut args: VecDeque<Value>,
) -> Result<(), Error> {
    let amount = args
        .pop_front()
        .wrap_error_with_message("missing damage amount")?
        .integer_u16()
        .wrap_error_with_message("invalid damage amount")?;
    core_battle_actions::damage(context, amount, source_handle, effect_handle.as_ref())
}

fn has_ability(context: &mut Context, mut args: VecDeque<Value>) -> Result<Value, Error> {
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
    let mut context = context.mon_context(mon_handle)?;
    Mon::has_ability(&mut context, &ability).map(|val| Value::Boolean(val))
}
