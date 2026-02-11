use alloc::string::String;

use anyhow::Result;
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    WrapOptionError,
    battle::Context,
    effect::{
        EffectManager,
        fxlang::{
            self,
            EvaluationContext,
        },
    },
};

/// The target of an [`OutsideEffect`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum OutsideEffectTarget {
    #[serde(rename = "field")]
    Field,
    #[serde(rename = "side")]
    Side { index: usize },
    #[serde(rename = "player")]
    Player { id: String },
    #[serde(rename = "mon")]
    Mon { player: String, position: usize },
}

/// An outside effect that runs at the start of the next turn.
///
/// Allows an arbitrary effect callback to be run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutsideEffect {
    pub name: String,
    /// Target of the effect.
    pub target: OutsideEffectTarget,
    /// Source effect.
    pub source_effect: Option<String>,
    /// Program to execute.
    pub program: fxlang::Program,
}

/// Evaluates an outside effect on the battle.
pub fn evaluate_outside_effect(
    context: &mut Context,
    outside_effect: &OutsideEffect,
) -> Result<()> {
    let effect_handle = context
        .battle_mut()
        .get_effect_handle(&outside_effect.name)?
        .clone();
    let source_effect_handle = match &outside_effect.source_effect {
        Some(source_effect) => Some(
            context
                .battle_mut()
                .get_effect_handle(&source_effect)?
                .clone(),
        ),
        None => Some(effect_handle.clone()),
    };
    let mut context = context.effect_context(effect_handle, source_effect_handle)?;
    let (mut context, event) = match &outside_effect.target {
        OutsideEffectTarget::Field => (
            EvaluationContext::FieldEffect(context.field_effect_context(None)?),
            fxlang::BattleEvent::ActivateField,
        ),
        OutsideEffectTarget::Side { index } => (
            EvaluationContext::SideEffect(context.side_effect_context(*index, None)?),
            fxlang::BattleEvent::ActivateSide,
        ),
        OutsideEffectTarget::Player { id } => {
            let player = context.battle().player_index_by_id(id)?;
            (
                EvaluationContext::PlayerEffect(context.player_effect_context(player, None)?),
                fxlang::BattleEvent::ActivatePlayer,
            )
        }
        OutsideEffectTarget::Mon { player, position } => {
            let player = context.battle().player_index_by_id(player)?;
            let mon_handle = context
                .battle()
                .player(player)?
                .active_mon_handle(*position)
                .wrap_expectation_with_format(format_args!(
                    "no active mon in position {position}"
                ))?;
            (
                EvaluationContext::ApplyingEffect(
                    context.applying_effect_context(None, mon_handle)?,
                ),
                fxlang::BattleEvent::Activate,
            )
        }
    };
    EffectManager::evaluate_outside_effect(&mut context, event, &outside_effect.program)?;
    Ok(())
}
