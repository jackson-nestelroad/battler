use anyhow::Result;

use crate::{
    WrapOptionError,
    battle::{
        ActiveMoveContext,
        ApplyingEffectContext,
        Context,
        EffectContext,
        FieldEffectContext,
        Mon,
        MonContext,
        MonHandle,
        MoveHandle,
        PlayerEffectContext,
        SideEffectContext,
    },
    common::MaybeOwnedMut,
    effect::EffectHandle,
    general_error,
    moves::{
        Move,
        MoveHitEffectType,
    },
};

/// The [`Context`][`crate::battle::Context`] in which an fxlang program is evaluated.
pub enum EvaluationContext<'effect, 'context, 'battle, 'data>
where
    'data: 'battle,
    'battle: 'context,
    'context: 'effect,
{
    ApplyingEffect(ApplyingEffectContext<'effect, 'context, 'battle, 'data>),
    Effect(EffectContext<'context, 'battle, 'data>),
    PlayerEffect(PlayerEffectContext<'effect, 'context, 'battle, 'data>),
    SideEffect(SideEffectContext<'effect, 'context, 'battle, 'data>),
    FieldEffect(FieldEffectContext<'effect, 'context, 'battle, 'data>),
}

impl<'effect, 'context, 'battle, 'data> EvaluationContext<'effect, 'context, 'battle, 'data> {
    pub fn battle_context<'eval>(&'eval self) -> &'eval Context<'battle, 'data> {
        match self {
            Self::ApplyingEffect(context) => context.as_battle_context(),
            Self::Effect(context) => context.as_battle_context(),
            Self::PlayerEffect(context) => context.as_battle_context(),
            Self::SideEffect(context) => context.as_battle_context(),
            Self::FieldEffect(context) => context.as_battle_context(),
        }
    }

    pub fn battle_context_mut<'eval>(&'eval mut self) -> &'eval mut Context<'battle, 'data> {
        match self {
            Self::ApplyingEffect(context) => context.as_battle_context_mut(),
            Self::Effect(context) => context.as_battle_context_mut(),
            Self::PlayerEffect(context) => context.as_battle_context_mut(),
            Self::SideEffect(context) => context.as_battle_context_mut(),
            Self::FieldEffect(context) => context.as_battle_context_mut(),
        }
    }

    pub fn effect_context<'eval>(&'eval self) -> &'eval EffectContext<'context, 'battle, 'data> {
        match self {
            Self::ApplyingEffect(context) => context.as_effect_context(),
            Self::Effect(context) => context,
            Self::PlayerEffect(context) => context.as_effect_context(),
            Self::SideEffect(context) => context.as_effect_context(),
            Self::FieldEffect(context) => context.as_effect_context(),
        }
    }

    pub fn effect_context_mut<'eval>(
        &'eval mut self,
    ) -> &'eval mut EffectContext<'context, 'battle, 'data> {
        match self {
            Self::ApplyingEffect(context) => context.as_effect_context_mut(),
            Self::Effect(context) => context,
            Self::PlayerEffect(context) => context.as_effect_context_mut(),
            Self::SideEffect(context) => context.as_effect_context_mut(),
            Self::FieldEffect(context) => context.as_effect_context_mut(),
        }
    }

    pub fn source_effect_context<'eval>(
        &'eval mut self,
    ) -> Result<Option<EffectContext<'eval, 'battle, 'data>>> {
        self.effect_context_mut().source_effect_context()
    }

    pub fn applying_effect_context<'eval>(
        &'eval self,
    ) -> Result<&'eval ApplyingEffectContext<'effect, 'context, 'battle, 'data>> {
        match self {
            Self::ApplyingEffect(context) => Ok(context),
            _ => Err(general_error("context is not an applying effect")),
        }
    }

    pub fn applying_effect_context_mut<'eval>(
        &'eval mut self,
    ) -> Result<&'eval mut ApplyingEffectContext<'effect, 'context, 'battle, 'data>> {
        match self {
            Self::ApplyingEffect(context) => Ok(context),
            _ => Err(general_error("context is not an applying effect")),
        }
    }

    pub fn source_applying_effect_context<'eval>(
        &'eval mut self,
    ) -> Result<Option<ApplyingEffectContext<'eval, 'eval, 'battle, 'data>>> {
        match self {
            Self::ApplyingEffect(context) => context.source_applying_effect_context(),
            _ => Err(general_error("context is not an applying effect")),
        }
    }

    pub fn source_active_move_context<'eval>(
        &'eval mut self,
    ) -> Result<Option<ActiveMoveContext<'eval, 'eval, 'eval, 'eval, 'battle, 'data>>> {
        self.effect_context_mut().source_active_move_context()
    }

    pub fn target_context<'eval>(
        &'eval mut self,
    ) -> Result<MonContext<'eval, 'eval, 'eval, 'battle, 'data>> {
        match self {
            Self::ApplyingEffect(context) => context.target_context(),
            _ => Err(general_error("effect cannot have a target")),
        }
    }

    pub fn source_context<'eval>(
        &'eval mut self,
    ) -> Result<Option<MonContext<'eval, 'eval, 'eval, 'battle, 'data>>> {
        match self {
            Self::ApplyingEffect(context) => context.source_context(),
            Self::SideEffect(context) => context.source_context(),
            Self::FieldEffect(context) => context.source_context(),
            _ => Err(general_error("effect cannot have a source")),
        }
    }

    pub fn mon_context<'eval>(
        &'eval mut self,
        mon_handle: MonHandle,
    ) -> Result<MonContext<'eval, 'eval, 'eval, 'battle, 'data>> {
        match self {
            Self::ApplyingEffect(context) => {
                if context
                    .source_handle()
                    .is_some_and(|source_handle| source_handle == mon_handle)
                {
                    context
                        .source_context()?
                        .wrap_expectation("expected source mon")
                } else if mon_handle == context.target_handle() {
                    context.target_context()
                } else {
                    context.as_battle_context_mut().mon_context(mon_handle)
                }
            }
            Self::Effect(context) => context.as_battle_context_mut().mon_context(mon_handle),
            Self::PlayerEffect(context) => {
                if context
                    .source_handle()
                    .is_some_and(|source_handle| source_handle == mon_handle)
                {
                    context
                        .source_context()?
                        .wrap_expectation("expected source mon")
                } else {
                    context.as_battle_context_mut().mon_context(mon_handle)
                }
            }
            Self::SideEffect(context) => {
                if context
                    .source_handle()
                    .is_some_and(|source_handle| source_handle == mon_handle)
                {
                    context
                        .source_context()?
                        .wrap_expectation("expected source mon")
                } else {
                    context.as_battle_context_mut().mon_context(mon_handle)
                }
            }
            Self::FieldEffect(context) => {
                if context
                    .source_handle()
                    .is_some_and(|source_handle| source_handle == mon_handle)
                {
                    context
                        .source_context()?
                        .wrap_expectation("expected source mon")
                } else {
                    context.as_battle_context_mut().mon_context(mon_handle)
                }
            }
        }
    }

    pub fn mon<'eval>(&'eval self, mon_handle: MonHandle) -> Result<&'eval Mon> {
        match self {
            Self::ApplyingEffect(context) => {
                if context
                    .source_handle()
                    .is_some_and(|source_handle| source_handle == mon_handle)
                {
                    context.source().wrap_expectation("expected source mon")
                } else if mon_handle == context.target_handle() {
                    Ok(context.target())
                } else {
                    context.as_battle_context().mon(mon_handle)
                }
            }
            Self::Effect(context) => context.as_battle_context().mon(mon_handle),
            Self::PlayerEffect(context) => {
                if context
                    .source_handle()
                    .is_some_and(|source_handle| source_handle == mon_handle)
                {
                    context.source().wrap_expectation("expected source mon")
                } else {
                    context.as_battle_context().mon(mon_handle)
                }
            }
            Self::SideEffect(context) => {
                if context
                    .source_handle()
                    .is_some_and(|source_handle| source_handle == mon_handle)
                {
                    context.source().wrap_expectation("expected source mon")
                } else {
                    context.as_battle_context().mon(mon_handle)
                }
            }
            Self::FieldEffect(context) => {
                if context
                    .source_handle()
                    .is_some_and(|source_handle| source_handle == mon_handle)
                {
                    context.source().wrap_expectation("expected source mon")
                } else {
                    context.as_battle_context().mon(mon_handle)
                }
            }
        }
    }

    pub fn mon_mut<'eval>(&'eval mut self, mon_handle: MonHandle) -> Result<&'eval mut Mon> {
        match self {
            Self::ApplyingEffect(context) => {
                if context
                    .source_handle()
                    .is_some_and(|source_handle| source_handle == mon_handle)
                {
                    context.source_mut().wrap_expectation("expected source mon")
                } else if mon_handle == context.target_handle() {
                    Ok(context.target_mut())
                } else {
                    context.as_battle_context_mut().mon_mut(mon_handle)
                }
            }
            Self::Effect(context) => context.as_battle_context_mut().mon_mut(mon_handle),
            Self::PlayerEffect(context) => {
                if context
                    .source_handle()
                    .is_some_and(|source_handle| source_handle == mon_handle)
                {
                    context.source_mut().wrap_expectation("expected source mon")
                } else {
                    context.as_battle_context_mut().mon_mut(mon_handle)
                }
            }
            Self::SideEffect(context) => {
                if context
                    .source_handle()
                    .is_some_and(|source_handle| source_handle == mon_handle)
                {
                    context.source_mut().wrap_expectation("expected source mon")
                } else {
                    context.as_battle_context_mut().mon_mut(mon_handle)
                }
            }
            Self::FieldEffect(context) => {
                if context
                    .source_handle()
                    .is_some_and(|source_handle| source_handle == mon_handle)
                {
                    context.source_mut().wrap_expectation("expected source mon")
                } else {
                    context.as_battle_context_mut().mon_mut(mon_handle)
                }
            }
        }
    }

    pub fn effect_context_for_handle<'eval>(
        &'eval mut self,
        effect_handle: &EffectHandle,
    ) -> Result<MaybeOwnedMut<'eval, EffectContext<'eval, 'battle, 'data>>> {
        if self.effect_handle() == effect_handle {
            let context = self.effect_context_mut();
            // SAFETY: We are shortening the lifetimes of this context to the lifetime of this
            // object.
            let context = unsafe {
                core::mem::transmute::<
                    &mut EffectContext<'_, '_, '_>,
                    &'eval mut EffectContext<'eval, 'battle, 'data>,
                >(context)
            };
            return Ok(context.into());
        }
        Ok(self
            .battle_context_mut()
            .effect_context(effect_handle.clone(), None)?
            .into())
    }

    pub fn active_move<'eval>(&'eval self, active_move_handle: MoveHandle) -> Result<&'eval Move> {
        self.battle_context().active_move(active_move_handle)
    }

    pub fn active_move_mut<'eval>(
        &'eval mut self,
        active_move_handle: MoveHandle,
    ) -> Result<&'eval mut Move> {
        self.battle_context_mut()
            .active_move_mut(active_move_handle)
    }

    pub fn active_move_context<'eval>(
        &'eval mut self,
        active_move_handle: MoveHandle,
    ) -> Result<ActiveMoveContext<'eval, 'eval, 'eval, 'eval, 'battle, 'data>> {
        self.battle_context_mut()
            .active_move_context(active_move_handle, MoveHitEffectType::PrimaryEffect)
    }

    pub fn target_handle(&self) -> Option<MonHandle> {
        match self {
            Self::ApplyingEffect(context) => Some(context.target_handle()),
            _ => None,
        }
    }

    pub fn source_handle(&self) -> Option<MonHandle> {
        match self {
            Self::ApplyingEffect(context) => context.source_handle(),
            Self::PlayerEffect(context) => context.source_handle(),
            Self::SideEffect(context) => context.source_handle(),
            Self::FieldEffect(context) => context.source_handle(),
            _ => None,
        }
    }

    pub fn effect_handle(&self) -> &EffectHandle {
        match self {
            Self::ApplyingEffect(context) => context.effect_handle(),
            Self::Effect(context) => context.effect_handle(),
            Self::PlayerEffect(context) => context.effect_handle(),
            Self::SideEffect(context) => context.effect_handle(),
            Self::FieldEffect(context) => context.effect_handle(),
        }
    }

    pub fn source_effect_handle(&self) -> Option<&EffectHandle> {
        match self {
            Self::ApplyingEffect(context) => context.source_effect_handle(),
            Self::Effect(context) => context.source_effect_handle(),
            Self::PlayerEffect(context) => context.source_effect_handle(),
            Self::SideEffect(context) => context.source_effect_handle(),
            Self::FieldEffect(context) => context.source_effect_handle(),
        }
    }

    pub fn source_active_move_handle(&self) -> Option<MoveHandle> {
        if let Some(EffectHandle::ActiveMove(active_move_handle, _)) = self.source_effect_handle() {
            Some(*active_move_handle)
        } else {
            None
        }
    }

    pub fn side_index(&self) -> Option<usize> {
        match self {
            Self::ApplyingEffect(context) => Some(context.target().side),
            Self::Effect(_) => None,
            Self::PlayerEffect(context) => Some(context.player().side),
            Self::SideEffect(context) => Some(context.side().index),
            Self::FieldEffect(_) => None,
        }
    }

    pub fn player_index(&self) -> Option<usize> {
        match self {
            Self::ApplyingEffect(context) => Some(context.target().player),
            Self::Effect(_) => None,
            Self::PlayerEffect(context) => Some(context.player().index),
            Self::SideEffect(_) => None,
            Self::FieldEffect(_) => None,
        }
    }
}
