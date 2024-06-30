use std::{
    mem,
    rc::Rc,
};

use crate::{
    battle::CoreBattle,
    common::{
        Error,
        LruCache,
        WrapResultError,
    },
    effect::{
        fxlang::{
            BattleEvent,
            EffectState,
            EvaluationContext,
            Evaluator,
            ParsedCallbacks,
            ProgramEvalResult,
            VariableInput,
        },
        Effect,
        EffectHandle,
    },
};

/// Module for managing fxlang effect programs and their evaluation.
pub struct EffectManager {
    callbacks: LruCache<String, Rc<ParsedCallbacks>>,
}

impl EffectManager {
    const MAX_SAVED_EFFECTS: usize = 6 * 4 * 2 + 16;

    pub fn new() -> Self {
        Self {
            callbacks: LruCache::new(Self::MAX_SAVED_EFFECTS),
        }
    }

    pub fn evaluate(
        context: &mut EvaluationContext,
        effect_handle: &EffectHandle,
        event: BattleEvent,
        input: VariableInput,
        effect_state: Option<EffectState>,
    ) -> Result<ProgramEvalResult, Error> {
        let effect = match CoreBattle::get_effect_by_handle(context.battle_context(), effect_handle)
        {
            Ok(effect) => effect,
            Err(_) => return Ok(ProgramEvalResult::default()),
        };
        // SAFETY: Effects are guaranteed to live at least through this turn, and no effect is
        // allowed to change the turn of the battle.
        let effect = unsafe { mem::transmute(effect) };
        Self::evaluate_internal(context, &effect, event, input, effect_state)
    }

    fn get_parsed_effect(&mut self, effect: &Effect) -> Result<Rc<ParsedCallbacks>, Error> {
        let id = effect.internal_fxlang_id();
        // TODO: Borrow checker is too strict to remove the extra lookup here.
        if self.callbacks.contains_key(&id) {
            return self
                .callbacks
                .get(&id)
                .cloned()
                .wrap_error_with_format(format_args!(
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
            .wrap_error_with_message("pushing to effect cache failed, so parsed program was lost")
    }

    fn evaluate_internal(
        context: &mut EvaluationContext,
        effect: &Effect,
        event: BattleEvent,
        input: VariableInput,
        effect_state: Option<EffectState>,
    ) -> Result<ProgramEvalResult, Error> {
        let mut evaluator = Evaluator::new();
        let callbacks = context
            .battle_context_mut()
            .battle_mut()
            .effect_manager
            .get_parsed_effect(effect)?;
        match callbacks.event(event) {
            Some(program) => {
                evaluator.evaluate_program(context, event, input, effect_state, program)
            }
            None => Ok(ProgramEvalResult::new(None, effect_state)),
        }
    }
}
