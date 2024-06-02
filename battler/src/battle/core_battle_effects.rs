use super::core_battle_logs;
use crate::{
    battle::ActiveMoveContext,
    common::{
        Error,
        UnsafelyDetachBorrow,
        UnsafelyDetachBorrowMut,
        WrapResultError,
    },
    effect::{
        fxlang,
        Effect,
        EffectManager,
    },
};

fn run_active_move_event_with_errors(
    context: &mut ActiveMoveContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> Result<Option<fxlang::Value>, Error> {
    // SAFETY: The active move lives for the lifetime of the turn.
    let active_move = unsafe { context.active_move_mut().unsafely_detach_borrow_mut() };
    let effect_state = active_move.effect_state.clone();
    let effect = Effect::for_active_move(active_move);
    let result = EffectManager::evaluate_under_active_move_context(
        context,
        &effect,
        event,
        input,
        Some(effect_state),
    )?;
    active_move.effect_state = result
        .effect_state
        .wrap_error_with_format(format_args!("effect_state missing from output of {event}"))?;
    Ok(result.value)
}

fn run_active_move_event(
    context: &mut ActiveMoveContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> Option<fxlang::Value> {
    match run_active_move_event_with_errors(context, event, input) {
        Ok(value) => value,
        Err(error) => {
            let active_move_name = &context.active_move().data.name;
            // SAFETY: The context is only borrowed again for logging.
            let active_move_name = unsafe { active_move_name.unsafely_detach_borrow() };
            core_battle_logs::debug_event_failure(
                context.as_battle_context_mut(),
                event,
                active_move_name,
                &error.message(),
            );
            None
        }
    }
}

/// Runs an event on an active [`Move`][`crate::moves::Move`], expecting no input and output.
pub fn run_active_move_event_expecting_void(
    context: &mut ActiveMoveContext,
    event: fxlang::BattleEvent,
) {
    run_active_move_event(context, event, fxlang::VariableInput::new());
}
