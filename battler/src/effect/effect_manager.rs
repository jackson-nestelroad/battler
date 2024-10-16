use std::{
    mem,
    rc::Rc,
};

use crate::{
    battle::CoreBattle,
    common::LruCache,
    effect::{
        fxlang::{
            BattleEvent,
            DynamicEffectStateConnector,
            EvaluationContext,
            Evaluator,
            ParsedCallbacks,
            ProgramEvalResult,
            VariableInput,
        },
        Effect,
        EffectHandle,
    },
    error::{
        general_error,
        Error,
        WrapOptionError,
    },
};

/// Module for managing fxlang effect programs and their evaluation.
pub struct EffectManager {
    callbacks: LruCache<String, Rc<ParsedCallbacks>>,
    stack: usize,
}

impl EffectManager {
    const MAX_SAVED_EFFECTS: usize = 6 * 4 * 2 + 16;
    const MAX_STACK_SIZE: usize = 10;

    /// Creates a new effect manager.
    pub fn new() -> Self {
        Self {
            callbacks: LruCache::new(Self::MAX_SAVED_EFFECTS),
            stack: 0,
        }
    }

    /// Evaluates the event callback for the given effect.
    pub fn evaluate(
        context: &mut EvaluationContext,
        effect_handle: &EffectHandle,
        event: BattleEvent,
        input: VariableInput,
        effect_state_connector: Option<DynamicEffectStateConnector>,
    ) -> Result<ProgramEvalResult, Error> {
        let effect = match CoreBattle::get_effect_by_handle(context.battle_context(), effect_handle)
        {
            Ok(effect) => effect,
            Err(_) => return Ok(ProgramEvalResult::default()),
        };

        // SAFETY: Effects are guaranteed to live at least through this turn, and no effect is
        // allowed to change the turn of the battle.
        let effect: Effect = unsafe { mem::transmute(effect) };

        context
            .battle_context_mut()
            .battle_mut()
            .effect_manager
            .stack += 1;

        if context
            .battle_context_mut()
            .battle_mut()
            .effect_manager
            .stack
            > Self::MAX_STACK_SIZE
        {
            return Err(general_error(format!(
                "fxlang effect callback stack size exceeded for {event} callback of effect {}",
                effect.full_name(),
            )));
        }

        let result = Self::evaluate_internal(
            context,
            effect_handle,
            &effect,
            event,
            input,
            effect_state_connector,
        );

        context
            .battle_context_mut()
            .battle_mut()
            .effect_manager
            .stack -= 1;

        result
    }

    fn get_parsed_effect(
        &mut self,
        effect_handle: &EffectHandle,
        effect: &Effect,
    ) -> Result<Rc<ParsedCallbacks>, Error> {
        let id = if effect.unlinked() {
            effect_handle.unlinked_internal_fxlang_id().wrap_expectation_with_format(format_args!("unlinked effect {effect_handle:?} does not have an unlinked fxlang id for callback caching"))?
        } else {
            effect.internal_fxlang_id()
        };

        // TODO: Borrow checker is too strict to remove the extra lookup here.
        if self.callbacks.contains_key(&id) {
            return self
                .callbacks
                .get(&id)
                .cloned()
                .wrap_expectation_with_format(format_args!(
                    "callbacks cache contains {id} but lookup failed"
                ));
        }
        self.callbacks.push(
            id.clone(),
            Rc::new(ParsedCallbacks::from(
                effect.fxlang_effect().map(|effect| &effect.callbacks),
            )?),
        );
        self.callbacks
            .get(&id)
            .cloned()
            .wrap_expectation("pushing to effect cache failed, so parsed program was lost")
    }

    fn evaluate_internal(
        context: &mut EvaluationContext,
        effect_handle: &EffectHandle,
        effect: &Effect,
        event: BattleEvent,
        input: VariableInput,
        effect_state_connector: Option<DynamicEffectStateConnector>,
    ) -> Result<ProgramEvalResult, Error> {
        let mut evaluator = Evaluator::new();
        let callbacks = context
            .battle_context_mut()
            .battle_mut()
            .effect_manager
            .get_parsed_effect(&effect_handle, effect)?;
        match callbacks.event(event) {
            Some(program) => {
                evaluator.evaluate_program(context, event, input, program, effect_state_connector)
            }
            None => Ok(ProgramEvalResult::new(None)),
        }
    }
}
