use std::iter;

use ahash::HashSetExt;

use crate::{
    battle::{
        core_battle_actions,
        core_battle_logs,
        mon_states,
        ActiveMoveContext,
        ApplyingEffectContext,
        BoostTable,
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
        SideContext,
        SideEffectContext,
        SpeedOrderable,
    },
    common::{
        Error,
        FastHashSet,
        Id,
        MaybeOwnedMut,
        UnsafelyDetachBorrow,
        WrapResultError,
    },
    effect::{
        fxlang::{
            self,
            EffectStateConnector,
        },
        ActiveMoveEffectStateConnector,
        EffectHandle,
        EffectManager,
        MonAbilityEffectStateConnector,
        MonItemEffectStateConnector,
        MonStatusEffectStateConnector,
        MonVolatileStatusEffectStateConnector,
        SideConditionEffectStateConnector,
        WeatherEffectStateConnector,
    },
    mons::Type,
};

enum UpcomingEvaluationContext<
    'field_effect,
    'side_effect,
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
    'effect: 'side_effect,
    'effect: 'field_effect,
{
    ApplyingEffect(
        MaybeOwnedMut<'applying_effect, ApplyingEffectContext<'effect, 'context, 'battle, 'data>>,
    ),
    Effect(MaybeOwnedMut<'effect, EffectContext<'context, 'battle, 'data>>),
    Mon(MaybeOwnedMut<'mon, MonContext<'player, 'side, 'context, 'battle, 'data>>),
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
    input: fxlang::VariableInput,
    effect_state_connector: Option<fxlang::DynamicEffectStateConnector>,
) -> Result<fxlang::ProgramEvalResult, Error> {
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
    EffectManager::evaluate(
        &mut context,
        effect_handle,
        event,
        input,
        effect_state_connector,
    )
}

fn run_active_move_event_with_errors(
    context: &mut ActiveMoveContext,
    event: fxlang::BattleEvent,
    target: MoveTargetForEvent,
    input: fxlang::VariableInput,
) -> Result<Option<fxlang::Value>, Error> {
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
            input,
            Some(effect_state_connector),
        )?,
        MoveTargetForEvent::Side(side) => run_effect_event_with_errors(
            &mut UpcomingEvaluationContext::SideEffect(context.side_effect_context(side)?.into()),
            &effect_handle,
            event,
            input,
            Some(effect_state_connector),
        )?,
        MoveTargetForEvent::Field => run_effect_event_with_errors(
            &mut UpcomingEvaluationContext::FieldEffect(context.field_effect_context()?.into()),
            &effect_handle,
            event,
            input,
            Some(effect_state_connector),
        )?,
        MoveTargetForEvent::User => run_effect_event_with_errors(
            &mut UpcomingEvaluationContext::ApplyingEffect(
                context.user_applying_effect_context(None)?.into(),
            ),
            &effect_handle,
            event,
            input,
            Some(effect_state_connector),
        )?,
        MoveTargetForEvent::None => run_effect_event_with_errors(
            &mut UpcomingEvaluationContext::Effect(context.effect_context()?.into()),
            &effect_handle,
            event,
            input,
            Some(effect_state_connector),
        )?,
    };

    Ok(result.value)
}

fn run_active_move_event(
    context: &mut ActiveMoveContext,
    event: fxlang::BattleEvent,
    target: MoveTargetForEvent,
    input: fxlang::VariableInput,
) -> Option<fxlang::Value> {
    match run_active_move_event_with_errors(context, event, target, input) {
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

fn run_effect_event_by_handle(
    context: &mut UpcomingEvaluationContext,
    effect: &EffectHandle,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
    effect_state_connector: Option<fxlang::DynamicEffectStateConnector>,
) -> fxlang::ProgramEvalResult {
    match run_effect_event_with_errors(context, &effect, event, input, effect_state_connector) {
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
                &error.message(),
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
    /// The effect runs with respect to a single target of the move.
    Mon(MonHandle),
    /// The effect runs with respect to the target side of the move.
    Side(usize),
    /// The effect runs with respect to the field as a whole.
    Field,
}

/// The origin of an effect, which is important for reading and writing the
/// [`EffectState`][`fxlang::EffectState`] of the effect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum EffectOrigin {
    None,
    MonAbility(MonHandle),
    MonItem(MonHandle),
    MonStatus(MonHandle),
    MonType(MonHandle),
    MonVolatileStatus(MonHandle),
    SideCondition(usize),
    Weather,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

    /// Creates a dynamic connector for retrieving the effect state of the callback.
    pub fn effect_state_connector(&self) -> Option<fxlang::DynamicEffectStateConnector> {
        match self.origin {
            EffectOrigin::None => None,
            EffectOrigin::MonAbility(mon) => {
                Some(MonAbilityEffectStateConnector::new(mon).make_dynamic())
            }
            EffectOrigin::MonItem(mon) => {
                Some(MonItemEffectStateConnector::new(mon).make_dynamic())
            }
            EffectOrigin::MonStatus(mon) => {
                Some(MonStatusEffectStateConnector::new(mon).make_dynamic())
            }
            EffectOrigin::MonType(_) => None,
            EffectOrigin::MonVolatileStatus(mon) => self.effect_handle.try_id().map(|id| {
                MonVolatileStatusEffectStateConnector::new(mon, id.clone()).make_dynamic()
            }),
            EffectOrigin::SideCondition(side) => self
                .effect_handle
                .try_id()
                .map(|id| SideConditionEffectStateConnector::new(side, id.clone()).make_dynamic()),
            EffectOrigin::Weather => Some(WeatherEffectStateConnector::new().make_dynamic()),
        }
    }
}

fn run_callback_with_errors(
    mut context: UpcomingEvaluationContext,
    input: fxlang::VariableInput,
    callback_handle: CallbackHandle,
) -> Result<Option<fxlang::Value>, Error> {
    // Run the event callback for the event.
    let result = run_effect_event_by_handle(
        &mut context,
        &callback_handle.effect_handle,
        callback_handle.event,
        input.clone(),
        callback_handle.effect_state_connector(),
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
        callback_handle,
    )
    .ok()
    .flatten()
}

fn run_mon_status_event_internal(
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
        CallbackHandle::new(effect_handle, event, EffectOrigin::MonStatus(target_handle)),
    )
}

fn run_mon_volatile_event_internal(
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
            EffectOrigin::MonVolatileStatus(target_handle),
        ),
    )
}

fn run_mon_ability_event_internal(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> Option<fxlang::Value> {
    let ability = context.target().ability.id.clone();
    let effect_handle = context
        .battle_mut()
        .get_effect_handle_by_id(&ability)
        .ok()?
        .clone();
    let target_handle = context.target_handle();
    run_callback_under_applying_effect(
        context,
        input,
        CallbackHandle::new(
            effect_handle,
            event,
            EffectOrigin::MonAbility(target_handle),
        ),
    )
}

fn run_mon_item_event_internal(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> Option<fxlang::Value> {
    let item = context.target().item.as_ref().map(|item| item.id.clone())?;
    let effect_handle = context
        .battle_mut()
        .get_effect_handle_by_id(&item)
        .ok()?
        .clone();
    let target_handle = context.target_handle();
    run_callback_under_applying_effect(
        context,
        input,
        CallbackHandle::new(effect_handle, event, EffectOrigin::MonItem(target_handle)),
    )
}

fn run_side_condition_event_internal(
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
            EffectOrigin::SideCondition(side_index),
        ),
    )
}

fn run_weather_event_internal(
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
        CallbackHandle::new(effect_handle, event, EffectOrigin::Weather),
    )
}

fn run_applying_effect_event_internal(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> Option<fxlang::Value> {
    let effect_handle = context.effect_handle().clone();
    run_callback_under_applying_effect(
        context,
        input,
        CallbackHandle::new(effect_handle, event, EffectOrigin::None),
    )
}

fn run_effect_event_internal(
    context: &mut EffectContext,
    event: fxlang::BattleEvent,
    input: fxlang::VariableInput,
) -> Option<fxlang::Value> {
    let effect_handle = context.effect_handle().clone();
    run_callback_under_effect(
        context,
        input,
        CallbackHandle::new(effect_handle, event, EffectOrigin::None),
    )
}

fn find_callbacks_on_mon(
    context: &mut Context,
    event: fxlang::BattleEvent,
    mon: MonHandle,
    include_applied_field_effects: bool,
) -> Result<FastHashSet<CallbackHandle>, Error> {
    let mut callbacks = FastHashSet::new();
    let mut context = context.mon_context(mon)?;

    if event.callback_lookup_layer() > fxlang::BattleEvent::Types.callback_lookup_layer() {
        let types = Mon::types(&mut context)?;
        for typ in types {
            callbacks.insert(CallbackHandle::new(
                EffectHandle::Condition(typ.id()),
                event,
                EffectOrigin::MonType(mon),
            ));
        }
    }

    if let Some(status) = context.mon().status.clone() {
        let status_effect_handle = context.battle_mut().get_effect_handle_by_id(&status)?;
        callbacks.insert(CallbackHandle::new(
            status_effect_handle.clone(),
            event,
            EffectOrigin::MonStatus(mon),
        ));
    }
    for volatile in context.mon().volatiles.clone().keys() {
        let status_effect_handle = context.battle_mut().get_effect_handle_by_id(&volatile)?;
        callbacks.insert(CallbackHandle::new(
            status_effect_handle.clone(),
            event,
            EffectOrigin::MonVolatileStatus(mon),
        ));
    }

    if let Some(ability) = mon_states::effective_ability(&mut context) {
        callbacks.insert(CallbackHandle::new(
            EffectHandle::Ability(ability),
            event,
            EffectOrigin::MonAbility(mon),
        ));
    }

    if event.callback_lookup_layer() > fxlang::BattleEvent::SuppressMonItem.callback_lookup_layer()
    {
        if let Some(item) = mon_states::effective_item(&mut context) {
            callbacks.insert(CallbackHandle::new(
                EffectHandle::Item(item),
                event,
                EffectOrigin::MonItem(mon),
            ));
        }
    }

    // TODO: Species.
    // TODO: Slot conditions on the side.

    if include_applied_field_effects {
        if event.callback_lookup_layer()
            > fxlang::BattleEvent::SuppressMonWeather.callback_lookup_layer()
        {
            if let Some(weather) = mon_states::effective_weather(&mut context) {
                let weather_handle = context.battle_mut().get_effect_handle_by_id(&weather)?;
                callbacks.insert(CallbackHandle::new(
                    weather_handle.clone(),
                    event,
                    EffectOrigin::Weather,
                ));
            }
        }
    }

    Ok(callbacks)
}

fn find_callbacks_on_side(
    context: &mut Context,
    event: fxlang::BattleEvent,
    side: usize,
) -> Result<FastHashSet<CallbackHandle>, Error> {
    let mut callbacks = FastHashSet::new();
    let mut context = context.side_context(side)?;

    for side_condition in context.side().conditions.clone().keys() {
        let side_condition_handle = context
            .battle_mut()
            .get_effect_handle_by_id(&side_condition)?;
        callbacks.insert(CallbackHandle::new(
            side_condition_handle.clone(),
            event,
            EffectOrigin::SideCondition(side),
        ));
    }

    Ok(callbacks)
}

fn find_callbacks_on_field(
    context: &mut Context,
    event: fxlang::BattleEvent,
) -> Result<FastHashSet<CallbackHandle>, Error> {
    let mut callbacks = FastHashSet::new();

    if event.callback_lookup_layer()
        > fxlang::BattleEvent::SuppressFieldWeather.callback_lookup_layer()
    {
        if let Some(weather) = Field::effective_weather(context) {
            let weather_handle = context.battle_mut().get_effect_handle_by_id(&weather)?;
            callbacks.insert(CallbackHandle::new(
                weather_handle.clone(),
                event,
                EffectOrigin::Weather,
            ));
        }
    }

    // TODO: Pseudo-weather.
    // TODO: Terrain.

    Ok(callbacks)
}

#[derive(Clone, Copy)]
enum AllEffectsTarget {
    Mon(MonHandle),
    Side(usize),
    Field,
    Residual,
}

fn find_all_callbacks(
    context: &mut Context,
    event: fxlang::BattleEvent,
    target: AllEffectsTarget,
    source: Option<MonHandle>,
) -> Result<FastHashSet<CallbackHandle>, Error> {
    let mut callbacks = FastHashSet::new();

    match target {
        AllEffectsTarget::Mon(mon) => {
            callbacks.extend(find_callbacks_on_mon(context, event, mon, true)?);
            let mut context = context.mon_context(mon)?;
            for mon in Mon::active_allies_and_self(&mut context).collect::<Vec<_>>() {
                if let Some(ally_event) = event.ally_event() {
                    callbacks.extend(find_callbacks_on_mon(
                        context.as_battle_context_mut(),
                        ally_event,
                        mon,
                        true,
                    )?);
                }
                if let Some(any_event) = event.any_event() {
                    callbacks.extend(find_callbacks_on_mon(
                        context.as_battle_context_mut(),
                        any_event,
                        mon,
                        true,
                    )?);
                }
            }
            for mon in Mon::active_foes(&mut context).collect::<Vec<_>>() {
                if let Some(foe_event) = event.foe_event() {
                    callbacks.extend(find_callbacks_on_mon(
                        context.as_battle_context_mut(),
                        foe_event,
                        mon,
                        true,
                    )?);
                }
                if let Some(any_event) = event.any_event() {
                    callbacks.extend(find_callbacks_on_mon(
                        context.as_battle_context_mut(),
                        any_event,
                        mon,
                        true,
                    )?);
                }
            }
            let side = context.side().index;
            callbacks.extend(find_callbacks_on_side(
                context.as_battle_context_mut(),
                event,
                side,
            )?);
            if let Some(foe_event) = event.foe_event() {
                let foe_side = context.foe_side().index;
                callbacks.extend(find_callbacks_on_side(
                    context.as_battle_context_mut(),
                    foe_event,
                    foe_side,
                )?);
            }

            callbacks.extend(find_callbacks_on_field(
                context.as_battle_context_mut(),
                event,
            )?);
        }
        AllEffectsTarget::Side(side) => {
            callbacks.extend(find_callbacks_on_side(context, event, side)?);
            let mut context = context.side_context(side)?;
            if let Some(foe_event) = event.foe_event() {
                let foe_side = context.foe_side().index;
                callbacks.extend(find_callbacks_on_side(
                    context.as_battle_context_mut(),
                    foe_event,
                    foe_side,
                )?);
            }

            callbacks.extend(find_callbacks_on_field(
                context.as_battle_context_mut(),
                event,
            )?);
        }
        AllEffectsTarget::Field => {
            for mon in context
                .battle()
                .all_active_mon_handles()
                .collect::<Vec<_>>()
            {
                callbacks.extend(find_callbacks_on_mon(context, event, mon, true)?);
            }
            for side in context.battle().side_indices() {
                callbacks.extend(find_callbacks_on_side(context, event, side)?);
            }
            callbacks.extend(find_callbacks_on_field(context, event)?);
        }
        AllEffectsTarget::Residual => {
            for mon in context
                .battle()
                .all_active_mon_handles()
                .collect::<Vec<_>>()
            {
                callbacks.extend(find_callbacks_on_mon(context, event, mon, false)?);
            }
            for side in context.battle().side_indices() {
                if let Some(side_event) = event.side_event() {
                    callbacks.extend(find_callbacks_on_side(context, side_event, side)?);
                }
            }
            if let Some(field_event) = event.field_event() {
                callbacks.extend(find_callbacks_on_field(context, field_event)?);
            }
        }
    }

    if let Some(source) = source {
        if let Some(source_event) = event.source_event() {
            callbacks.extend(find_callbacks_on_mon(context, source_event, source, true)?);
            let side = context.mon(source)?.side;
            callbacks.extend(find_callbacks_on_side(context, source_event, side)?);
        }
    }

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
    let effect = CoreBattle::get_effect_by_handle(context, &callback_handle.effect_handle)
        .wrap_error_with_format(format_args!(
            "effect {:?} not found",
            callback_handle.effect_handle
        ))?;
    let callback = match effect.fxlang_effect() {
        Some(effect) => match effect.callbacks.event(callback_handle.event) {
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
    callback_handle: CallbackHandle,
    options: &RunCallbacksOptions,
) -> Result<Option<fxlang::Value>, Error> {
    let value = run_callback_with_errors(context, input.clone(), callback_handle)?;
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
) -> Result<Option<fxlang::Value>, Error> {
    for callback_handle in callbacks {
        let result = match source_effect {
            Some(source_effect) => run_callbacks_with_forwarding_input_with_errors(
                UpcomingEvaluationContext::ApplyingEffect(
                    context
                        .applying_effect_context(source_effect.clone(), source, None)?
                        .into(),
                ),
                &mut input,
                callback_handle,
                options,
            )?,
            None => run_callbacks_with_forwarding_input_with_errors(
                UpcomingEvaluationContext::Mon(context.into()),
                &mut input,
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
) -> Result<Option<fxlang::Value>, Error> {
    for callback_handle in callbacks {
        let result = match source_effect {
            Some(source_effect) => run_callbacks_with_forwarding_input_with_errors(
                UpcomingEvaluationContext::SideEffect(
                    context
                        .side_effect_context(source_effect.clone(), source, None)?
                        .into(),
                ),
                &mut input,
                callback_handle,
                options,
            )?,
            None => run_callbacks_with_forwarding_input_with_errors(
                UpcomingEvaluationContext::Side(context.into()),
                &mut input,
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
) -> Result<Option<fxlang::Value>, Error> {
    for callback_handle in callbacks {
        let result = match source_effect {
            Some(source_effect) => run_callbacks_with_forwarding_input_with_errors(
                UpcomingEvaluationContext::FieldEffect(
                    context
                        .field_effect_context(source_effect.clone(), source, None)?
                        .into(),
                ),
                &mut input,
                callback_handle,
                options,
            )?,
            None => run_callbacks_with_forwarding_input_with_errors(
                UpcomingEvaluationContext::Field(context.into()),
                &mut input,
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
) -> Result<(), Error> {
    for callback_handle in callbacks {
        if context.battle().ending() {
            break;
        }

        let mut context = context.effect_context(callback_handle.effect_handle.clone(), None)?;

        let mut ended = false;
        if let Some(effect_state_connector) = callback_handle.effect_state_connector() {
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

        match callback_handle.origin {
            EffectOrigin::None => {
                run_callback_with_errors(
                    UpcomingEvaluationContext::Effect(context.into()),
                    fxlang::VariableInput::default(),
                    callback_handle,
                )?;
            }
            EffectOrigin::MonAbility(mon) => {
                let context = context.applying_effect_context(None, mon)?;
                run_callback_with_errors(
                    UpcomingEvaluationContext::ApplyingEffect(context.into()),
                    fxlang::VariableInput::default(),
                    callback_handle,
                )?;
            }
            EffectOrigin::MonItem(mon) => {
                let context = context.applying_effect_context(None, mon)?;
                run_callback_with_errors(
                    UpcomingEvaluationContext::ApplyingEffect(context.into()),
                    fxlang::VariableInput::default(),
                    callback_handle,
                )?;
            }
            EffectOrigin::MonStatus(mon) => {
                let mut context = context.applying_effect_context(None, mon)?;
                if ended {
                    core_battle_actions::clear_status(&mut context, false)?;
                } else {
                    run_callback_with_errors(
                        UpcomingEvaluationContext::ApplyingEffect(context.into()),
                        fxlang::VariableInput::default(),
                        callback_handle,
                    )?;
                }
            }
            EffectOrigin::MonType(mon) => {
                let context = context.applying_effect_context(None, mon)?;
                run_callback_with_errors(
                    UpcomingEvaluationContext::ApplyingEffect(context.into()),
                    fxlang::VariableInput::default(),
                    callback_handle,
                )?;
            }
            EffectOrigin::MonVolatileStatus(mon) => {
                let mut context = context.applying_effect_context(None, mon)?;
                if ended {
                    core_battle_actions::remove_volatile(
                        &mut context,
                        callback_handle
                            .effect_handle
                            .try_id()
                            .wrap_error_with_message("expected volatile to have an id")?,
                        false,
                    )?;
                } else {
                    run_callback_with_errors(
                        UpcomingEvaluationContext::ApplyingEffect(context.into()),
                        fxlang::VariableInput::default(),
                        callback_handle,
                    )?;
                }
            }
            EffectOrigin::SideCondition(side) => {
                let mut context = context.side_effect_context(side, None)?;
                if ended {
                    core_battle_actions::remove_side_condition(
                        &mut context,
                        callback_handle
                            .effect_handle
                            .try_id()
                            .wrap_error_with_message("expected side condition to have an id")?,
                    )?;
                } else {
                    run_callback_with_errors(
                        UpcomingEvaluationContext::SideEffect(context.into()),
                        fxlang::VariableInput::default(),
                        callback_handle,
                    )?;
                }
            }
            EffectOrigin::Weather => {
                let mut context = context.field_effect_context(None)?;
                if ended {
                    core_battle_actions::clear_weather(&mut context)?;
                } else {
                    run_callback_with_errors(
                        UpcomingEvaluationContext::FieldEffect(context.into()),
                        fxlang::VariableInput::default(),
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
    source_effect: Option<&EffectHandle>,
    target: AllEffectsTarget,
    source: Option<MonHandle>,
    input: fxlang::VariableInput,
    options: &RunCallbacksOptions,
) -> Result<Option<fxlang::Value>, Error> {
    let callbacks = find_all_callbacks(context, event, target, source)?;
    let callbacks = Vec::from_iter(callbacks.into_iter());
    let callbacks = get_ordered_effects_for_event(context, callbacks)?;

    match target {
        AllEffectsTarget::Mon(mon) => run_mon_callbacks_with_errors(
            &mut context.mon_context(mon)?,
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
                &error.message(),
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
                &error.message(),
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
                &error.message(),
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
                &error.message(),
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
            core_battle_logs::debug_full_event_failure(context, event, &error.message());
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
            core_battle_logs::debug_full_event_failure(context, event, &error.message());
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
    input: fxlang::VariableInput,
) -> Option<u32> {
    run_active_move_event(context, event, target, input)?
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
    run_mon_status_event_internal(context, event, fxlang::VariableInput::default())?
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
    run_mon_status_event_internal(context, event, fxlang::VariableInput::default())?
        .boolean()
        .ok()
}

/// Runs an event on the target [`Mon`]'s volatile status.
pub fn run_mon_volatile_event(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    status: &Id,
) {
    run_mon_volatile_event_internal(context, event, fxlang::VariableInput::default(), status);
}

/// Runs an event on the target [`Mon`]'s volatile status.
///
/// Expects a [`bool`].
pub fn run_mon_volatile_event_expecting_bool(
    context: &mut ApplyingEffectContext,
    event: fxlang::BattleEvent,
    status: &Id,
) -> Option<bool> {
    run_mon_volatile_event_internal(context, event, fxlang::VariableInput::default(), status)?
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
    run_mon_volatile_event_internal(context, event, fxlang::VariableInput::default(), status)?
        .integer_u8()
        .ok()
}

/// Runs an event on the target [`Mon`]'s current ability.
pub fn run_mon_ability_event(context: &mut ApplyingEffectContext, event: fxlang::BattleEvent) {
    run_mon_ability_event_internal(context, event, fxlang::VariableInput::default());
}

/// Runs an event on the target [`Mon`]'s current itemt.
pub fn run_mon_item_event(context: &mut ApplyingEffectContext, event: fxlang::BattleEvent) {
    run_mon_item_event_internal(context, event, fxlang::VariableInput::default());
}

/// Runs an event on the target [`Side`][`crate::battle::Side`]'s side condition.
pub fn run_side_condition_event(
    context: &mut SideEffectContext,
    event: fxlang::BattleEvent,
    condition: &Id,
) {
    run_side_condition_event_internal(context, event, fxlang::VariableInput::default(), condition);
}

/// Runs an event on the target [`Side`][`crate::battle::Side`]'s side condition.
///
/// Expects a [`bool`].
pub fn run_side_condition_event_expecting_bool(
    context: &mut SideEffectContext,
    event: fxlang::BattleEvent,
    condition: &Id,
) -> Option<bool> {
    run_side_condition_event_internal(context, event, fxlang::VariableInput::default(), condition)?
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
    run_side_condition_event_internal(context, event, fxlang::VariableInput::default(), condition)?
        .integer_u8()
        .ok()
}

/// Runs an event on the [`Field`][`crate::battle::Field`]'s current weather.
pub fn run_weather_event(context: &mut FieldEffectContext, event: fxlang::BattleEvent) {
    run_weather_event_internal(context, event, fxlang::VariableInput::default());
}

/// Runs an event on the [`Field`][`crate::battle::Field`]'s weather.
///
/// Expects a [`bool`].
pub fn run_weather_event_expecting_bool(
    context: &mut FieldEffectContext,
    event: fxlang::BattleEvent,
) -> Option<bool> {
    run_weather_event_internal(context, event, fxlang::VariableInput::default())?
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
    run_weather_event_internal(context, event, fxlang::VariableInput::default())?
        .integer_u8()
        .ok()
}

/// Runs an event on the applying [`Effect`][`crate::effect::Effect`].
pub fn run_applying_effect_event(context: &mut ApplyingEffectContext, event: fxlang::BattleEvent) {
    run_applying_effect_event_internal(context, event, fxlang::VariableInput::default());
}

/// Runs an event on the [`Effect`][`crate::effect::Effect`].
///
/// Expects a [`bool`].
pub fn run_effect_event_expecting_bool(
    context: &mut EffectContext,
    event: fxlang::BattleEvent,
) -> Option<bool> {
    run_effect_event_internal(context, event, fxlang::VariableInput::default())?
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
    .unwrap_or(false)
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
/// Expects an integer that can fit in a [`i8`].
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
/// Exepcts a [`MonHandle`]. Returns the value of the first callback that returns a value.
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
) -> Option<String> {
    run_event_for_mon_internal(
        context,
        event,
        fxlang::VariableInput::default(),
        &RunCallbacksOptions::default(),
    )?
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
    .unwrap_or(false)
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

/// Runs an event on the [`CoreBattle`] for each active [`Mon`].
///
/// Returns `true` if all event handlers succeeded (i.e., did not return `false`).
pub fn run_event_for_each_active_mon(
    context: &mut EffectContext,
    event: fxlang::BattleEvent,
) -> Result<(), Error> {
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
