use std::{
    mem,
    sync::Arc,
};

use anyhow::Result;

use crate::{
    battle::{
        Context,
        CoreBattle,
    },
    common::LruCache,
    effect::{
        Effect,
        EffectHandle,
        fxlang::{
            BattleEvent,
            DynamicEffectStateConnector,
            EvaluationContext,
            Evaluator,
            ParsedEffect,
            ProgramEvalResult,
            VariableInput,
        },
    },
    error::{
        WrapOptionError,
        general_error,
    },
};
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
        input: VariableInput,
        effect_state_connector: Option<DynamicEffectStateConnector>,
    ) -> Result<ProgramEvalResult> {
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
                "fxlang effect callback stack size exceeded for {event} callback of effect {:?}",
                effect_handle,
            )));
        }

        let result =
            Self::evaluate_internal(context, effect_handle, event, input, effect_state_connector);

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
        let effect: Effect = unsafe { mem::transmute(effect) };

        let id = if effect.unlinked() {
            effect_handle.unlinked_fxlang_id().wrap_expectation_with_format(format_args!("unlinked effect {effect_handle:?} does not have an unlinked fxlang id for callback caching"))?
        } else {
            effect.fxlang_id()
        };

        // Callbacks are cached.
        //
        // TODO: Borrow checker is too strict to remove the extra lookup here.
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
                    let delegate_effect_handle = EffectHandle::from_fxlang_id(delegate);

                    // NOTE: We don't protect against circular dependencies here.
                    let delegate_effect = Self::parsed_effect(context, &delegate_effect_handle)?
                        .map(|effect| effect.as_ref().clone())
                        .unwrap_or_default();

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
        input: VariableInput,
        effect_state_connector: Option<DynamicEffectStateConnector>,
    ) -> Result<ProgramEvalResult> {
        let mut evaluator = Evaluator::new(event);
        let effect = Self::parsed_effect(context.battle_context_mut(), effect_handle)?;
        match effect.as_ref().map(|effect| effect.event(event)).flatten() {
            Some(callback) => {
                evaluator.evaluate_program(context, input, callback, effect_state_connector)
            }
            None => Ok(ProgramEvalResult::new(None)),
        }
    }
}
