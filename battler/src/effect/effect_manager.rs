use alloc::{
    format,
    string::String,
    sync::Arc,
};

use anyhow::{
    Error,
    Result,
};

use crate::{
    WrapResultError,
    battle::{
        Context,
        CoreBattle,
        MonHandle,
    },
    common::{
        LruCache,
        split_once_optional,
    },
    effect::{
        Effect,
        EffectHandle,
        fxlang::{
            BattleEvent,
            BattleEventModifier,
            DynamicEffectStateConnector,
            EvaluationContext,
            Evaluator,
            EventState,
            ParsedCallback,
            ParsedEffect,
            ParsedProgram,
            Program,
            ProgramEvalResult,
            ProgramMetadata,
            VariableInput,
        },
    },
    error::{
        WrapOptionError,
        general_error,
    },
};

enum EventCallbackMapping<'s> {
    Move(&'s str, &'s str),
    Swap(&'s str, &'s str),
}

impl<'s> TryFrom<&'s str> for EventCallbackMapping<'s> {
    type Error = Error;
    fn try_from(s: &'s str) -> Result<Self> {
        if let Some((from, to)) = s.split_once("<=>") {
            Ok(EventCallbackMapping::Swap(from, to))
        } else if let Some((from, to)) = s.split_once("=>") {
            Ok(EventCallbackMapping::Move(from, to))
        } else {
            Err(Error::msg("invalid event callback mapping"))
        }
    }
}

/// Module for managing fxlang effect programs and their evaluation.
pub struct EffectManager {
    effects: LruCache<String, Arc<ParsedEffect>>,
    stack: usize,
}

impl EffectManager {
    // 2 teams per battle, 6 Mons per team, 6 effects per Mon (4 moves + 1 ability + 1 item), 2
    // fxlang effects per effect (1 effect + 1 condition), plus an additional buffer.
    const MAX_SAVED_EFFECTS: usize = 2 * 6 * 6 * 2 + 16;
    const MAX_STACK_SIZE: usize = 10;

    /// Creates a new effect manager.
    pub fn new() -> Self {
        Self {
            effects: LruCache::new(Self::MAX_SAVED_EFFECTS),
            stack: 0,
        }
    }

    /// Evaluates the event callback for the given effect.
    pub fn evaluate(
        context: &mut EvaluationContext,
        effect_handle: &EffectHandle,
        event: BattleEvent,
        modifier: BattleEventModifier,
        input: VariableInput,
        event_state: &EventState,
        effect_state_connector: Option<DynamicEffectStateConnector>,
        effect_mon_handle: Option<MonHandle>,
    ) -> Result<ProgramEvalResult> {
        context
            .battle_context_mut()
            .battle_mut()
            .effect_manager
            .stack += 1;

        if context.battle_context().battle().effect_manager.stack > Self::MAX_STACK_SIZE {
            return Err(general_error(format!(
                "fxlang effect callback stack size exceeded for {event} callback of effect {:?}",
                effect_handle,
            )));
        }

        let result = Self::evaluate_internal(
            context,
            effect_handle,
            event,
            modifier,
            input,
            event_state,
            effect_state_connector,
            effect_mon_handle,
        );

        context
            .battle_context_mut()
            .battle_mut()
            .effect_manager
            .stack -= 1;

        result
    }

    pub fn parsed_effect(
        context: &mut Context,
        effect_handle: &EffectHandle,
    ) -> Result<Option<Arc<ParsedEffect>>> {
        let effect = match CoreBattle::get_effect_by_handle(context, effect_handle) {
            Ok(effect) => effect,
            Err(_) => return Ok(None),
        };

        // SAFETY: Effects are guaranteed to live at least through this turn. We only detach the
        // lifetime of this reference for looking up parsed callbacks in the EffectManager.
        let effect = unsafe { core::mem::transmute::<Effect<'_>, Effect<'_>>(effect) };

        let id = if effect.unlinked() {
            effect_handle.unlinked_fxlang_id().wrap_expectation_with_format(format_args!("unlinked effect {effect_handle:?} does not have an unlinked fxlang id for callback caching"))?
        } else {
            effect.fxlang_id()
        };

        // Callbacks are cached.
        //
        // Borrow checker is too strict to remove the extra lookup here.
        if context.battle().effect_manager.effects.contains_key(&id) {
            return Ok(context
                .battle_mut()
                .effect_manager
                .effects
                .get(&id)
                .cloned());
        }

        // Parse the effect's callbacks.
        let parsed_effect = match effect.fxlang_effect() {
            Some(fxlang_effect) => {
                let parsed_effect = ParsedEffect::new(
                    &fxlang_effect.callbacks,
                    fxlang_effect.attributes.condition.clone(),
                )?;
                let mut combined_effect = ParsedEffect::default();
                // If we are delegating to other effects, look them up and merge our callbacks in at
                // the end.
                for delegate in &fxlang_effect.attributes.delegates {
                    let (fxlang_id, mappings) = split_once_optional(delegate, ';');
                    let delegate_effect_handle = EffectHandle::from_fxlang_id(fxlang_id);

                    // NOTE: We don't protect against circular dependencies here.
                    let mut delegate_effect =
                        Self::parsed_effect(context, &delegate_effect_handle)?
                            .map(|effect| effect.as_ref().clone())
                            .unwrap_or_default();

                    for mapping in mappings.unwrap_or_default().split(';') {
                        if let Ok(mapping) = EventCallbackMapping::try_from(mapping) {
                            match mapping {
                                EventCallbackMapping::Move(from, to) => {
                                    let from = ParsedEffect::callback_name_to_event_key(from)
                                        .wrap_error_with_message("invalid from event")?;
                                    let to = ParsedEffect::callback_name_to_event_key(to)
                                        .wrap_error_with_message("invalid to event")?;
                                    if let Some(callback) =
                                        delegate_effect.take_event(from.0, from.1)
                                    {
                                        delegate_effect.set_event(to.0, to.1, callback);
                                    }
                                }
                                EventCallbackMapping::Swap(from, to) => {
                                    let from = ParsedEffect::callback_name_to_event_key(from)
                                        .wrap_error_with_message("invalid from event")?;
                                    let to = ParsedEffect::callback_name_to_event_key(to)
                                        .wrap_error_with_message("invalid to event")?;
                                    let from_callback = delegate_effect.take_event(from.0, from.1);
                                    let to_callback = delegate_effect.take_event(to.0, to.1);
                                    if let Some(from) = from_callback {
                                        delegate_effect.set_event(to.0, to.1, from);
                                    }
                                    if let Some(to) = to_callback {
                                        delegate_effect.set_event(from.0, from.1, to);
                                    }
                                }
                            }
                        }
                    }

                    combined_effect.extend(delegate_effect);
                }

                combined_effect.extend(parsed_effect);
                combined_effect
            }
            None => ParsedEffect::default(),
        };

        context
            .battle_mut()
            .effect_manager
            .effects
            .push(id.clone(), Arc::new(parsed_effect));

        Ok(context
            .battle_mut()
            .effect_manager
            .effects
            .get(&id)
            .cloned())
    }

    fn evaluate_internal(
        context: &mut EvaluationContext,
        effect_handle: &EffectHandle,
        event: BattleEvent,
        modifier: BattleEventModifier,
        input: VariableInput,
        event_state: &EventState,
        effect_state_connector: Option<DynamicEffectStateConnector>,
        effect_mon_handle: Option<MonHandle>,
    ) -> Result<ProgramEvalResult> {
        let mut evaluator = Evaluator::new(event, event_state);
        let effect = Self::parsed_effect(context.battle_context_mut(), effect_handle)?;
        match effect
            .as_ref()
            .map(|effect| effect.event(event, modifier))
            .flatten()
        {
            Some(callback) => evaluator.evaluate_program(
                context,
                input,
                callback,
                effect_state_connector,
                effect_mon_handle,
            ),
            None => Ok(ProgramEvalResult::new(None)),
        }
    }

    /// Evaluates a program from an outside effect.
    pub fn evaluate_outside_effect(
        context: &mut EvaluationContext,
        event: BattleEvent,
        program: &Program,
    ) -> Result<ProgramEvalResult> {
        let event_state = EventState::default();
        let mut evaluator = Evaluator::new(event, &event_state);
        evaluator.evaluate_program(
            context,
            VariableInput::default(),
            &ParsedCallback {
                program: ParsedProgram::from(program)
                    .wrap_error_with_format(format_args!("error parsing outside effect program"))?,
                order: 0,
                priority: 0,
                sub_order: 0,
                metadata: ProgramMetadata::default(),
            },
            None,
            None,
        )
    }
}
