use std::collections::VecDeque;

use crate::{
    battle::Context,
    battler_error,
    common::{
        Error,
        WrapResultError,
    },
    effect::fxlang::{
        EvaluationContext,
        Value,
    },
    log::Event,
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
        "log" => log(context.battle_context_mut(), args).map(|()| None),
        "random" => random(context.battle_context_mut(), args).map(|val| Some(val)),
        "chance" => chance(context.battle_context_mut(), args).map(|val| Some(val)),
        _ => Err(battler_error!("undefined function: {function_name}")),
    }
}

fn log(context: &mut Context, mut args: VecDeque<Value>) -> Result<(), Error> {
    let title = args
        .pop_front()
        .wrap_error_with_message("missing log title")?
        .string()
        .wrap_error_with_message("invalid title")?;
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

fn random(context: &mut Context, mut args: VecDeque<Value>) -> Result<Value, Error> {
    let a = args.pop_front().map(|val| val.integer_u64().ok()).flatten();
    let b = args.pop_front().map(|val| val.integer_u64().ok()).flatten();
    let val = match (a, b) {
        (None, None) => context.battle_mut().prng.next(),
        (Some(max), None) => rand_util::range(context.battle_mut().prng.as_mut(), 0, max),
        (Some(min), Some(max)) => rand_util::range(context.battle_mut().prng.as_mut(), min, max),
        _ => return Err(battler_error!("impossible arguments")),
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
        _ => return Err(battler_error!("impossible arguments")),
    };
    Ok(Value::Boolean(val))
}
