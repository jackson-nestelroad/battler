use std::{
    cell::RefMut,
    mem,
};

use crate::{
    battle::{
        ActiveMoveContext,
        ApplyingEffectContext,
        MonContext,
    },
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
    },
};

/// Module for managing fxlang effect programs and their evaluation.
pub struct EffectManager {
    callbacks: LruCache<String, ParsedCallbacks>,
}

impl EffectManager {
    const MAX_SAVED_EFFECTS: usize = 6 * 4 * 2 + 16;

    pub fn new() -> Self {
        Self {
            callbacks: LruCache::new(Self::MAX_SAVED_EFFECTS),
        }
    }

    pub fn evaluate(
        context: EvaluationContext,
        effect: &Effect,
        event: BattleEvent,
        input: VariableInput,
        effect_state: Option<EffectState>,
    ) -> Result<ProgramEvalResult, Error> {
        Self::evaluate_internal(context, effect, event, input, effect_state)
    }

    fn get_parsed_effect(&mut self, effect: &Effect) -> Result<&ParsedCallbacks, Error> {
        let id = effect.internal_fxlang_id();
        // TODO: Borrow checker is too strict to remove the extra lookup here.
        if self.callbacks.contains_key(&id) {
            return self.callbacks.get(&id).wrap_error_with_format(format_args!(
                "callbacks cache contains {id} but lookup failed"
            ));
        }
        self.callbacks.push(
            id.clone(),
            ParsedCallbacks::from(effect.fxlang_callbacks())?,
        );
        self.callbacks
            .get(&id)
            .wrap_error_with_message("pushing to effect cache failed, so parsed program was lost")
    }

    fn evaluate_internal(
        mut context: EvaluationContext,
        effect: &Effect,
        event: BattleEvent,
        input: VariableInput,
        effect_state: Option<EffectState>,
    ) -> Result<ProgramEvalResult, Error> {
        let mut evaluator = Evaluator::new();
        let effect_manager = context
            .battle_context_mut()
            .battle_mut()
            .effect_manager
            .try_borrow_mut()
            .wrap_error_with_message("failed to borrow effect manager")?;
        // SAFETY: We use `RefCell` to wrap the `EffectManager` for the battle. This ensures that
        // only one mutable borrow can occur, and any other mutable borrow will produce an error. We
        // use this so that we can get the parsed effect while still doing everything else we
        // normally would with the battle context inside of the evaluator.
        //
        // However, we have to borrow the whole `context` object to get to this dynamically-checked
        // mutable memory location. Thus, to continue forward, we disconnect this `RefMut` from its
        // owner.
        let mut effect_manager: RefMut<Self> = unsafe { mem::transmute(effect_manager) };
        let callbacks = effect_manager.get_parsed_effect(effect)?;
        match callbacks.event(event) {
            Some(program) => {
                evaluator.evaluate_program(context, event, input, effect_state, program)
            }
            None => Ok(ProgramEvalResult::new(None, effect_state)),
        }
    }
}
