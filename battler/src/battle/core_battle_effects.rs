use std::iter;

use ahash::HashSet;
use anyhow::Result;
use battler_data::{
    BoostTable,
    Fraction,
    Id,
    MoveTarget,
    SecondaryEffectData,
    StatTable,
    Type,
};

use crate::{
    battle::{
        ActiveMoveContext,
        ApplyingEffectContext,
        Context,
        CoreBattle,
        EffectContext,
        Field,
        FieldEffectContext,
        Mon,
        MonContext,
        MonHandle,
        MoveEventResult,
        MoveOutcomeOnTarget,
        PlayerContext,
        PlayerEffectContext,
        SideContext,
        SideEffectContext,
        SpeedOrderable,
        core_battle_logs,
        mon_states,
    },
    common::{
        MaybeOwnedMut,
        UnsafelyDetachBorrow,
    },
    effect::{
        ActiveMoveEffectStateConnector,
        AppliedEffectHandle,
        AppliedEffectLocation,
        EffectHandle,
        EffectManager,
        fxlang::{
            self,
            EffectStateConnector,
        },
    },
    error::WrapOptionError,
};

enum UpcomingEvaluationContext<
    'field_effect,
    'side_effect,
    'player_effect,
    'applying_effect,
    'effect,
    'mon,
    'player,
    'side,
    'context,
    'battle,
    'data,
> where
    'data: 'battle,
    'battle: 'context,
    'context: 'side,
    'side: 'player,
    'player: 'mon,
    'context: 'effect,
    'effect: 'applying_effect,
    'effect: 'player_effect,
    'effect: 'side_effect,
    'effect: 'field_effect,
{
    ApplyingEffect(
        MaybeOwnedMut<'applying_effect, ApplyingEffectContext<'effect, 'context, 'battle, 'data>>,
    ),
    Effect(MaybeOwnedMut<'effect, EffectContext<'context, 'battle, 'data>>),
    Mon(MaybeOwnedMut<'mon, MonContext<'player, 'side, 'context, 'battle, 'data>>),
    PlayerEffect(
        MaybeOwnedMut<'player_effect, PlayerEffectContext<'effect, 'context, 'battle, 'data>>,
    ),
    Player(MaybeOwnedMut<'player, PlayerContext<'side, 'context, 'battle, 'data>>),
    SideEffect(MaybeOwnedMut<'side_effect, SideEffectContext<'effect, 'context, 'battle, 'data>>),
    Side(MaybeOwnedMut<'side, SideContext<'context, 'battle, 'data>>),
    FieldEffect(
        MaybeOwnedMut<'field_effect, FieldEffectContext<'effect, 'context, 'battle, 'data>>,
    ),
    Field(MaybeOwnedMut<'context, Context<'battle, 'data>>),
}

impl<
    'field_effect,
    'side_effect,
    'player_effect,
    'applying_effect,
    'effect,
    'mon,
    'player,
    'side,
    'context,
    'battle,
    'data,
>
    UpcomingEvaluationContext<
        'field_effect,
        'side_effect,
        'player_effect,
        'applying_effect,
        'effect,
        'mon,
        'player,
        'side,
        'context,
        'battle,
        'data,
    >
{
    fn battle_context(&self) -> &Context<'battle, 'data> {
        match self {
            Self::ApplyingEffect(context) => context.as_battle_context(),
            Self::Effect(context) => context.as_battle_context(),
            Self::Mon(context) => context.as_battle_context(),
            Self::Player(context) => context.as_battle_context(),
            Self::PlayerEffect(context) => context.as_battle_context(),
            Self::SideEffect(context) => context.as_battle_context(),
            Self::Side(context) => context.as_battle_context(),
            Self::FieldEffect(context) => context.as_battle_context(),
            Self::Field(context) => context,
        }
    }

    fn battle_context_mut(&mut self) -> &mut Context<'battle, 'data> {
        match self {
            Self::ApplyingEffect(context) => context.as_battle_context_mut(),
            Self::Effect(context) => context.as_battle_context_mut(),
            Self::Mon(context) => context.as_battle_context_mut(),
            Self::Player(context) => context.as_battle_context_mut(),
            Self::PlayerEffect(context) => context.as_battle_context_mut(),
            Self::SideEffect(context) => context.as_battle_context_mut(),
            Self::Side(context) => context.as_battle_context_mut(),
            Self::FieldEffect(context) => context.as_battle_context_mut(),
            Self::Field(context) => context,
        }
    }
}

fn run_effect_event_with_errors(
    context: &mut UpcomingEvaluationContext,
    effect_handle: &EffectHandle,
    event: fxlang::BattleEvent,
    modifier: fxlang::BattleEventModifier,
    input: fxlang::VariableInput,
    event_state: &fxlang::EventState,
    effect_state_connector: Option<fxlang::DynamicEffectStateConnector>,
    suppressed: bool,
) -> Result<fxlang::ProgramEvalResult> {
    // Effect was suppressed somewhere up the stack, so we should skip the callback.
    //
    // This is important for residual callbacks, which can be suppressed but should still attempt to
    // run in order to properly decrement duration counters.
    if suppressed {
        return Ok(fxlang::ProgramEvalResult::default());
    }

    // Effect state no longer exists, so we should skip the callback.
    if let Some(effect_state_connector) = &effect_state_connector {
        if !effect_state_connector.exists(context.battle_context_mut())? {
            return Ok(fxlang::ProgramEvalResult::default());
        }

        // If we are ending, set the ending flag, so that nested events don't use this callback.
        if event.ends_effect() {
            effect_state_connector
                .get_mut(context.battle_context_mut())?
                .set_ending(true);
        }
    }

    let mut context = match context {
        UpcomingEvaluationContext::ApplyingEffect(context) => {
            fxlang::EvaluationContext::ApplyingEffect(
                context.forward_applying_effect_context(effect_handle.clone())?,
            )
        }
        UpcomingEvaluationContext::Effect(context) => fxlang::EvaluationContext::Effect(
            context.forward_effect_context(effect_handle.clone())?,
        ),
        UpcomingEvaluationContext::Mon(context) => fxlang::EvaluationContext::ApplyingEffect(
            context.applying_effect_context(effect_handle.clone(), None, None)?,
        ),
        UpcomingEvaluationContext::PlayerEffect(context) => {
            fxlang::EvaluationContext::PlayerEffect(
                context.forward_player_effect_context(effect_handle.clone())?,
            )
        }
        UpcomingEvaluationContext::Player(context) => fxlang::EvaluationContext::PlayerEffect(
            context.player_effect_context(effect_handle.clone(), None, None)?,
        ),
        UpcomingEvaluationContext::SideEffect(context) => fxlang::EvaluationContext::SideEffect(
            context.forward_side_effect_context(effect_handle.clone())?,
        ),
        UpcomingEvaluationContext::Side(context) => fxlang::EvaluationContext::SideEffect(
            context.side_effect_context(effect_handle.clone(), None, None)?,
        ),
        UpcomingEvaluationContext::FieldEffect(context) => fxlang::EvaluationContext::FieldEffect(
            context.forward_field_effect_context(effect_handle.clone())?,
        ),
        UpcomingEvaluationContext::Field(context) => fxlang::EvaluationContext::FieldEffect(
            context.field_effect_context(effect_handle.clone(), None, None)?,
        ),
    };
    let result = EffectManager::evaluate(
        &mut context,
        effect_handle,
        event,
        modifier,
        input,
        event_state,
        effect_state_connector.clone(),
    );

    if let Some(effect_state_connector) = &effect_state_connector {
        if event.starts_effect() {
            effect_state_connector
                .get_mut(context.battle_context_mut())?
                .set_started(true);
            effect_state_connector
                .get_mut(context.battle_context_mut())?
                .set_ending(false);
        }

        if event.ends_effect() {
            effect_state_connector
                .get_mut(context.battle_context_mut())?
                .set_started(false);
        }
    }

    result
}

fn run_active_move_event_with_errors(
    context: &mut ActiveMoveContext,
    event: fxlang::BattleEvent,
    modifier: fxlang::BattleEventModifier,
    target: MoveTargetForEvent,
    input: fxlang::VariableInput,
) -> Result<Option<fxlang::Value>> {
    let effect_state_connector =
        ActiveMoveEffectStateConnector::new(context.active_move_handle()).make_dynamic();
    let effect_handle = context.effect_handle().clone();

    let result = match target {
        MoveTargetForEvent::Mon(mon) => run_effect_event_with_errors(
            &mut UpcomingEvaluationContext::ApplyingEffect(
                context.applying_effect_context_for_target(mon)?.into(),
            ),
            &effect_handle,
            event,
            modifier,
            input,
            &fxlang::EventState::default(),
            Some(effect_state_connector),
            false,
        )?,
        MoveTargetForEvent::Side(side) => run_effect_event_with_errors(
            &mut UpcomingEvaluationContext::SideEffect(context.side_effect_context(side)?.into()),
            &effect_handle,
            event,
            modifier,
            input,
            &fxlang::EventState::default(),
            Some(effect_state_connector),
            false,
        )?,
        MoveTargetForEvent::Field => run_effect_event_with_errors(
            &mut UpcomingEvaluationContext::FieldEffect(context.field_effect_context()?.into()),
            &effect_handle,
            event,
            modifier,
            input,
            &fxlang::EventState::default(),
            Some(effect_state_connector),
            false,
        )?,
        MoveTargetForEvent::User => run_effect_event_with_errors(
            &mut UpcomingEvaluationContext::ApplyingEffect(
                context.user_applying_effect_context(None)?.into(),
            ),
            &effect_handle,
            event,
            modifier,
            input,
            &fxlang::EventState::default(),
            Some(effect_state_connector),
            false,
        )?,
        MoveTargetForEvent::UserWithTarget(target) => run_effect_event_with_errors(
            &mut UpcomingEvaluationContext::ApplyingEffect(
                context.user_applying_effect_context(target)?.into(),
            ),
            &effect_handle,
            event,
            modifier,
            input,
            &fxlang::EventState::default(),
            Some(effect_state_connector),
            false,
        )?,
        MoveTargetForEvent::None => run_effect_event_with_errors(
            &mut UpcomingEvaluationContext::Effect(context.effect_context()?.into()),
            &effect_handle,
            event,
            modifier,
            input,
            &fxlang::EventState::default(),
            Some(effect_state_connector),
            false,
        )?,
    };

    Ok(result.value)
}

/// Runs an event on an active [`Move`][`crate::moves::Move`].
pub fn run_active_move_event(
    context: &mut ActiveMoveContext,
    event: fxlang::BattleEvent,
    target: MoveTargetForEvent,
    input: fxlang::VariableInput,
) -> Option<fxlang::Value> {
    match run_active_move_event_with_errors(
        context,
        event,
        fxlang::BattleEventModifier::None,
        target,
        input,
    ) {
        Ok(value) => value,
        Err(error) => {
            let active_move_name = &context.active_move().data.name;
            // SAFETY: The context is only borrowed again for logging.
            let active_move_name = unsafe { active_move_name.unsafely_detach_borrow() };
            core_battle_logs::debug_event_failure(
                context.as_battle_context_mut(),
                event,
                active_move_name,
                &&format!("{error:#}"),
            );
            None
        }
    }
}

fn run_effect_event_by_handle(
    context: &mut UpcomingEvaluationContext,
    effect: &EffectHandle,
    event: fxlang::BattleEvent,
    modifier: fxlang::BattleEventModifier,
    input: fxlang::VariableInput,
    event_state: &fxlang::EventState,
    effect_state_connector: Option<fxlang::DynamicEffectStateConnector>,
    suppressed: bool,
) -> fxlang::ProgramEvalResult {
    match run_effect_event_with_errors(
        context,
        &effect,
        event,
        modifier,
        input,
        event_state,
        effect_state_connector,
        suppressed,
    ) {
        Ok(result) => result,
        Err(error) => {
            let effect_name =
                match CoreBattle::get_effect_by_handle(context.battle_context(), effect) {
                    Ok(effect) => effect.name().to_owned(),
                    Err(_) => format!("{:?}", effect),
                };
            core_battle_logs::debug_event_failure(
                context.battle_context_mut(),
                event,
                &effect_name,
                &&format!("{error:#}"),
            );
            fxlang::ProgramEvalResult::default()
        }
    }
}

/// The target of a move for effect callbacks that run directly on an active move.
pub enum MoveTargetForEvent {
    /// The effect runs with no target.
    None,
    /// The effect runs with respect to the user of the move.
    ///
    /// This does not mean the target of the move is the user.
    User,
    /// The effect runs with respect to the user of the move, additionally with an optional target
    /// if one is available.
    UserWithTarget(Option<MonHandle>),
    /// The effect runs with respect to a single target of the move.
    Mon(MonHandle),
    /// The effect runs with respect to the target side of the move.
    Side(usize),
    /// The effect runs with respect to the field as a whole.
    Field,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CallbackHandle {
    pub applied_effect_handle: AppliedEffectHandle,
    pub event: fxlang::BattleEvent,
    pub modifier: fxlang::BattleEventModifier,
    pub suppressed: bool,
}

impl CallbackHandle {
    pub fn new(
        effect_handle: EffectHandle,
        event: fxlang::BattleEvent,
        modifier: fxlang::BattleEventModifier,
        location: AppliedEffectLocation,
    ) -> Self {
        Self {
            applied_effect_handle: AppliedEffectHandle::new(effect_handle, location),
            event,
            modifier,
            suppressed: false,
        }
    }

    /// The speed of the callback.
    pub fn speed(&self, context: &mut Context) -> Result<u32> {
        if let Some(mon_handle) = self.applied_effect_handle.location.mon_handle() {
            return Ok(context.mon(mon_handle)?.speed as u32);
        }
        Ok(0)
    }
}

fn run_callback_with_errors(
    mut context: UpcomingEvaluationContext,
    input: fxlang::VariableInput,
    event_state: &fxlang::EventState,
    callback_handle: CallbackHandle,
) -> Result<Option<fxlang::Value>> {
    // Run the event callback for the event.
    let result = run_effect_event_by_handle(
        &mut context,
        &callback_handle.applied_effect_handle.effect_handle,
        callback_handle.event,
        callback_handle.modifier,
        input,
        event_state,
        callback_handle
            .applied_effect_handle
            .effect_state_connector(),
        callback_handle.suppressed,
    );

    Ok(result.value)
}

fn run_callback_under_applying_effect(
    context: &mut ApplyingEffectContext,
    input: fxlang::VariableInput,
    callback_handle: CallbackHandle,
) -> Option<fxlang::Value> {
    run_callback_with_errors(
        UpcomingEvaluationContext::ApplyingEffect(context.into()),
        input,
        &fxlang::EventState::default(),
        callback_handle,
    )
    .ok()
    .flatten()
}

fn run_callback_under_effect(
    context: &mut EffectContext,
    input: fxlang::VariableInput,
    callback_handle: CallbackHandle,
) -> Option<fxlang::Value> {
    run_callback_with_errors(
        UpcomingEvaluationContext::Effect(context.into()),
        input,
        &fxlang::EventState::default(),
        callback_handle,
    )
    .ok()
    .flatten()
}

fn run_callback_under_side_effect(
    context: &mut SideEffectContext,
    input: fxlang::VariableInput,
    callback_handle: CallbackHandle,
) -> Option<fxlang::Value> {
    run_callback_with_errors(
        UpcomingEvaluationContext::SideEffect(context.into()),
        input,
        &fxlang::EventState::default(),
        callback_handle,
    )
    .ok()
    .flatten()
}

fn run_callback_under_field_effect(
    context: &mut FieldEffectContext,
    input: fxlang::VariableInput,
    callback_handle: CallbackHandle,
) -> Option<fxlang::Value> {
    run_callback_with_errors(
        UpcomingEvaluationContext::FieldEffect(context.into()),
        input,
        &fxlang::EventState::default(),
        callback_handle,
    )
    .ok()
    .flatten()
}

fn run_callback_under_mon(
    context: &mut MonContext,
    input: fxlang::VariableInput,
    callback_handle: CallbackHandle,
) -> Option<fxlang::Value> {
    run_callback_with_errors(
        UpcomingEvaluationContext::Mon(context.into()),
        input,
        &fxlang::EventState::default(),
        callback_handle,
    )
    .ok()
    .flatten()
}

pub fn run_mon_status_event(
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
    let target_handle = context.target_handle();
    run_callback_under_applying_effect(
        context,
        input,
        CallbackHandle::new(
            effect_handle,
            event,
            fxlang::BattleEventModifier::default(),
            AppliedEffectLocation::MonStatus(target_handle),
        ),
    )
}

/// Runs an event on the target [`Mon`]'s volatile status.
pub fn run_mon_volatile_event(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
    status: &Id,
) -> Option<fxlang::Value> {
    let effect_handle = context
        .battle_mut()
        .get_effect_handle_by_id(status)
        .ok()?
        .clone();
    let target_handle = context.target_handle();
    run_callback_under_applying_effect(
        context,
        input,
        CallbackHandle::new(
            effect_handle,
            event,
            fxlang::BattleEventModifier::default(),
            AppliedEffectLocation::MonVolatile(target_handle),
        ),
    )
}

/// Runs an event on the target [`Mon`]'s current ability.
pub fn run_mon_ability_event(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> Option<fxlang::Value> {
    let ability = mon_states::effective_ability(&mut context.target_context().ok()?)?;
    let target_handle = context.target_handle();
    run_callback_under_applying_effect(
        context,
        input,
        CallbackHandle::new(
            EffectHandle::Ability(ability),
            event,
            fxlang::BattleEventModifier::default(),
            AppliedEffectLocation::MonAbility(target_handle),
        ),
    )
}

/// Runs an event on the target [`Mon`]'s current item.
pub fn run_mon_item_event(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> Option<fxlang::Value> {
    let item = mon_states::effective_item(&mut context.target_context().ok()?)?;
    let target_handle = context.target_handle();
    run_callback_under_applying_effect(
        context,
        input,
        CallbackHandle::new(
            EffectHandle::Item(item),
            event,
            fxlang::BattleEventModifier::default(),
            AppliedEffectLocation::MonItem(target_handle),
        ),
    )
}

pub fn run_mon_inactive_move_event(
    context: &mut MonContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
    mov: &Id,
) -> Option<fxlang::Value> {
    let mon_handle = context.mon_handle();
    run_callback_under_mon(
        context,
        input,
        CallbackHandle::new(
            EffectHandle::InactiveMove(mov.clone()),
            event,
            fxlang::BattleEventModifier::default(),
            AppliedEffectLocation::MonInactiveMove(mon_handle),
        ),
    )
}

/// Runs an event on the target [`Side`][`crate::battle::Side`]'s side condition.
pub fn run_side_condition_event(
    context: &mut SideEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
    condition: &Id,
) -> Option<fxlang::Value> {
    let effect_handle = context
        .battle_mut()
        .get_effect_handle_by_id(condition)
        .ok()?
        .clone();
    let side_index = context.side().index;
    run_callback_under_side_effect(
        context,
        input,
        CallbackHandle::new(
            effect_handle,
            event,
            fxlang::BattleEventModifier::default(),
            AppliedEffectLocation::SideCondition(side_index),
        ),
    )
}

/// Runs an event on the target [`Side`][`crate::battle::Side`]'s slot condition.
pub fn run_slot_condition_event(
    context: &mut SideEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
    slot: usize,
    condition: &Id,
) -> Option<fxlang::Value> {
    let effect_handle = context
        .battle_mut()
        .get_effect_handle_by_id(condition)
        .ok()?
        .clone();
    let side_index = context.side().index;
    run_callback_under_side_effect(
        context,
        input,
        CallbackHandle::new(
            effect_handle,
            event,
            fxlang::BattleEventModifier::default(),
            AppliedEffectLocation::SlotCondition(side_index, slot),
        ),
    )
}

/// Runs an event on the [`Field`][`crate::battle::Field`]'s current terrain.
pub fn run_terrain_event(
    context: &mut FieldEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> Option<fxlang::Value> {
    let weather = context.battle().field.terrain.clone()?;
    let effect_handle = context
        .battle_mut()
        .get_effect_handle_by_id(&weather)
        .ok()?
        .clone();
    run_callback_under_field_effect(
        context,
        input,
        CallbackHandle::new(
            effect_handle,
            event,
            fxlang::BattleEventModifier::default(),
            AppliedEffectLocation::Terrain,
        ),
    )
}

/// Runs an event on the [`Field`][`crate::battle::Field`]'s current weather.
pub fn run_weather_event(
    context: &mut FieldEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> Option<fxlang::Value> {
    let weather = context.battle().field.weather.clone()?;
    let effect_handle = context
        .battle_mut()
        .get_effect_handle_by_id(&weather)
        .ok()?
        .clone();
    run_callback_under_field_effect(
        context,
        input,
        CallbackHandle::new(
            effect_handle,
            event,
            fxlang::BattleEventModifier::default(),
            AppliedEffectLocation::Weather,
        ),
    )
}

/// Runs an event on one of the [`Field`][`crate::battle::Field`]'s pseudo-weather.
pub fn run_pseudo_weather_event(
    context: &mut FieldEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
    pseudo_weather: &Id,
) -> Option<fxlang::Value> {
    let effect_handle = context
        .battle_mut()
        .get_effect_handle_by_id(&pseudo_weather)
        .ok()?
        .clone();
    run_callback_under_field_effect(
        context,
        input,
        CallbackHandle::new(
            effect_handle,
            event,
            fxlang::BattleEventModifier::default(),
            AppliedEffectLocation::PseudoWeather,
        ),
    )
}

/// Runs an event on the applying [`Effect`][`crate::effect::Effect`].
pub fn run_applying_effect_event(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> Option<fxlang::Value> {
    let effect = context.effect_handle().clone();
    match context.source_applying_effect_context().ok()? {
        Some(mut context) => run_callback_under_applying_effect(
            &mut context,
            input,
            CallbackHandle::new(
                effect,
                event,
                fxlang::BattleEventModifier::default(),
                AppliedEffectLocation::None,
            ),
        ),
        None => run_callback_under_applying_effect(
            context,
            input,
            CallbackHandle::new(
                effect,
                event,
                fxlang::BattleEventModifier::default(),
                AppliedEffectLocation::None,
            ),
        ),
    }
}

/// Runs an event on the [`Effect`][`crate::effect::Effect`].
pub fn run_effect_event(
    context: &mut EffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> Option<fxlang::Value> {
    let effect = context.effect_handle().clone();
    match context.source_effect_context().ok()? {
        Some(mut context) => run_callback_under_effect(
            &mut context,
            input,
            CallbackHandle::new(
                effect,
                event,
                fxlang::BattleEventModifier::default(),
                AppliedEffectLocation::None,
            ),
        ),
        None => run_callback_under_effect(
            context,
            input,
            CallbackHandle::new(
                effect,
                event,
                fxlang::BattleEventModifier::default(),
                AppliedEffectLocation::None,
            ),
        ),
    }
}

fn find_callbacks_on_mon(
    context: &mut Context,
    event: fxlang::BattleEvent,
    modifier: fxlang::BattleEventModifier,
    mon: MonHandle,
) -> Result<Vec<CallbackHandle>> {
    let mut callbacks = Vec::new();
    let mut context = context.mon_context(mon)?;

    callbacks.push(CallbackHandle::new(
        EffectHandle::Condition(Id::from_known("mon")),
        event,
        modifier,
        AppliedEffectLocation::None,
    ));

    if event.callback_lookup_layer() > fxlang::BattleEvent::Types.callback_lookup_layer() {
        let types = mon_states::effective_types(&mut context);
        for typ in types {
            callbacks.push(CallbackHandle::new(
                EffectHandle::Condition(Id::from(format!("{typ}type"))),
                event,
                modifier,
                AppliedEffectLocation::MonType(mon),
            ));
        }
    }

    if let Some(status) = context.mon().status.clone() {
        let status_effect_handle = context.battle_mut().get_effect_handle_by_id(&status)?;
        callbacks.push(CallbackHandle::new(
            status_effect_handle.clone(),
            event,
            modifier,
            AppliedEffectLocation::MonStatus(mon),
        ));
    }
    for volatile in context.mon().volatiles.clone().keys() {
        let volatile_status_handle = context.battle_mut().get_effect_handle_by_id(&volatile)?;
        callbacks.push(CallbackHandle::new(
            volatile_status_handle.clone(),
            event,
            modifier,
            AppliedEffectLocation::MonVolatile(mon),
        ));
    }

    if event.callback_lookup_layer()
        > fxlang::BattleEvent::SuppressMonAbility.callback_lookup_layer()
    {
        let ability = context.mon().ability.id.clone();
        let effective_ability = mon_states::effective_ability(&mut context);
        let suppressed = effective_ability.is_none();
        if event.force_default_callback() || !suppressed {
            let ability = effective_ability.unwrap_or(ability);
            let mut callback_handle = CallbackHandle::new(
                EffectHandle::Ability(ability),
                event,
                modifier,
                AppliedEffectLocation::MonAbility(mon),
            );
            callback_handle.suppressed = suppressed;
            callbacks.push(callback_handle);
        }
    }

    if event.callback_lookup_layer() > fxlang::BattleEvent::SuppressMonItem.callback_lookup_layer()
        && let Some(item) = context.mon().item.as_ref().map(|item| item.id.clone())
    {
        let effective_item = mon_states::effective_item(&mut context);
        let suppressed = effective_item.is_none();
        if event.force_default_callback() || !suppressed {
            let item = effective_item.unwrap_or(item);
            let mut callback_handle = CallbackHandle::new(
                EffectHandle::Item(item),
                event,
                modifier,
                AppliedEffectLocation::MonItem(mon),
            );
            callback_handle.suppressed = suppressed;
            callbacks.push(callback_handle);
        }
    }

    callbacks.push(CallbackHandle::new(
        EffectHandle::Species(context.mon().species.clone()),
        event,
        modifier,
        AppliedEffectLocation::Mon(context.mon_handle()),
    ));

    callbacks.push(CallbackHandle::new(
        EffectHandle::ItemCondition(context.mon().ball.clone()),
        event,
        modifier,
        AppliedEffectLocation::Mon(context.mon_handle()),
    ));

    if context.mon().different_original_trainer
        && context.mon().level > context.battle().format.options.obedience_cap
    {
        callbacks.push(CallbackHandle::new(
            EffectHandle::Condition(Id::from_known("disobedience")),
            event,
            modifier,
            AppliedEffectLocation::Mon(context.mon_handle()),
        ));
    }

    if context.player().player_options.has_affection {
        callbacks.push(CallbackHandle::new(
            EffectHandle::Condition(Id::from_known("affection")),
            event,
            modifier,
            AppliedEffectLocation::Mon(context.mon_handle()),
        ));
    }

    Ok(callbacks)
}

fn find_callbacks_on_side(
    context: &mut Context,
    event: fxlang::BattleEvent,
    modifier: fxlang::BattleEventModifier,
    side: usize,
) -> Result<Vec<CallbackHandle>> {
    let mut callbacks = Vec::new();
    let mut context = context.side_context(side)?;

    for side_condition in context.side().conditions.clone().keys() {
        let side_condition_handle = context
            .battle_mut()
            .get_effect_handle_by_id(&side_condition)?;
        callbacks.push(CallbackHandle::new(
            side_condition_handle.clone(),
            event,
            modifier,
            AppliedEffectLocation::SideCondition(side),
        ));
    }

    for (slot, slot_conditions) in context.side().slot_conditions.clone() {
        for slot_condition in slot_conditions.keys() {
            let slot_condition_handle = context
                .battle_mut()
                .get_effect_handle_by_id(&slot_condition)?;
            callbacks.push(CallbackHandle::new(
                slot_condition_handle.clone(),
                event,
                modifier,
                AppliedEffectLocation::SlotCondition(side, slot),
            ));
        }
    }

    Ok(callbacks)
}

fn find_callbacks_on_side_on_mon(
    context: &mut Context,
    event: fxlang::BattleEvent,
    modifier: fxlang::BattleEventModifier,
    mon: MonHandle,
) -> Result<Vec<CallbackHandle>> {
    let mut callbacks = Vec::new();
    let mut context = context.mon_context(mon)?;
    let side = context.mon().side;

    for side_condition in context.side().conditions.clone().keys() {
        let side_condition_handle = context
            .battle_mut()
            .get_effect_handle_by_id(&side_condition)?;
        callbacks.push(CallbackHandle::new(
            side_condition_handle.clone(),
            event,
            modifier,
            AppliedEffectLocation::MonSideCondition(side, mon),
        ));
    }

    if context.mon().active {
        let slot =
            Mon::position_on_side(&context).wrap_expectation("expected target to be active")?;
        if let Some(slot_conditions) = context.side().slot_conditions.get(&slot).cloned() {
            for slot_condition in slot_conditions.keys() {
                let slot_condition_handle = context
                    .battle_mut()
                    .get_effect_handle_by_id(&slot_condition)?;
                callbacks.push(CallbackHandle::new(
                    slot_condition_handle.clone(),
                    event,
                    modifier,
                    AppliedEffectLocation::MonSlotCondition(side, slot, mon),
                ));
            }
        }
    }

    Ok(callbacks)
}

fn find_callbacks_on_field(
    context: &mut Context,
    event: fxlang::BattleEvent,
    modifier: fxlang::BattleEventModifier,
) -> Result<Vec<CallbackHandle>> {
    let mut callbacks = Vec::new();

    if event.callback_lookup_layer()
        > fxlang::BattleEvent::SuppressFieldWeather.callback_lookup_layer()
    {
        if let Some(weather) = context.battle().field.weather.clone() {
            let effective_weather = Field::effective_weather(context);
            let suppressed = effective_weather.is_none();
            if event.force_default_callback() || !suppressed {
                let weather_handle = context
                    .battle_mut()
                    .get_effect_handle_by_id(&effective_weather.unwrap_or(weather))?;
                let mut callback_handle = CallbackHandle::new(
                    weather_handle.clone(),
                    event,
                    modifier,
                    AppliedEffectLocation::Weather,
                );
                callback_handle.suppressed = suppressed;
                callbacks.push(callback_handle);
            }
        }
    }

    if event.callback_lookup_layer()
        > fxlang::BattleEvent::SuppressFieldTerrain.callback_lookup_layer()
    {
        if let Some(terrain) = context.battle().field.terrain.clone() {
            let effective_terrain = Field::effective_terrain(context);
            let suppressed = effective_terrain.is_none();
            if event.force_default_callback() || !suppressed {
                let terrain_handle = context
                    .battle_mut()
                    .get_effect_handle_by_id(&effective_terrain.unwrap_or(terrain))?;
                let mut callback_handle = CallbackHandle::new(
                    terrain_handle.clone(),
                    event,
                    modifier,
                    AppliedEffectLocation::Terrain,
                );
                callback_handle.suppressed = suppressed;
                callbacks.push(callback_handle);
            }
        }
    }

    for pseudo_weather in context.battle().field.pseudo_weathers.clone().keys() {
        let pseudo_weather_handle = context
            .battle_mut()
            .get_effect_handle_by_id(&pseudo_weather)?;
        callbacks.push(CallbackHandle::new(
            pseudo_weather_handle.clone(),
            event,
            modifier,
            AppliedEffectLocation::PseudoWeather,
        ));
    }

    for rule in context.battle().format.rules.rules() {
        callbacks.push(CallbackHandle::new(
            EffectHandle::Clause(rule.clone()),
            event,
            modifier,
            AppliedEffectLocation::None,
        ));
    }

    Ok(callbacks)
}

fn find_callbacks_on_field_on_mon(
    context: &mut Context,
    event: fxlang::BattleEvent,
    modifier: fxlang::BattleEventModifier,
    mon: MonHandle,
) -> Result<Vec<CallbackHandle>> {
    let mut callbacks = Vec::new();
    let mut context = context.mon_context(mon)?;

    if event.callback_lookup_layer()
        > fxlang::BattleEvent::SuppressMonTerrain.callback_lookup_layer()
    {
        if let Some(terrain) = context.battle().field.terrain.clone() {
            let effective_terrain = mon_states::effective_terrain(&mut context);
            let suppressed = effective_terrain.is_none();
            if event.force_default_callback() || !suppressed {
                let terrain_handle = context
                    .battle_mut()
                    .get_effect_handle_by_id(&effective_terrain.unwrap_or(terrain))?;
                let mut callback_handle = CallbackHandle::new(
                    terrain_handle.clone(),
                    event,
                    modifier,
                    AppliedEffectLocation::MonTerrain(mon),
                );
                callback_handle.suppressed = suppressed;
                callbacks.push(callback_handle);
            }
        }
    }

    if event.callback_lookup_layer()
        > fxlang::BattleEvent::SuppressMonWeather.callback_lookup_layer()
    {
        if let Some(weather) = context.battle().field.weather.clone() {
            let effective_weather = mon_states::effective_weather(&mut context);
            let suppressed = effective_weather.is_none();
            if event.force_default_callback() || !suppressed {
                let weather_handle = context
                    .battle_mut()
                    .get_effect_handle_by_id(&effective_weather.unwrap_or(weather))?;
                let mut callback_handle = CallbackHandle::new(
                    weather_handle.clone(),
                    event,
                    modifier,
                    AppliedEffectLocation::MonWeather(mon),
                );
                callback_handle.suppressed = suppressed;
                callbacks.push(callback_handle);
            }
        }
    }

    for pseudo_weather in context.battle().field.pseudo_weathers.clone().keys() {
        let pseudo_weather_handle = context
            .battle_mut()
            .get_effect_handle_by_id(&pseudo_weather)?;
        callbacks.push(CallbackHandle::new(
            pseudo_weather_handle.clone(),
            event,
            modifier,
            AppliedEffectLocation::MonPseudoWeather(mon),
        ));
    }

    for rule in context.battle().format.rules.rules() {
        callbacks.push(CallbackHandle::new(
            EffectHandle::Clause(rule.clone()),
            event,
            modifier,
            AppliedEffectLocation::None,
        ));
    }

    Ok(callbacks)
}

#[derive(Clone, Copy)]
enum AllEffectsTarget {
    Mon(MonHandle),
    Player(usize),
    Side(usize),
    Field,
    Residual,
}

fn find_all_callbacks(
    context: &mut Context,
    event: fxlang::BattleEvent,
    target: AllEffectsTarget,
    source: Option<MonHandle>,
) -> Result<Vec<CallbackHandle>> {
    let mut callbacks = Vec::new();

    match target {
        AllEffectsTarget::Mon(mon) => {
            callbacks.extend(find_callbacks_on_mon(
                context,
                event,
                fxlang::BattleEventModifier::None,
                mon,
            )?);
            let mut context = context.mon_context(mon)?;
            for mon in Mon::active_allies_and_self(&mut context).collect::<Vec<_>>() {
                callbacks.extend(find_callbacks_on_mon(
                    context.as_battle_context_mut(),
                    event,
                    fxlang::BattleEventModifier::Ally,
                    mon,
                )?);
                callbacks.extend(find_callbacks_on_mon(
                    context.as_battle_context_mut(),
                    event,
                    fxlang::BattleEventModifier::Any,
                    mon,
                )?);
            }
            for mon in Mon::active_foes(&mut context).collect::<Vec<_>>() {
                callbacks.extend(find_callbacks_on_mon(
                    context.as_battle_context_mut(),
                    event,
                    fxlang::BattleEventModifier::Foe,
                    mon,
                )?);
                callbacks.extend(find_callbacks_on_mon(
                    context.as_battle_context_mut(),
                    event,
                    fxlang::BattleEventModifier::Any,
                    mon,
                )?);
            }
            callbacks.extend(find_callbacks_on_side_on_mon(
                context.as_battle_context_mut(),
                event,
                fxlang::BattleEventModifier::None,
                mon,
            )?);
            let side = context.side().index;
            callbacks.extend(find_callbacks_on_side(
                context.as_battle_context_mut(),
                event,
                fxlang::BattleEventModifier::Side,
                side,
            )?);
            let foe_side = context.foe_side().index;
            callbacks.extend(find_callbacks_on_side(
                context.as_battle_context_mut(),
                event,
                fxlang::BattleEventModifier::Foe,
                foe_side,
            )?);

            callbacks.extend(find_callbacks_on_field_on_mon(
                context.as_battle_context_mut(),
                event,
                fxlang::BattleEventModifier::None,
                mon,
            )?);
        }
        AllEffectsTarget::Player(player) => {
            let mut context = context.player_context(player)?;
            let side = context.side().index;
            callbacks.extend(find_callbacks_on_side(
                context.as_battle_context_mut(),
                event,
                fxlang::BattleEventModifier::None,
                side,
            )?);
            let foe_side = context.foe_side().index;
            callbacks.extend(find_callbacks_on_side(
                context.as_battle_context_mut(),
                event,
                fxlang::BattleEventModifier::Foe,
                foe_side,
            )?);

            callbacks.extend(find_callbacks_on_field(
                context.as_battle_context_mut(),
                event,
                fxlang::BattleEventModifier::None,
            )?);

            for mon in context
                .battle()
                .active_mon_handles_on_side(side)
                .collect::<Vec<_>>()
            {
                callbacks.extend(find_callbacks_on_mon(
                    context.as_battle_context_mut(),
                    event,
                    fxlang::BattleEventModifier::Side,
                    mon,
                )?);
            }
        }
        AllEffectsTarget::Side(side) => {
            callbacks.extend(find_callbacks_on_side(
                context,
                event,
                fxlang::BattleEventModifier::None,
                side,
            )?);
            let mut context = context.side_context(side)?;
            let foe_side = context.foe_side().index;
            callbacks.extend(find_callbacks_on_side(
                context.as_battle_context_mut(),
                event,
                fxlang::BattleEventModifier::Foe,
                foe_side,
            )?);

            callbacks.extend(find_callbacks_on_field(
                context.as_battle_context_mut(),
                event,
                fxlang::BattleEventModifier::None,
            )?);

            for mon in context
                .battle()
                .active_mon_handles_on_side(side)
                .collect::<Vec<_>>()
            {
                callbacks.extend(find_callbacks_on_mon(
                    context.as_battle_context_mut(),
                    event,
                    fxlang::BattleEventModifier::Side,
                    mon,
                )?);
            }
        }
        AllEffectsTarget::Field => {
            for mon in context
                .battle()
                .all_active_mon_handles()
                .collect::<Vec<_>>()
            {
                callbacks.extend(find_callbacks_on_mon(
                    context,
                    event,
                    fxlang::BattleEventModifier::None,
                    mon,
                )?);
            }
            for side in context.battle().side_indices() {
                callbacks.extend(find_callbacks_on_side(
                    context,
                    event,
                    fxlang::BattleEventModifier::None,
                    side,
                )?);
            }
            callbacks.extend(find_callbacks_on_field(
                context,
                event,
                fxlang::BattleEventModifier::None,
            )?);
        }
        AllEffectsTarget::Residual => {
            for mon in context
                .battle()
                .all_active_mon_handles()
                .collect::<Vec<_>>()
            {
                callbacks.extend(find_callbacks_on_mon(
                    context,
                    event,
                    fxlang::BattleEventModifier::None,
                    mon,
                )?);
                callbacks.extend(find_callbacks_on_side_on_mon(
                    context,
                    event,
                    fxlang::BattleEventModifier::None,
                    mon,
                )?);
                callbacks.extend(find_callbacks_on_field_on_mon(
                    context,
                    event,
                    fxlang::BattleEventModifier::None,
                    mon,
                )?);
            }
            for side in context.battle().side_indices() {
                callbacks.extend(find_callbacks_on_side(
                    context,
                    event
                        .side_event()
                        .wrap_expectation("residual event has no side event")?,
                    fxlang::BattleEventModifier::None,
                    side,
                )?);
            }
            callbacks.extend(find_callbacks_on_field(
                context,
                event
                    .field_event()
                    .wrap_expectation("residual event has no side event")?,
                fxlang::BattleEventModifier::None,
            )?);
        }
    }

    if let Some(source) = source {
        callbacks.extend(find_callbacks_on_mon(
            context,
            event,
            fxlang::BattleEventModifier::Source,
            source,
        )?);
        callbacks.extend(find_callbacks_on_side_on_mon(
            context,
            event,
            fxlang::BattleEventModifier::Source,
            source,
        )?);
        callbacks.extend(find_callbacks_on_field_on_mon(
            context,
            event,
            fxlang::BattleEventModifier::Source,
            source,
        )?);
    }

    Ok(callbacks)
}

struct SpeedOrderableCallbackHandle {
    pub callback_handle: CallbackHandle,
    pub order: u32,
    pub priority: i32,
    pub speed: u32,
    pub sub_order: u32,
}

impl SpeedOrderableCallbackHandle {
    pub fn new(callback_handle: CallbackHandle, speed: u32) -> Self {
        Self {
            callback_handle,
            order: u32::MAX,
            priority: 0,
            speed,
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

    fn sub_priority(&self) -> i32 {
        0
    }

    fn speed(&self) -> u32 {
        self.speed
    }

    fn sub_order(&self) -> u32 {
        self.sub_order
    }
}

fn get_speed_orderable_effect_handle_internal(
    context: &mut Context,
    callback_handle: CallbackHandle,
) -> Result<Option<SpeedOrderableCallbackHandle>> {
    // Ensure the effect is not ending.
    if let Some(effect_state) = callback_handle
        .applied_effect_handle
        .effect_state_connector()
    {
        if effect_state.exists(context).unwrap_or(false) {
            if effect_state
                .get_mut(context)
                .is_ok_and(|effect_state| effect_state.ending())
            {
                return Ok(None);
            }
        }
    }

    let speed = callback_handle.speed(context)?;

    // Ensure the event callback exists. An empty callback is ignored.
    let effect = CoreBattle::get_parsed_effect_by_handle(
        context,
        &callback_handle.applied_effect_handle.effect_handle,
    )?;
    let callback = match effect
        .as_ref()
        .map(|effect| effect.event(callback_handle.event, callback_handle.modifier))
        .flatten()
    {
        Some(callback) => callback,
        None => return Ok(None),
    };

    let mut result = SpeedOrderableCallbackHandle::new(callback_handle, speed);
    result.order = callback.order();
    result.priority = callback.priority();
    result.sub_order = callback.sub_order();
    Ok(Some(result))
}

fn get_speed_orderable_effect_handle(
    context: &mut Context,
    callback_handle: CallbackHandle,
) -> Result<Option<SpeedOrderableCallbackHandle>> {
    match get_speed_orderable_effect_handle_internal(context, callback_handle.clone())? {
        Some(handle) => Ok(Some(handle)),
        None => {
            if callback_handle.event.force_default_callback() {
                Ok(Some(SpeedOrderableCallbackHandle::new(callback_handle, 0)))
            } else {
                Ok(None)
            }
        }
    }
}

fn filter_and_order_effects_for_event(
    context: &mut Context,
    callback_handles: Vec<CallbackHandle>,
) -> Result<Vec<CallbackHandle>> {
    let mut speed_orderable_handles = Vec::new();
    speed_orderable_handles.reserve(callback_handles.len());
    for effect_handle in callback_handles {
        match get_speed_orderable_effect_handle(context, effect_handle)? {
            Some(handle) => speed_orderable_handles.push(handle),
            None => (),
        }
    }

    CoreBattle::speed_sort(context, speed_orderable_handles.as_mut_slice());
    Ok(speed_orderable_handles
        .into_iter()
        .map(|handle| handle.callback_handle)
        .collect())
}

struct RunCallbacksOptions {
    pub return_first_value: bool,
}

impl Default for RunCallbacksOptions {
    fn default() -> Self {
        Self {
            return_first_value: false,
        }
    }
}

fn run_callbacks_with_forwarding_input_with_errors(
    context: UpcomingEvaluationContext,
    input: &mut fxlang::VariableInput,
    event_state: &fxlang::EventState,
    callback_handle: CallbackHandle,
    options: &RunCallbacksOptions,
) -> Result<Option<fxlang::Value>> {
    let value = run_callback_with_errors(context, input.clone(), event_state, callback_handle)?;
    // Support for early exit.
    if let Some(value) = value {
        if value.signals_early_exit() || options.return_first_value {
            return Ok(Some(value));
        }
        // Pass the output to the next effect.
        if let Some(forward_input) = input.get_mut(0) {
            *forward_input = value;
        } else {
            *input = fxlang::VariableInput::from_iter([value]);
        }
    }

    Ok(None)
}

fn run_mon_callbacks_with_errors(
    context: &mut MonContext,
    source_effect: Option<&EffectHandle>,
    source: Option<MonHandle>,
    mut input: fxlang::VariableInput,
    options: &RunCallbacksOptions,
    callbacks: Vec<CallbackHandle>,
) -> Result<Option<fxlang::Value>> {
    let event_state = fxlang::EventState::default();
    for callback_handle in callbacks {
        if let Some(id) = callback_handle.applied_effect_handle.effect_handle.try_id() {
            if let Some(id) = context.battle_mut().resolve_effect_id(id)
                && !event_state.effect_should_run(id.as_ref())
            {
                continue;
            }
        }

        let result = match source_effect {
            Some(source_effect) => run_callbacks_with_forwarding_input_with_errors(
                UpcomingEvaluationContext::ApplyingEffect(
                    context
                        .applying_effect_context(source_effect.clone(), source, None)?
                        .into(),
                ),
                &mut input,
                &event_state,
                callback_handle,
                options,
            )?,
            None => run_callbacks_with_forwarding_input_with_errors(
                UpcomingEvaluationContext::Mon(context.into()),
                &mut input,
                &event_state,
                callback_handle,
                options,
            )?,
        };
        if let Some(return_value) = result {
            return Ok(Some(return_value));
        }
    }

    // The first input variable is always returned as the result.
    Ok(input.get(0).cloned())
}

fn run_player_callbacks_with_errors(
    context: &mut PlayerContext,
    source_effect: Option<&EffectHandle>,
    source: Option<MonHandle>,
    mut input: fxlang::VariableInput,
    options: &RunCallbacksOptions,
    callbacks: Vec<CallbackHandle>,
) -> Result<Option<fxlang::Value>> {
    let event_state = fxlang::EventState::default();
    for callback_handle in callbacks {
        if let Some(id) = callback_handle.applied_effect_handle.effect_handle.try_id() {
            if let Some(id) = context.battle_mut().resolve_effect_id(id)
                && !event_state.effect_should_run(id.as_ref())
            {
                continue;
            }
        }

        let result = match source_effect {
            Some(source_effect) => run_callbacks_with_forwarding_input_with_errors(
                UpcomingEvaluationContext::PlayerEffect(
                    context
                        .player_effect_context(source_effect.clone(), source, None)?
                        .into(),
                ),
                &mut input,
                &event_state,
                callback_handle,
                options,
            )?,
            None => run_callbacks_with_forwarding_input_with_errors(
                UpcomingEvaluationContext::Player(context.into()),
                &mut input,
                &event_state,
                callback_handle,
                options,
            )?,
        };
        if let Some(return_value) = result {
            return Ok(Some(return_value));
        }
    }

    // The first input variable is always returned as the result.
    Ok(input.get(0).cloned())
}

fn run_side_callbacks_with_errors(
    context: &mut SideContext,
    source_effect: Option<&EffectHandle>,
    source: Option<MonHandle>,
    mut input: fxlang::VariableInput,
    options: &RunCallbacksOptions,
    callbacks: Vec<CallbackHandle>,
) -> Result<Option<fxlang::Value>> {
    let event_state = fxlang::EventState::default();
    for callback_handle in callbacks {
        if let Some(id) = callback_handle.applied_effect_handle.effect_handle.try_id() {
            if let Some(id) = context.battle_mut().resolve_effect_id(id)
                && !event_state.effect_should_run(id.as_ref())
            {
                continue;
            }
        }

        let result = match source_effect {
            Some(source_effect) => run_callbacks_with_forwarding_input_with_errors(
                UpcomingEvaluationContext::SideEffect(
                    context
                        .side_effect_context(source_effect.clone(), source, None)?
                        .into(),
                ),
                &mut input,
                &event_state,
                callback_handle,
                options,
            )?,
            None => run_callbacks_with_forwarding_input_with_errors(
                UpcomingEvaluationContext::Side(context.into()),
                &mut input,
                &event_state,
                callback_handle,
                options,
            )?,
        };
        if let Some(return_value) = result {
            return Ok(Some(return_value));
        }
    }

    // The first input variable is always returned as the result.
    Ok(input.get(0).cloned())
}

fn run_field_callbacks_with_errors(
    context: &mut Context,
    source_effect: Option<&EffectHandle>,
    source: Option<MonHandle>,
    mut input: fxlang::VariableInput,
    options: &RunCallbacksOptions,
    callbacks: Vec<CallbackHandle>,
) -> Result<Option<fxlang::Value>> {
    let event_state = fxlang::EventState::default();
    for callback_handle in callbacks {
        if let Some(id) = callback_handle.applied_effect_handle.effect_handle.try_id() {
            if let Some(id) = context.battle_mut().resolve_effect_id(id)
                && !event_state.effect_should_run(id.as_ref())
            {
                continue;
            }
        }

        let result = match source_effect {
            Some(source_effect) => run_callbacks_with_forwarding_input_with_errors(
                UpcomingEvaluationContext::FieldEffect(
                    context
                        .field_effect_context(source_effect.clone(), source, None)?
                        .into(),
                ),
                &mut input,
                &event_state,
                callback_handle,
                options,
            )?,
            None => run_callbacks_with_forwarding_input_with_errors(
                UpcomingEvaluationContext::Field(context.into()),
                &mut input,
                &event_state,
                callback_handle,
                options,
            )?,
        };
        if let Some(return_value) = result {
            return Ok(Some(return_value));
        }
    }

    // The first input variable is always returned as the result.
    Ok(input.get(0).cloned())
}

fn run_residual_callbacks_with_errors(
    context: &mut Context,
    callbacks: Vec<CallbackHandle>,
) -> Result<()> {
    // Ensure we only decrease the duration of each event once.
    let mut duration_decreased = HashSet::default();

    let event_state = fxlang::EventState::default();
    for callback_handle in callbacks {
        if let Some(id) = callback_handle.applied_effect_handle.effect_handle.try_id() {
            if let Some(id) = context.battle_mut().resolve_effect_id(id)
                && !event_state.effect_should_run(id.as_ref())
            {
                continue;
            }
        }

        if context.battle().ending() {
            break;
        }

        let mut context = match context.effect_context(
            callback_handle.applied_effect_handle.effect_handle.clone(),
            None,
        ) {
            Ok(context) => context,
            Err(_) => continue,
        };

        let mut ended = false;
        if duration_decreased.insert((
            callback_handle.applied_effect_handle.effect_handle.clone(),
            callback_handle
                .applied_effect_handle
                .location
                .for_residual(),
        )) {
            if let Some(effect_state_connector) = callback_handle
                .applied_effect_handle
                .effect_state_connector()
            {
                if effect_state_connector.exists(context.as_battle_context_mut())? {
                    let effect_state =
                        effect_state_connector.get_mut(context.as_battle_context_mut())?;
                    if let Some(duration) = effect_state.duration() {
                        let duration = if duration > 0 { duration - 1 } else { duration };
                        effect_state.set_duration(duration);
                        if duration == 0 {
                            ended = true;
                        }
                    }
                }
            }
        }

        if ended {
            if callback_handle.applied_effect_handle.end(&mut context)? {
                continue;
            }
        }

        match callback_handle.applied_effect_handle.location {
            AppliedEffectLocation::None => {
                run_callback_with_errors(
                    UpcomingEvaluationContext::Effect(context.into()),
                    fxlang::VariableInput::default(),
                    &event_state,
                    callback_handle,
                )?;
            }
            AppliedEffectLocation::MonAbility(mon) => {
                let context = context.applying_effect_context(None, mon)?;
                run_callback_with_errors(
                    UpcomingEvaluationContext::ApplyingEffect(context.into()),
                    fxlang::VariableInput::default(),
                    &event_state,
                    callback_handle,
                )?;
            }
            AppliedEffectLocation::MonInactiveMove(mon) => {
                let context = context.applying_effect_context(None, mon)?;
                run_callback_with_errors(
                    UpcomingEvaluationContext::ApplyingEffect(context.into()),
                    fxlang::VariableInput::default(),
                    &event_state,
                    callback_handle,
                )?;
            }
            AppliedEffectLocation::MonItem(mon) => {
                let context = context.applying_effect_context(None, mon)?;
                run_callback_with_errors(
                    UpcomingEvaluationContext::ApplyingEffect(context.into()),
                    fxlang::VariableInput::default(),
                    &event_state,
                    callback_handle,
                )?;
            }
            AppliedEffectLocation::MonPseudoWeather(mon) => {
                let context = context.applying_effect_context(None, mon)?;
                run_callback_with_errors(
                    UpcomingEvaluationContext::ApplyingEffect(context.into()),
                    fxlang::VariableInput::default(),
                    &event_state,
                    callback_handle,
                )?;
            }
            AppliedEffectLocation::MonStatus(mon) => {
                let context = context.applying_effect_context(None, mon)?;
                run_callback_with_errors(
                    UpcomingEvaluationContext::ApplyingEffect(context.into()),
                    fxlang::VariableInput::default(),
                    &event_state,
                    callback_handle,
                )?;
            }
            AppliedEffectLocation::Mon(mon) | AppliedEffectLocation::MonType(mon) => {
                let context = context.applying_effect_context(None, mon)?;
                run_callback_with_errors(
                    UpcomingEvaluationContext::ApplyingEffect(context.into()),
                    fxlang::VariableInput::default(),
                    &event_state,
                    callback_handle,
                )?;
            }
            AppliedEffectLocation::MonSideCondition(_, mon) => {
                let context = context.applying_effect_context(None, mon)?;
                run_callback_with_errors(
                    UpcomingEvaluationContext::ApplyingEffect(context.into()),
                    fxlang::VariableInput::default(),
                    &event_state,
                    callback_handle,
                )?;
            }
            AppliedEffectLocation::MonSlotCondition(_, _, mon) => {
                let context = context.applying_effect_context(None, mon)?;
                run_callback_with_errors(
                    UpcomingEvaluationContext::ApplyingEffect(context.into()),
                    fxlang::VariableInput::default(),
                    &event_state,
                    callback_handle,
                )?;
            }
            AppliedEffectLocation::MonTerrain(mon) => {
                let context = context.applying_effect_context(None, mon)?;
                run_callback_with_errors(
                    UpcomingEvaluationContext::ApplyingEffect(context.into()),
                    fxlang::VariableInput::default(),
                    &event_state,
                    callback_handle,
                )?;
            }
            AppliedEffectLocation::MonVolatile(mon) => {
                let context = context.applying_effect_context(None, mon)?;
                run_callback_with_errors(
                    UpcomingEvaluationContext::ApplyingEffect(context.into()),
                    fxlang::VariableInput::default(),
                    &event_state,
                    callback_handle,
                )?;
            }
            AppliedEffectLocation::MonWeather(mon) => {
                let context = context.applying_effect_context(None, mon)?;
                run_callback_with_errors(
                    UpcomingEvaluationContext::ApplyingEffect(context.into()),
                    fxlang::VariableInput::default(),
                    &event_state,
                    callback_handle,
                )?;
            }
            AppliedEffectLocation::PseudoWeather => {
                let context = context.field_effect_context(None)?;
                run_callback_with_errors(
                    UpcomingEvaluationContext::FieldEffect(context.into()),
                    fxlang::VariableInput::default(),
                    &event_state,
                    callback_handle,
                )?;
            }
            AppliedEffectLocation::SideCondition(side) => {
                let context = context.side_effect_context(side, None)?;
                run_callback_with_errors(
                    UpcomingEvaluationContext::SideEffect(context.into()),
                    fxlang::VariableInput::default(),
                    &event_state,
                    callback_handle,
                )?;
            }
            AppliedEffectLocation::SlotCondition(side, _) => {
                let context = context.side_effect_context(side, None)?;
                run_callback_with_errors(
                    UpcomingEvaluationContext::SideEffect(context.into()),
                    fxlang::VariableInput::default(),
                    &event_state,
                    callback_handle,
                )?;
            }
            AppliedEffectLocation::Terrain => {
                let context = context.field_effect_context(None)?;
                run_callback_with_errors(
                    UpcomingEvaluationContext::FieldEffect(context.into()),
                    fxlang::VariableInput::default(),
                    &event_state,
                    callback_handle,
                )?;
            }
            AppliedEffectLocation::Weather => {
                let context = context.field_effect_context(None)?;
                run_callback_with_errors(
                    UpcomingEvaluationContext::FieldEffect(context.into()),
                    fxlang::VariableInput::default(),
                    &event_state,
                    callback_handle,
                )?;
            }
        }
    }
    Ok(())
}

fn run_event_with_errors(
    context: &mut Context,
    event: fxlang::BattleEvent,
    source_effect: Option<&EffectHandle>,
    target: AllEffectsTarget,
    source: Option<MonHandle>,
    input: fxlang::VariableInput,
    options: &RunCallbacksOptions,
) -> Result<Option<fxlang::Value>> {
    let mut callbacks = find_all_callbacks(context, event, target, source)?;
    if event.run_callback_on_source_effect() {
        if let Some(source_effect) = source_effect {
            callbacks.push(CallbackHandle::new(
                source_effect.clone(),
                event,
                fxlang::BattleEventModifier::None,
                AppliedEffectLocation::None,
            ));
        }
    }
    let mut callbacks = filter_and_order_effects_for_event(context, callbacks)?;
    callbacks.dedup();

    match target {
        AllEffectsTarget::Mon(mon) => run_mon_callbacks_with_errors(
            &mut context.mon_context(mon)?,
            source_effect,
            source,
            input,
            options,
            callbacks,
        ),
        AllEffectsTarget::Player(player) => run_player_callbacks_with_errors(
            &mut context.player_context(player)?,
            source_effect,
            source,
            input,
            options,
            callbacks,
        ),
        AllEffectsTarget::Side(side) => run_side_callbacks_with_errors(
            &mut context.side_context(side)?,
            source_effect,
            source,
            input,
            options,
            callbacks,
        ),
        AllEffectsTarget::Field => run_field_callbacks_with_errors(
            context,
            source_effect,
            source,
            input,
            options,
            callbacks,
        ),
        AllEffectsTarget::Residual => {
            run_residual_callbacks_with_errors(context, callbacks).map(|()| None)
        }
    }
}

fn run_event_for_applying_effect_internal(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
    options: &RunCallbacksOptions,
) -> Option<fxlang::Value> {
    let target = AllEffectsTarget::Mon(context.target_handle());
    let effect = context.effect_handle().clone();
    let source = context.source_handle();
    match run_event_with_errors(
        context.as_battle_context_mut(),
        event,
        Some(&effect),
        target,
        source,
        input,
        options,
    ) {
        Ok(value) => value,
        Err(error) => {
            core_battle_logs::debug_full_event_failure(
                context.as_battle_context_mut(),
                event,
                &&format!("{error:#}"),
            );
            None
        }
    }
}

fn run_event_for_mon_internal(
    context: &mut MonContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
    options: &RunCallbacksOptions,
) -> Option<fxlang::Value> {
    let target = AllEffectsTarget::Mon(context.mon_handle());
    match run_event_with_errors(
        context.as_battle_context_mut(),
        event,
        None,
        target,
        None,
        input,
        options,
    ) {
        Ok(value) => value,
        Err(error) => {
            core_battle_logs::debug_full_event_failure(
                context.as_battle_context_mut(),
                event,
                &&format!("{error:#}"),
            );
            None
        }
    }
}

fn run_event_for_player_internal(
    context: &mut PlayerContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
    options: &RunCallbacksOptions,
) -> Option<fxlang::Value> {
    let target = AllEffectsTarget::Player(context.player().index);
    match run_event_with_errors(
        context.as_battle_context_mut(),
        event,
        None,
        target,
        None,
        input,
        options,
    ) {
        Ok(value) => value,
        Err(error) => {
            core_battle_logs::debug_full_event_failure(
                context.as_battle_context_mut(),
                event,
                &&format!("{error:#}"),
            );
            None
        }
    }
}

fn run_event_for_player_effect_internal(
    context: &mut PlayerEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
    options: &RunCallbacksOptions,
) -> Option<fxlang::Value> {
    let target = AllEffectsTarget::Player(context.player().index);
    let effect = context.effect_handle().clone();
    let source = context.source_handle();
    match run_event_with_errors(
        context.as_battle_context_mut(),
        event,
        Some(&effect),
        target,
        source,
        input,
        options,
    ) {
        Ok(value) => value,
        Err(error) => {
            core_battle_logs::debug_full_event_failure(
                context.as_battle_context_mut(),
                event,
                &&format!("{error:#}"),
            );
            None
        }
    }
}

fn run_event_for_side_effect_internal(
    context: &mut SideEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
    options: &RunCallbacksOptions,
) -> Option<fxlang::Value> {
    let target = AllEffectsTarget::Side(context.side().index);
    let effect = context.effect_handle().clone();
    let source = context.source_handle();
    match run_event_with_errors(
        context.as_battle_context_mut(),
        event,
        Some(&effect),
        target,
        source,
        input,
        options,
    ) {
        Ok(value) => value,
        Err(error) => {
            core_battle_logs::debug_full_event_failure(
                context.as_battle_context_mut(),
                event,
                &&format!("{error:#}"),
            );
            None
        }
    }
}

fn run_event_for_field_effect_internal(
    context: &mut FieldEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
    options: &RunCallbacksOptions,
) -> Option<fxlang::Value> {
    let target = AllEffectsTarget::Field;
    let effect = context.effect_handle().clone();
    let source = context.source_handle();
    match run_event_with_errors(
        context.as_battle_context_mut(),
        event,
        Some(&effect),
        target,
        source,
        input,
        options,
    ) {
        Ok(value) => value,
        Err(error) => {
            core_battle_logs::debug_full_event_failure(
                context.as_battle_context_mut(),
                event,
                &&format!("{error:#}"),
            );
            None
        }
    }
}

fn run_event_for_battle_internal(
    context: &mut Context,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
    options: &RunCallbacksOptions,
) -> Option<fxlang::Value> {
    match run_event_with_errors(
        context,
        event,
        None,
        AllEffectsTarget::Field,
        None,
        input,
        options,
    ) {
        Ok(value) => value,
        Err(error) => {
            core_battle_logs::debug_full_event_failure(context, event, &&format!("{error:#}"));
            None
        }
    }
}

fn run_event_for_residual_internal(context: &mut Context, event: fxlang::BattleEvent) {
    match run_event_with_errors(
        context,
        event,
        None,
        AllEffectsTarget::Residual,
        None,
        fxlang::VariableInput::default(),
        &RunCallbacksOptions::default(),
    ) {
        Ok(_) => (),
        Err(error) => {
            core_battle_logs::debug_full_event_failure(context, event, &&format!("{error:#}"));
        }
    }
}

/// Runs an event on an active [`Move`][`crate::moves::Move`].
pub fn run_active_move_event_expecting_void(
    context: &mut ActiveMoveContext,
    event: fxlang::BattleEvent,
    target: MoveTargetForEvent,
    input: fxlang::VariableInput,
) {
    run_active_move_event(context, event, target, input);
}

/// Runs an event on an active [`Move`][`crate::moves::Move`].
///
/// Expects an integer that can fit in a [`u16`].
pub fn run_active_move_event_expecting_u16(
    context: &mut ActiveMoveContext,
    event: fxlang::BattleEvent,
    target: MoveTargetForEvent,
) -> Option<u16> {
    run_active_move_event(context, event, target, fxlang::VariableInput::default())?
        .integer_u16()
        .ok()
}

/// Runs an event on an active [`Move`][`crate::moves::Move`].
///
/// Expects an integer that can fit in a [`u32`].
pub fn run_active_move_event_expecting_u32(
    context: &mut ActiveMoveContext,
    event: fxlang::BattleEvent,
    target: MoveTargetForEvent,
) -> Option<u32> {
    run_active_move_event(context, event, target, fxlang::VariableInput::default())?
        .integer_u32()
        .ok()
}

/// Runs an event on an active [`Move`][`crate::moves::Move`].
///
/// Expects an integer that can fit in a [`i8`].
pub fn run_active_move_event_expecting_i8(
    context: &mut ActiveMoveContext,
    event: fxlang::BattleEvent,
    target: MoveTargetForEvent,
    input: fxlang::VariableInput,
) -> Option<i8> {
    run_active_move_event(context, event, target, input)?
        .integer_i8()
        .ok()
}

/// Runs an event on an active [`Move`][`crate::moves::Move`].
///
/// Expects a [`bool`].
pub fn run_active_move_event_expecting_bool(
    context: &mut ActiveMoveContext,
    event: fxlang::BattleEvent,
    target: MoveTargetForEvent,
) -> Option<bool> {
    run_active_move_event(context, event, target, fxlang::VariableInput::default())?
        .boolean()
        .ok()
}

/// Runs an event on an active [`Move`][`crate::moves::Move`].
///
/// Expects a [`MoveEventResult`].
pub fn run_active_move_event_expecting_move_event_result(
    context: &mut ActiveMoveContext,
    event: fxlang::BattleEvent,
    target: MoveTargetForEvent,
) -> MoveEventResult {
    match run_active_move_event(context, event, target, fxlang::VariableInput::default()) {
        Some(value) => value.move_result().unwrap_or(MoveEventResult::Advance),
        None => MoveEventResult::Advance,
    }
}

/// Runs an event on the target [`Mon`]'s current status.
///
/// Expects an integer that can fit in a [`u8`].
pub fn run_mon_status_event_expecting_u8(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
) -> Option<u8> {
    run_mon_status_event(context, event, fxlang::VariableInput::default())?
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
    run_mon_status_event(context, event, fxlang::VariableInput::default())?
        .boolean()
        .ok()
}

/// Runs an event on the target [`Mon`]'s volatile status.
pub fn run_mon_volatile_event_expecting_void(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    status: &Id,
) {
    run_mon_volatile_event(context, event, fxlang::VariableInput::default(), status);
}

/// Runs an event on the target [`Mon`]'s volatile status.
///
/// Expects a [`bool`].
pub fn run_mon_volatile_event_expecting_bool(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    status: &Id,
) -> Option<bool> {
    run_mon_volatile_event(context, event, fxlang::VariableInput::default(), status)?
        .boolean()
        .ok()
}

/// Runs an event on the target [`Mon`]'s volatile status.
///
/// Expects an integer that can fit in a [`u8`].
pub fn run_mon_volatile_event_expecting_u8(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    status: &Id,
) -> Option<u8> {
    run_mon_volatile_event(context, event, fxlang::VariableInput::default(), status)?
        .integer_u8()
        .ok()
}

/// Runs an event on the target [`Mon`]'s current ability.
pub fn run_mon_ability_event_expecting_void(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
) {
    run_mon_ability_event(context, event, fxlang::VariableInput::default());
}

/// Runs an event on the target [`Mon`]'s current item.
pub fn run_mon_item_event_expecting_void(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
) {
    run_mon_item_event(context, event, fxlang::VariableInput::default());
}

/// Runs an event on the target [`Mon`]'s current item.
///
/// Expects a [`bool`].
pub fn run_mon_item_event_expecting_bool(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
) -> Option<bool> {
    run_mon_item_event(context, event, fxlang::VariableInput::default())?
        .boolean()
        .ok()
}

/// Runs an event on the target [`Mon`]'s inactive move.
///
/// Expects a [`MoveTarget`].
pub fn run_mon_inactive_move_event_expecting_move_target(
    context: &mut MonContext,
    event: fxlang::BattleEvent,
    mov: &Id,
) -> Option<MoveTarget> {
    run_mon_inactive_move_event(context, event, fxlang::VariableInput::default(), mov)?
        .move_target()
        .ok()
}

/// Runs an event on the target [`Side`][`crate::battle::Side`]'s side condition.
pub fn run_side_condition_event_expecting_void(
    context: &mut SideEffectContext,
    event: fxlang::BattleEvent,
    condition: &Id,
) {
    run_side_condition_event(context, event, fxlang::VariableInput::default(), condition);
}

/// Runs an event on the target [`Side`][`crate::battle::Side`]'s side condition.
///
/// Expects a [`bool`].
pub fn run_side_condition_event_expecting_bool(
    context: &mut SideEffectContext,
    event: fxlang::BattleEvent,
    condition: &Id,
) -> Option<bool> {
    run_side_condition_event(context, event, fxlang::VariableInput::default(), condition)?
        .boolean()
        .ok()
}

/// Runs an event on the target [`Side`][`crate::battle::Side`]'s side condition.
///
/// Expects an integer that can fit in a [`u8`].
pub fn run_side_condition_event_expecting_u8(
    context: &mut SideEffectContext,
    event: fxlang::BattleEvent,
    condition: &Id,
) -> Option<u8> {
    run_side_condition_event(context, event, fxlang::VariableInput::default(), condition)?
        .integer_u8()
        .ok()
}

/// Runs an event on the target [`Side`][`crate::battle::Side`]'s slot condition.
pub fn run_slot_condition_event_expecting_void(
    context: &mut SideEffectContext,
    event: fxlang::BattleEvent,
    slot: usize,
    condition: &Id,
) {
    match TryInto::<u64>::try_into(slot) {
        Ok(value) => {
            run_slot_condition_event(
                context,
                event,
                fxlang::VariableInput::from_iter([fxlang::Value::UFraction(value.into())]),
                slot,
                condition,
            );
        }
        Err(_) => (),
    }
}

/// Runs an event on the target [`Side`][`crate::battle::Side`]'s slot condition.
///
/// Expects a [`bool`].
pub fn run_slot_condition_event_expecting_bool(
    context: &mut SideEffectContext,
    event: fxlang::BattleEvent,
    slot: usize,
    condition: &Id,
) -> Option<bool> {
    run_slot_condition_event(
        context,
        event,
        fxlang::VariableInput::from_iter([fxlang::Value::UFraction(
            TryInto::<u64>::try_into(slot).ok()?.into(),
        )]),
        slot,
        condition,
    )?
    .boolean()
    .ok()
}

/// Runs an event on the target [`Side`][`crate::battle::Side`]'s slot condition.
///
/// Expects an integer that can fit in a [`u8`].
pub fn run_slot_condition_event_expecting_u8(
    context: &mut SideEffectContext,
    event: fxlang::BattleEvent,
    slot: usize,
    condition: &Id,
) -> Option<u8> {
    run_slot_condition_event(
        context,
        event,
        fxlang::VariableInput::from_iter([fxlang::Value::UFraction(
            TryInto::<u64>::try_into(slot).ok()?.into(),
        )]),
        slot,
        condition,
    )?
    .integer_u8()
    .ok()
}

/// Runs an event on the [`Field`][`crate::battle::Field`]'s current weather.
pub fn run_weather_event_expecting_void(
    context: &mut FieldEffectContext,
    event: fxlang::BattleEvent,
) {
    run_weather_event(context, event, fxlang::VariableInput::default());
}

/// Runs an event on the [`Field`][`crate::battle::Field`]'s weather.
///
/// Expects a [`bool`].
pub fn run_weather_event_expecting_bool(
    context: &mut FieldEffectContext,
    event: fxlang::BattleEvent,
) -> Option<bool> {
    run_weather_event(context, event, fxlang::VariableInput::default())?
        .boolean()
        .ok()
}

/// Runs an event on the [`Field`][`crate::battle::Field`]'s weather.
///
/// Expects an integer that can fit in a [`u8`].
pub fn run_weather_event_expecting_u8(
    context: &mut FieldEffectContext,
    event: fxlang::BattleEvent,
) -> Option<u8> {
    run_weather_event(context, event, fxlang::VariableInput::default())?
        .integer_u8()
        .ok()
}

/// Runs an event on the [`Field`][`crate::battle::Field`]'s current terrain.
pub fn run_terrain_event_expecting_void(
    context: &mut FieldEffectContext,
    event: fxlang::BattleEvent,
) {
    run_terrain_event(context, event, fxlang::VariableInput::default());
}

/// Runs an event on the [`Field`][`crate::battle::Field`]'s terrain.
///
/// Expects a [`bool`].
pub fn run_terrain_event_expecting_bool(
    context: &mut FieldEffectContext,
    event: fxlang::BattleEvent,
) -> Option<bool> {
    run_terrain_event(context, event, fxlang::VariableInput::default())?
        .boolean()
        .ok()
}

/// Runs an event on the [`Field`][`crate::battle::Field`]'s terrain.
///
/// Expects an integer that can fit in a [`u8`].
pub fn run_terrain_event_expecting_u8(
    context: &mut FieldEffectContext,
    event: fxlang::BattleEvent,
) -> Option<u8> {
    run_terrain_event(context, event, fxlang::VariableInput::default())?
        .integer_u8()
        .ok()
}

/// Runs an event on one of the [`Field`][`crate::battle::Field`]'s pseudo-weather.
pub fn run_pseudo_weather_event_expecting_void(
    context: &mut FieldEffectContext,
    event: fxlang::BattleEvent,
    pseudo_weather: &Id,
) {
    run_pseudo_weather_event(
        context,
        event,
        fxlang::VariableInput::default(),
        pseudo_weather,
    );
}

/// Runs an event on one of the [`Field`][`crate::battle::Field`]'s pseudo-weather.
///
/// Expects a [`bool`].
pub fn run_pseudo_weather_event_expecting_bool(
    context: &mut FieldEffectContext,
    event: fxlang::BattleEvent,
    pseudo_weather: &Id,
) -> Option<bool> {
    run_pseudo_weather_event(
        context,
        event,
        fxlang::VariableInput::default(),
        pseudo_weather,
    )?
    .boolean()
    .ok()
}

/// Runs an event on one of the [`Field`][`crate::battle::Field`]'s pseudo-weather.
///
/// Expects an integer that can fit in a [`u8`].
pub fn run_pseudo_weather_event_expecting_u8(
    context: &mut FieldEffectContext,
    event: fxlang::BattleEvent,
    pseudo_weather: &Id,
) -> Option<u8> {
    run_pseudo_weather_event(
        context,
        event,
        fxlang::VariableInput::default(),
        pseudo_weather,
    )?
    .integer_u8()
    .ok()
}

/// Runs an event on the applying [`Effect`][`crate::effect::Effect`].
pub fn run_applying_effect_event_expecting_void(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) {
    run_applying_effect_event(context, event, input);
}

/// Runs an event on the applying [`Effect`][`crate::effect::Effect`].
pub fn run_applying_effect_event_expecting_bool(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> Option<bool> {
    run_applying_effect_event(context, event, input)?
        .boolean()
        .ok()
}

/// Runs an event on the applying [`Effect`][`crate::effect::Effect`].
pub fn run_applying_effect_event_expecting_u64(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> Option<u64> {
    run_applying_effect_event(context, event, input)?
        .integer_u64()
        .ok()
}

/// Runs an event on the [`Effect`][`crate::effect::Effect`].
///
/// Expects a [`bool`].
pub fn run_effect_event_expecting_bool(
    context: &mut EffectContext,
    event: fxlang::BattleEvent,
) -> Option<bool> {
    run_effect_event(context, event, fxlang::VariableInput::default())?
        .boolean()
        .ok()
}

/// Runs an event on the [`CoreBattle`] for an applying effect.
///
/// Returns `true` if all event handlers succeeded (i.e., did not return `false`).
pub fn run_event_for_applying_effect(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> bool {
    run_event_for_applying_effect_internal(context, event, input, &RunCallbacksOptions::default())
        .map(|value| value.boolean().ok())
        .flatten()
        .unwrap_or(true)
}

/// Runs an event on the [`CoreBattle`] for an applying effect.
///
/// Expects a [`bool`]. Returns the value of the first callback that returns a value.
pub fn run_event_for_applying_effect_expecting_bool_quick_return(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    default: bool,
) -> bool {
    run_event_for_applying_effect_internal(
        context,
        event,
        fxlang::VariableInput::default(),
        &RunCallbacksOptions {
            return_first_value: true,
        },
    )
    .map(|value| value.boolean().ok())
    .flatten()
    .unwrap_or(default)
}

/// Runs an event on the [`CoreBattle`] for an applying effect.
///
/// Expects an integer that can fit in a [`u32`].
pub fn run_event_for_applying_effect_expecting_u32(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: u32,
) -> u32 {
    match run_event_for_applying_effect_internal(
        context,
        event,
        fxlang::VariableInput::from_iter([fxlang::Value::UFraction(input.into())]),
        &RunCallbacksOptions::default(),
    ) {
        Some(value) => value.integer_u32().unwrap_or(input),
        None => input,
    }
}

/// Runs an event on the [`CoreBattle`] for an applying effect.
///
/// Expects an integer that can fit in a [`u64`].
pub fn run_event_for_applying_effect_expecting_u64(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: u64,
) -> u64 {
    match run_event_for_applying_effect_internal(
        context,
        event,
        fxlang::VariableInput::from_iter([fxlang::Value::UFraction(input.into())]),
        &RunCallbacksOptions::default(),
    ) {
        Some(value) => value.integer_u64().unwrap_or(input),
        None => input,
    }
}

/// Runs an event on the [`CoreBattle`] for an applying effect.
///
/// Expects an integer that can fit in a [`u8`].
pub fn run_event_for_applying_effect_expecting_u8(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: u8,
) -> u8 {
    match run_event_for_applying_effect_internal(
        context,
        event,
        fxlang::VariableInput::from_iter([fxlang::Value::UFraction(input.into())]),
        &RunCallbacksOptions::default(),
    ) {
        Some(value) => value.integer_u8().unwrap_or(input),
        None => input,
    }
}

/// Runs an event on the [`CoreBattle`] for an applying effect.
///
/// Expects an integer that can fit in an [`i8`].
pub fn run_event_for_applying_effect_expecting_i8(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: i8,
    other_input: fxlang::VariableInput,
) -> i8 {
    match run_event_for_applying_effect_internal(
        context,
        event,
        fxlang::VariableInput::from_iter(
            iter::once(fxlang::Value::Fraction(input.into())).chain(other_input.into_iter()),
        ),
        &RunCallbacksOptions::default(),
    ) {
        Some(value) => value.integer_i8().unwrap_or(input),
        None => input,
    }
}

/// Runs an event on the [`CoreBattle`] for an applying effect.
///
/// Expects an integer that can fit in an [`i32`].
pub fn run_event_for_applying_effect_expecting_i32(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: i32,
) -> i32 {
    match run_event_for_applying_effect_internal(
        context,
        event,
        fxlang::VariableInput::from_iter([fxlang::Value::Fraction(input.into())]),
        &RunCallbacksOptions::default(),
    ) {
        Some(value) => value.integer_i32().unwrap_or(input),
        None => input,
    }
}

/// Runs an event on the [`CoreBattle`] for an applying effect.
///
/// Expects an integer that can fit in a [`u16`].
pub fn run_event_for_applying_effect_expecting_u16(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: u16,
) -> u16 {
    match run_event_for_applying_effect_internal(
        context,
        event,
        fxlang::VariableInput::from_iter([fxlang::Value::UFraction(input.into())]),
        &RunCallbacksOptions::default(),
    ) {
        Some(value) => value.integer_u16().unwrap_or(input),
        None => input,
    }
}

/// Runs an event on the [`CoreBattle`] for an applying effect.
///
/// Expects an integer that can fit in a [`Fraction<u32>`].
pub fn run_event_for_applying_effect_expecting_fraction_u32(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: Fraction<u32>,
) -> Fraction<u32> {
    match run_event_for_applying_effect_internal(
        context,
        event,
        fxlang::VariableInput::from_iter([fxlang::Value::UFraction(input.convert())]),
        &RunCallbacksOptions::default(),
    ) {
        Some(value) => value.fraction_u32().unwrap_or(input),
        None => input,
    }
}

/// Runs an event on the [`CoreBattle`] for an applying effect.
///
/// Expects a [`MoveOutcomeOnTarget`].
pub fn run_event_for_applying_effect_expecting_move_outcome_on_target(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
) -> Option<MoveOutcomeOnTarget> {
    run_event_for_applying_effect_internal(
        context,
        event,
        fxlang::VariableInput::default(),
        &RunCallbacksOptions::default(),
    )?
    .move_outcome_on_target()
    .ok()
}

/// Runs an event on the [`CoreBattle`] for an applying effect.
///
/// Expects a [`BoostTable`].
pub fn run_event_for_applying_effect_expecting_boost_table(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    boost_table: BoostTable,
) -> BoostTable {
    match run_event_for_applying_effect_internal(
        context,
        event,
        fxlang::VariableInput::from_iter([fxlang::Value::BoostTable(boost_table.clone())]),
        &RunCallbacksOptions::default(),
    ) {
        Some(value) => value.boost_table().unwrap_or(boost_table),
        None => boost_table,
    }
}

/// Runs an event on the [`CoreBattle`] for an applying effect.
///
/// Expects a [`MoveEventResult`].
pub fn run_event_for_applying_effect_expecting_move_event_result(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
) -> MoveEventResult {
    match run_event_for_applying_effect_internal(
        context,
        event,
        fxlang::VariableInput::default(),
        &RunCallbacksOptions::default(),
    ) {
        Some(result) => result.move_result().unwrap_or(MoveEventResult::Advance),
        None => MoveEventResult::Advance,
    }
}

/// Runs an event on the [`CoreBattle`] for an applying effect.
///
/// Expects a [`MonHandle`].
pub fn run_event_for_applying_effect_expecting_mon(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> Option<MonHandle> {
    run_event_for_applying_effect_internal(context, event, input, &RunCallbacksOptions::default())?
        .mon_handle()
        .ok()
}

/// Runs an event on the [`CoreBattle`] for an applying effect.
///
/// Expects a [`MonHandle`]. Returns the value of the first callback that returns a value.
pub fn run_event_for_applying_effect_expecting_mon_quick_return(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> Option<MonHandle> {
    run_event_for_applying_effect_internal(
        context,
        event,
        input,
        &RunCallbacksOptions {
            return_first_value: true,
        },
    )?
    .mon_handle()
    .ok()
}

/// Runs an event on the [`CoreBattle`] for an applying effect.
///
/// Expects a [`Vec<SecondaryEffect>`].
pub fn run_event_for_applying_effect_expecting_secondary_effects(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    secondary_effects: Vec<SecondaryEffectData>,
) -> Vec<SecondaryEffectData> {
    match run_event_for_applying_effect_internal(
        context,
        event,
        fxlang::VariableInput::from_iter([fxlang::Value::List(
            secondary_effects
                .iter()
                .map(|secondary_effect| fxlang::Value::SecondaryHitEffect(secondary_effect.clone()))
                .collect(),
        )]),
        &RunCallbacksOptions::default(),
    ) {
        Some(value) => value
            .secondary_hit_effects_list()
            .unwrap_or(secondary_effects),
        None => secondary_effects,
    }
}

/// Runs an event targeted on the given [`Mon`].
///
/// Returns `true` if all event handlers succeeded (i.e., did not return `false`).
pub fn run_event_for_mon(
    context: &mut MonContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> bool {
    run_event_for_mon_internal(context, event, input, &RunCallbacksOptions::default())
        .map(|value| value.boolean().ok())
        .flatten()
        .unwrap_or(true)
}

/// Runs an event targeted on the given [`Mon`].
///
/// Expects an integer that can fit in a [`u32`].
pub fn run_event_for_mon_expecting_u32(
    context: &mut MonContext,
    event: fxlang::BattleEvent,
    input: u32,
) -> u32 {
    match run_event_for_mon_internal(
        context,
        event,
        fxlang::VariableInput::from_iter([fxlang::Value::UFraction(input.into())]),
        &RunCallbacksOptions::default(),
    ) {
        Some(value) => value.integer_u32().unwrap_or(input),
        None => input,
    }
}

/// Runs an event targeted on the given [`Mon`].
///
/// Expects an integer that can fit in a [`u16`].
pub fn run_event_for_mon_expecting_u16(
    context: &mut MonContext,
    event: fxlang::BattleEvent,
    input: u16,
) -> u16 {
    match run_event_for_mon_internal(
        context,
        event,
        fxlang::VariableInput::from_iter([fxlang::Value::UFraction(input.into())]),
        &RunCallbacksOptions::default(),
    ) {
        Some(value) => value.integer_u16().unwrap_or(input),
        None => input,
    }
}

/// Runs an event targeted on the given [`Mon`].
///
/// Expects an integer that can fit in a [`u8`].
pub fn run_event_for_mon_expecting_u8(
    context: &mut MonContext,
    event: fxlang::BattleEvent,
    input: u8,
) -> u8 {
    match run_event_for_mon_internal(
        context,
        event,
        fxlang::VariableInput::from_iter([fxlang::Value::UFraction(input.into())]),
        &RunCallbacksOptions::default(),
    ) {
        Some(value) => value.integer_u8().unwrap_or(input),
        None => input,
    }
}

/// Runs an event targeted on the given [`Mon`].
///
/// Expects a [`String`].
pub fn run_event_for_mon_expecting_string(
    context: &mut MonContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> Option<String> {
    run_event_for_mon_internal(context, event, input, &RunCallbacksOptions::default())?
        .string()
        .ok()
}

/// Runs an event targeted on the given [`Mon`].
///
/// Expects a [`BoostTable`].
pub fn run_event_for_mon_expecting_boost_table(
    context: &mut MonContext,
    event: fxlang::BattleEvent,
    boost_table: BoostTable,
) -> BoostTable {
    match run_event_for_mon_internal(
        context,
        event,
        fxlang::VariableInput::from_iter([fxlang::Value::BoostTable(boost_table.clone())]),
        &RunCallbacksOptions::default(),
    ) {
        Some(value) => value.boost_table().unwrap_or(boost_table),
        None => boost_table,
    }
}

/// Runs an event targeted on the given [`Mon`].
///
/// Expects a [`StatTable`].
pub fn run_event_for_mon_expecting_stat_table(
    context: &mut MonContext,
    event: fxlang::BattleEvent,
    stat_table: StatTable,
) -> StatTable {
    match run_event_for_mon_internal(
        context,
        event,
        fxlang::VariableInput::from_iter([fxlang::Value::StatTable(stat_table.clone())]),
        &RunCallbacksOptions::default(),
    ) {
        Some(value) => value.stat_table().unwrap_or(stat_table),
        None => stat_table,
    }
}

/// Runs an event targeted on the given [`Mon`].
///
/// Expects a [`Vec<Type>`].
pub fn run_event_for_mon_expecting_types(
    context: &mut MonContext,
    event: fxlang::BattleEvent,
    types: Vec<Type>,
) -> Vec<Type> {
    match run_event_for_mon_internal(
        context,
        event,
        fxlang::VariableInput::from_iter([fxlang::Value::List(
            types.iter().map(|typ| fxlang::Value::Type(*typ)).collect(),
        )]),
        &RunCallbacksOptions::default(),
    ) {
        Some(value) => value.types_list().unwrap_or(types),
        None => types,
    }
}

/// Runs an event targeted on the given [`Mon`].
///
/// Expects a [`bool`]. Returns the value of the first callback that returns a value.
pub fn run_event_for_mon_expecting_bool_quick_return(
    context: &mut MonContext,
    event: fxlang::BattleEvent,
    default: bool,
) -> bool {
    run_event_for_mon_internal(
        context,
        event,
        fxlang::VariableInput::default(),
        &RunCallbacksOptions {
            return_first_value: true,
        },
    )
    .map(|value| value.boolean().ok())
    .flatten()
    .unwrap_or(default)
}

/// Runs an event targeted on the given [`Mon`].
///
/// Expects an integer that can fit in an [`i32`]. Returns the value of the first callback that
/// returns a value.
pub fn run_event_for_mon_expecting_i32_quick_return(
    context: &mut MonContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> Option<i32> {
    run_event_for_mon_internal(
        context,
        event,
        input,
        &RunCallbacksOptions {
            return_first_value: true,
        },
    )
    .map(|value| value.integer_i32().ok())
    .flatten()
}

/// Runs an event targeted on the given [`Mon`].
///
/// Expects a string list.
pub fn run_event_for_mon_expecting_string_list(
    context: &mut MonContext,
    event: fxlang::BattleEvent,
) -> Vec<String> {
    run_event_for_mon_internal(
        context,
        event,
        fxlang::VariableInput::from_iter([fxlang::Value::List(Vec::new())]),
        &RunCallbacksOptions::default(),
    )
    .map(|value| value.strings_list().ok())
    .flatten()
    .unwrap_or_default()
}

/// Runs an event on the [`CoreBattle`] for the residual effect, which
/// occurs at the end of every turn.
pub fn run_event_for_residual(context: &mut Context, event: fxlang::BattleEvent) {
    run_event_for_residual_internal(context, event)
}

/// Runs an event on the [`CoreBattle`] for a side-applying effect.
///
/// Returns `true` if all event handlers succeeded (i.e., did not return `false`).
pub fn run_event_for_side_effect(
    context: &mut SideEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> bool {
    run_event_for_side_effect_internal(context, event, input, &RunCallbacksOptions::default())
        .map(|value| value.boolean().ok())
        .flatten()
        .unwrap_or(true)
}

/// Runs an event on the [`CoreBattle`] for a side-applying effect.
///
/// Expects a [`MoveEventResult`].
pub fn run_event_for_side_effect_expecting_move_event_result(
    context: &mut SideEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> MoveEventResult {
    match run_event_for_side_effect_internal(context, event, input, &RunCallbacksOptions::default())
    {
        Some(value) => value.move_result().unwrap_or(MoveEventResult::Advance),
        None => MoveEventResult::Advance,
    }
}

/// Runs an event on the [`CoreBattle`] for a field-applying effect.
///
/// Returns `true` if all event handlers succeeded (i.e., did not return `false`).
pub fn run_event_for_field_effect(
    context: &mut FieldEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> bool {
    run_event_for_field_effect_internal(context, event, input, &RunCallbacksOptions::default())
        .map(|value| value.boolean().ok())
        .flatten()
        .unwrap_or(true)
}

/// Runs an event on the [`CoreBattle`] for a field-applying effect.
///
/// Expects a [`MoveEventResult`].
pub fn run_event_for_field_effect_expecting_move_event_result(
    context: &mut FieldEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> MoveEventResult {
    match run_event_for_field_effect_internal(
        context,
        event,
        input,
        &RunCallbacksOptions::default(),
    ) {
        Some(value) => value.move_result().unwrap_or(MoveEventResult::Advance),
        None => MoveEventResult::Advance,
    }
}

/// Runs an event on the [`CoreBattle`].
///
/// Expects a [`bool`]. Returns the value of the first callback that returns a value.
pub fn run_event_for_battle_expecting_bool_quick_return(
    context: &mut Context,
    event: fxlang::BattleEvent,
) -> bool {
    run_event_for_battle_internal(
        context,
        event,
        fxlang::VariableInput::default(),
        &RunCallbacksOptions {
            return_first_value: true,
        },
    )
    .map(|value| value.boolean().ok())
    .flatten()
    .unwrap_or(false)
}

/// Runs an event on the [`CoreBattle`] for each active [`Mon`], with a source effect.
///
/// Returns `true` if all event handlers succeeded (i.e., did not return `false`).
pub fn run_event_for_each_active_mon_with_effect(
    context: &mut EffectContext,
    event: fxlang::BattleEvent,
) -> Result<()> {
    for mon_handle in
        CoreBattle::all_active_mon_handles_in_speed_order(context.as_battle_context_mut())?
    {
        run_event_for_applying_effect(
            &mut context.applying_effect_context(None, mon_handle)?,
            event,
            fxlang::VariableInput::default(),
        );
    }
    Ok(())
}

/// Runs an event on the [`CoreBattle`] for each active [`Mon`].
///
/// Returns `true` if all event handlers succeeded (i.e., did not return `false`).
pub fn run_event_for_each_active_mon(
    context: &mut Context,
    event: fxlang::BattleEvent,
) -> Result<()> {
    for mon_handle in CoreBattle::all_active_mon_handles_in_speed_order(context)? {
        run_event_for_mon(
            &mut context.mon_context(mon_handle)?,
            event,
            fxlang::VariableInput::default(),
        );
    }
    Ok(())
}

/// Runs an event on the target [`Player`][`crate::battle::Player`].
///
/// Expects a string list.
pub fn run_event_for_player_expecting_string_list(
    context: &mut PlayerContext,
    event: fxlang::BattleEvent,
) -> Vec<String> {
    run_event_for_player_internal(
        context,
        event,
        fxlang::VariableInput::from_iter([fxlang::Value::List(Vec::new())]),
        &RunCallbacksOptions::default(),
    )
    .map(|value| value.strings_list().ok())
    .flatten()
    .unwrap_or_default()
}

/// Runs an event on the [`CoreBattle`] for a player-applying effect.
///
/// Returns `true` if all event handlers succeeded (i.e., did not return `false`).
pub fn run_event_for_player_effect(
    context: &mut PlayerEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> bool {
    run_event_for_player_effect_internal(context, event, input, &RunCallbacksOptions::default())
        .map(|value| value.boolean().ok())
        .flatten()
        .unwrap_or(true)
}
