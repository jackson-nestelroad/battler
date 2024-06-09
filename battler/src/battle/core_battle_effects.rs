use std::mem;

use crate::{
    battle::{
        core_battle_actions,
        core_battle_logs,
        speed_sort,
        ActiveMoveContext,
        ApplyingEffectContext,
        Context,
        CoreBattle,
        Mon,
        MonHandle,
        SpeedOrderable,
    },
    battler_error,
    common::{
        Error,
        Id,
        UnsafelyDetachBorrow,
        UnsafelyDetachBorrowMut,
        WrapResultError,
    },
    effect::{
        fxlang::{
            self,
            EffectState,
            ProgramEvalResult,
        },
        Effect,
        EffectHandle,
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
    let result = EffectManager::evaluate_under_active_move(
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

fn run_effect_event_under_applying_effect_with_errors(
    context: &mut ApplyingEffectContext,
    effect: &Effect,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
    effect_state: Option<fxlang::EffectState>,
) -> Result<ProgramEvalResult, Error> {
    EffectManager::evaluate_under_applying_effect(context, effect, event, input, effect_state)
}

fn run_effect_event_by_handle_under_applying_effect(
    context: &mut ApplyingEffectContext,
    effect: &EffectHandle,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
    effect_state: Option<fxlang::EffectState>,
) -> ProgramEvalResult {
    let effect = match CoreBattle::get_effect_by_handle(context.as_battle_context(), effect) {
        Ok(effect) => effect,
        Err(_) => return ProgramEvalResult::default(),
    };
    // SAFETY: Effect is guaranteed to exist beyond this turn. We do not advance the turn in
    // any effect.
    let effect: Effect = unsafe { mem::transmute(effect) };
    match run_effect_event_under_applying_effect_with_errors(
        context,
        &effect,
        event,
        input,
        effect_state,
    ) {
        Ok(result) => result,
        Err(error) => {
            core_battle_logs::debug_event_failure(
                context.as_battle_context_mut(),
                event,
                effect.name(),
                &error.message(),
            );
            ProgramEvalResult::default()
        }
    }
}

fn run_effect_event_by_id_under_applying_effect(
    context: &mut ApplyingEffectContext,
    effect: &Id,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
    effect_state: Option<fxlang::EffectState>,
) -> ProgramEvalResult {
    let effect = match CoreBattle::get_effect_by_id(context.as_battle_context_mut(), effect) {
        Ok(effect) => effect,
        Err(_) => return ProgramEvalResult::default(),
    };
    // SAFETY: Effect is guaranteed to exist beyond this turn. We do not advance the turn in
    // any effect.
    let effect: Effect = unsafe { mem::transmute(effect) };
    match run_effect_event_under_applying_effect_with_errors(
        context,
        &effect,
        event,
        input,
        effect_state,
    ) {
        Ok(result) => result,
        Err(error) => {
            let effect_name = context.effect().name().to_owned();
            core_battle_logs::debug_event_failure(
                context.as_battle_context_mut(),
                event,
                effect.name(),
                &error.message(),
            );
            ProgramEvalResult::default()
        }
    }
}

enum EffectOrigin {
    MonStatus(MonHandle),
}

struct CallbackHandle {
    pub effect_handle: EffectHandle,
    pub event: fxlang::BattleEvent,
    pub origin: EffectOrigin,
}

impl CallbackHandle {
    pub fn new(
        effect_handle: EffectHandle,
        event: fxlang::BattleEvent,
        origin: EffectOrigin,
    ) -> Self {
        Self {
            effect_handle,
            event,
            origin,
        }
    }

    pub fn effect_state_mut<'context, 'battle, 'data>(
        &self,
        context: &'context mut Context<'battle, 'data>,
    ) -> Result<Option<&'context mut EffectState>, Error> {
        match self.origin {
            EffectOrigin::MonStatus(mon) => Ok(Some(&mut context.mon_mut(mon)?.status_state)),
        }
    }
}

fn run_callback_against_target_with_errors(
    context: &mut ApplyingEffectContext,
    input: fxlang::VariableInput,
    callback_handle: CallbackHandle,
) -> Result<Option<fxlang::Value>, Error> {
    let effect_state = callback_handle
        .effect_state_mut(context.as_battle_context_mut())?
        .cloned();

    // Run the event callback for the event.
    let result = run_effect_event_by_handle_under_applying_effect(
        context,
        &callback_handle.effect_handle,
        callback_handle.event,
        input.clone(),
        effect_state,
    );

    // Save the new effect state if applicable.
    if let Some(effect_state) = callback_handle.effect_state_mut(context.as_battle_context_mut())? {
        if let Some(new_effect_state) = result.effect_state {
            *effect_state = new_effect_state;
        }
    }

    Ok(result.value)
}

fn run_callback_against_target(
    context: &mut ApplyingEffectContext,
    input: fxlang::VariableInput,
    callback_handle: CallbackHandle,
) -> Option<fxlang::Value> {
    run_callback_against_target_with_errors(context, input, callback_handle)
        .ok()
        .flatten()
}

fn run_mon_status_event(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> Option<fxlang::Value> {
    let status = context.target().status.clone()?;
    let effect_handle = context
        .battle_mut()
        .get_effect_handle_by_id(&status)
        .ok()?
        .clone();
    run_callback_against_target(
        context,
        input,
        CallbackHandle::new(
            effect_handle,
            event,
            EffectOrigin::MonStatus(context.target_handle()),
        ),
    )
}

fn find_callbacks_on_mon(
    context: &mut Context,
    event: fxlang::BattleEvent,
    mon: MonHandle,
) -> Result<Vec<CallbackHandle>, Error> {
    let mut callbacks = Vec::new();
    let mut context = context.mon_context(mon)?;

    if let Some(status) = context.mon().status.clone() {
        let status_effect_handle = context.battle_mut().get_effect_handle_by_id(&status)?;
        callbacks.push(CallbackHandle::new(
            status_effect_handle.clone(),
            event,
            EffectOrigin::MonStatus(mon),
        ));
    }

    // TODO: Volatile statuses.
    // TODO: Ability.
    // TODO: Item.
    // TODO: Species.
    // TODO: Slot conditions on the side.

    Ok(callbacks)
}

fn find_callbacks_on_side(
    context: &mut Context,
    event: fxlang::BattleEvent,
    side: usize,
) -> Result<Vec<CallbackHandle>, Error> {
    let mut callbacks = Vec::new();
    let mut context = context.side_context(side)?;

    // TODO: Side conditions.

    Ok(callbacks)
}

fn find_callbacks_on_field(
    context: &mut Context,
    event: fxlang::BattleEvent,
) -> Result<Vec<CallbackHandle>, Error> {
    let mut callbacks = Vec::new();

    // TODO: Pseudo-weather.
    // TODO: Weather.
    // TODO: Terrain.

    Ok(callbacks)
}

#[derive(Clone, Copy)]
enum AllEffectsTarget {
    Mon(MonHandle),
    Side(usize),
    Residual,
}

fn find_all_callbacks(
    context: &mut Context,
    event: fxlang::BattleEvent,
    target: AllEffectsTarget,
    source: Option<MonHandle>,
) -> Result<Vec<CallbackHandle>, Error> {
    let mut callbacks = Vec::new();

    match target {
        AllEffectsTarget::Mon(mon) => {
            callbacks.extend(find_callbacks_on_mon(context, event, mon)?);
            let mut context = context.mon_context(mon)?;
            for mon in Mon::active_allies_and_self(&mut context).collect::<Vec<_>>() {
                if let Some(ally_event) = event.ally_event() {
                    callbacks.extend(find_callbacks_on_mon(
                        context.as_battle_context_mut(),
                        ally_event,
                        mon,
                    )?);
                }
                if let Some(any_event) = event.any_event() {
                    callbacks.extend(find_callbacks_on_mon(
                        context.as_battle_context_mut(),
                        any_event,
                        mon,
                    )?);
                }
            }
            for mon in Mon::active_foes(&mut context).collect::<Vec<_>>() {
                if let Some(foe_event) = event.foe_event() {
                    callbacks.extend(find_callbacks_on_mon(
                        context.as_battle_context_mut(),
                        foe_event,
                        mon,
                    )?);
                }
                if let Some(any_event) = event.any_event() {
                    callbacks.extend(find_callbacks_on_mon(
                        context.as_battle_context_mut(),
                        any_event,
                        mon,
                    )?);
                }
            }
        }
        AllEffectsTarget::Side(side) => {
            callbacks.extend(find_callbacks_on_side(context, event, side)?);
        }
        AllEffectsTarget::Residual => {
            for mon in context
                .battle()
                .all_active_mon_handles()
                .collect::<Vec<_>>()
            {
                callbacks.extend(find_callbacks_on_mon(context, event, mon)?);
            }
            for side in context.battle().side_indices() {
                callbacks.extend(find_callbacks_on_side(context, event, side)?);
            }
        }
    }

    if let Some(source) = source {
        if let Some(source_event) = event.source_event() {
            callbacks.extend(find_callbacks_on_mon(context, source_event, source)?);
        }
    }

    callbacks.extend(find_callbacks_on_field(context, event)?);

    Ok(callbacks)
}

struct SpeedOrderableCallbackHandle {
    pub callback_handle: CallbackHandle,
    pub order: u32,
    pub priority: i32,
    pub sub_order: u32,
}

impl SpeedOrderableCallbackHandle {
    pub fn new(callback_handle: CallbackHandle) -> Self {
        Self {
            callback_handle,
            order: u32::MAX,
            priority: 0,
            sub_order: 0,
        }
    }
}

impl SpeedOrderable for SpeedOrderableCallbackHandle {
    fn order(&self) -> u32 {
        self.order
    }

    fn priority(&self) -> i32 {
        self.priority
    }

    fn speed(&self) -> u32 {
        0
    }

    fn sub_order(&self) -> u32 {
        self.sub_order
    }
}

fn get_speed_orderable_effect_handle(
    context: &mut Context,
    callback_handle: CallbackHandle,
) -> Result<Option<SpeedOrderableCallbackHandle>, Error> {
    let effect = CoreBattle::get_effect_by_handle(context, &callback_handle.effect_handle)?;
    let callback = match effect.fxlang_callbacks() {
        Some(callbacks) => match callbacks.event(callback_handle.event) {
            Some(callback) => callback,
            None => return Ok(None),
        },
        None => return Ok(None),
    };
    let mut result = SpeedOrderableCallbackHandle::new(callback_handle);
    result.order = callback.order();
    result.priority = callback.priority();
    result.sub_order = callback.sub_order();
    Ok(Some(result))
}

fn get_ordered_effects_for_event(
    context: &mut Context,
    callback_handles: Vec<CallbackHandle>,
) -> Result<Vec<CallbackHandle>, Error> {
    let mut speed_orderable_handles = Vec::new();
    speed_orderable_handles.reserve(callback_handles.len());
    for effect_handle in callback_handles {
        match get_speed_orderable_effect_handle(context, effect_handle)? {
            Some(handle) => speed_orderable_handles.push(handle),
            None => (),
        }
    }

    speed_sort(
        speed_orderable_handles.as_mut_slice(),
        context.battle_mut().prng.as_mut(),
    );
    Ok(speed_orderable_handles
        .into_iter()
        .map(|handle| handle.callback_handle)
        .collect())
}

fn run_callbacks_against_target_with_errors(
    context: &mut ApplyingEffectContext,
    mut input: fxlang::VariableInput,
    callbacks: Vec<CallbackHandle>,
) -> Result<Option<fxlang::Value>, Error> {
    for callback_handle in callbacks {
        // If a value was returned, use it to determine what we do next.
        match run_callback_against_target_with_errors(context, input.clone(), callback_handle)? {
            // Early exit.
            value @ Some(fxlang::Value::Boolean(false)) => return Ok(value),
            // Pass the output to the next effect.
            Some(value) => input = Vec::from([value]),
            // Do nothing (the input will be passed to the next callback).
            _ => (),
        }
    }

    // The first input variable is always returned as the result.
    Ok(input.get(0).cloned())
}

fn run_residual_callbacks_with_errors(
    context: &mut Context,
    callbacks: Vec<CallbackHandle>,
) -> Result<(), Error> {
    for callback_handle in callbacks {
        if context.battle().ended() {
            break;
        }

        let mut context = context.effect_context(&callback_handle.effect_handle)?;

        let mut ended = false;
        if let Some(effect_state) =
            callback_handle.effect_state_mut(context.as_battle_context_mut())?
        {
            if let Some(duration) = effect_state.duration() {
                let duration = duration - 1;
                effect_state.set_duration(duration);
                if duration == 0 {
                    ended = true;
                }
            }
        }

        match callback_handle.origin {
            EffectOrigin::MonStatus(mon) => {
                let mut context = context.applying_effect_context(None, mon)?;
                if ended {
                    core_battle_actions::clear_status(&mut context, false)?;
                } else {
                    run_callback_against_target_with_errors(
                        &mut context,
                        fxlang::VariableInput::new(),
                        callback_handle,
                    )?;
                }
            }
        }
    }
    Ok(())
}

fn run_event_with_errors(
    context: &mut Context,
    event: fxlang::BattleEvent,
    effect: Option<EffectHandle>,
    target: AllEffectsTarget,
    source: Option<MonHandle>,
    input: fxlang::VariableInput,
) -> Result<Option<fxlang::Value>, Error> {
    let callbacks = find_all_callbacks(context, event, target, source)?;
    let callbacks = get_ordered_effects_for_event(context, callbacks)?;

    match target {
        AllEffectsTarget::Mon(mon) => {
            let effect = match effect {
                Some(effect) => effect,
                None => {
                    return Err(battler_error!(
                        "event against a target mon must have an applying effect"
                    ))
                }
            };
            let mut context = context.effect_context(&effect)?;
            let mut context = context.applying_effect_context(source, mon)?;
            run_callbacks_against_target_with_errors(&mut context, input, callbacks)
        }
        AllEffectsTarget::Side(_) => todo!("running effects against a side is not implemented"),
        AllEffectsTarget::Residual => {
            run_residual_callbacks_with_errors(context, callbacks).map(|()| None)
        }
    }
}

fn run_event_for_applying_effect_internal(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> Option<fxlang::Value> {
    let target = AllEffectsTarget::Mon(context.target_handle());
    let effect = context.effect_handle();
    let source = context.source_handle();
    match run_event_with_errors(
        context.as_battle_context_mut(),
        event,
        Some(effect),
        target,
        source,
        input,
    ) {
        Ok(value) => value,
        Err(error) => {
            core_battle_logs::debug_full_event_failure(
                context.as_battle_context_mut(),
                event,
                &error.message(),
            );
            None
        }
    }
}

fn run_event_for_no_target_internal(context: &mut Context, event: fxlang::BattleEvent) {
    match run_event_with_errors(
        context,
        event,
        None,
        AllEffectsTarget::Residual,
        None,
        fxlang::VariableInput::new(),
    ) {
        Ok(_) => (),
        Err(error) => {
            core_battle_logs::debug_full_event_failure(context, event, &error.message());
        }
    }
}

/// Runs an event on an active [`Move`][`crate::moves::Move`].
///
/// Expects no input or output. Any output is ignored.
pub fn run_active_move_event_expecting_void(
    context: &mut ActiveMoveContext,
    event: fxlang::BattleEvent,
) {
    run_active_move_event(context, event, fxlang::VariableInput::new());
}

/// Runs an event on the target [`Mon`]'s current status.
///
/// Expects an integer that can fit ina [`u8`].
pub fn run_mon_status_event_expecting_u8(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
) -> Option<u8> {
    run_mon_status_event(context, event, fxlang::VariableInput::new())?
        .integer_u8()
        .ok()
}

/// Runs an event on the target [`Mon`]'s current status.
///
/// Expects a [`bool`].
pub fn run_mon_status_event_expecting_bool(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
) -> Option<bool> {
    run_mon_status_event(context, event, fxlang::VariableInput::new())?
        .boolean()
        .ok()
}

/// Runs an event on the [`Battle`][`crate::battle::Battle`] for an applying effect.
///
/// Returns `true` if all event handlers succeeded (i.e., did not return `false`).
pub fn run_event_for_applying_effect(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> bool {
    run_event_for_applying_effect_internal(context, event, input)
        .map(|value| value.boolean().ok())
        .flatten()
        .unwrap_or(true)
}

/// Runs an event on the [`Battle`][`crate::battle::Battle`] for the residual effect, which occurs
/// at the end of every turn.
///
/// Expects no input or output.
pub fn run_event_for_no_target(context: &mut Context, event: fxlang::BattleEvent) {
    run_event_for_no_target_internal(context, event)
}
