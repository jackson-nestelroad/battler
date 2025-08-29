use std::{
    collections::VecDeque,
    mem,
};

use anyhow::{
    Error,
    Result,
};
use battler_data::{
    Fraction,
    Identifiable,
};
use zone_alloc::{
    BorrowError,
    ElementRef,
    ElementRefMut,
    KeyedRegistry,
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
        MonExitType,
        MonHandle,
        MoveHandle,
        Player,
        PlayerEffectContext,
        SideEffectContext,
        mon_states,
        weather_states,
    },
    common::{
        MaybeOwnedMut,
        UnsafelyDetachBorrowMut,
    },
    effect::{
        ActiveMoveEffectStateConnector,
        EffectHandle,
        MonStatusEffectStateConnector,
        fxlang::{
            BattleEvent,
            CallbackFlag,
            DynamicEffectStateConnector,
            EffectStateConnector,
            EventState,
            MaybeReferenceValue,
            MaybeReferenceValueForOperation,
            ParsedProgramBlock,
            Value,
            ValueRef,
            ValueRefMut,
            ValueRefToStoredValue,
            ValueType,
            parsed_effect::ParsedCallback,
            run_function,
            tree,
        },
    },
    error::{
        WrapOptionError,
        WrapResultError,
        general_error,
        integer_overflow_error,
    },
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

    fn mon_mut<'eval>(&'eval mut self, mon_handle: MonHandle) -> Result<&'eval mut Mon> {
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
            let context: &'eval mut EffectContext<'eval, 'battle, 'data> =
                unsafe { mem::transmute(context) };
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

/// A registry of variables for an fxlang program evaluation.
struct VariableRegistry {
    vars: KeyedRegistry<String, Value>,
}

impl VariableRegistry {
    pub fn new() -> Self {
        Self {
            vars: KeyedRegistry::new(),
        }
    }

    fn get(&self, var: &str) -> Result<Option<ElementRef<'_, Value>>> {
        match self.vars.get(var) {
            Ok(val) => Ok(Some(val)),
            Err(BorrowError::OutOfBounds) => Ok(None),
            Err(_) => Err(general_error(format!("failed to borrow variable ${var}"))),
        }
    }

    fn get_mut(&self, var: &str) -> Result<Option<ElementRefMut<'_, Value>>> {
        match self.vars.get_mut(var) {
            Ok(val) => Ok(Some(val)),
            Err(BorrowError::OutOfBounds) => Ok(None),
            Err(_) => Err(general_error(format!("failed to borrow variable ${var}"))),
        }
    }

    fn set(&self, var: &str, value: Value) -> Result<()> {
        match self.vars.get_mut(var) {
            Ok(mut var) => {
                *var = value;
                Ok(())
            }
            Err(BorrowError::OutOfBounds) => {
                self.vars.register(var.to_owned(), value);
                Ok(())
            }
            Err(_) => Err(general_error(format!(
                "failed to mutably borrow variable ${var}"
            ))),
        }
    }
}

/// An fxlang variable.
///
/// Acts as a wrapper for an immutable access of a variable that can be consumed at some later time.
struct Variable<'eval, 'program> {
    stored: Option<ElementRef<'eval, Value>>,
    member_access: Vec<&'program str>,
}

impl<'eval, 'program> Variable<'eval, 'program>
where
    'program: 'eval,
{
    fn new(stored: Option<ElementRef<'eval, Value>>, member_access: Vec<&'program str>) -> Self {
        Self {
            stored,
            member_access,
        }
    }

    fn bad_member_access(member: &str, value_type: ValueType) -> Error {
        general_error(format!("value of type {value_type} has no member {member}"))
    }

    fn get_ref<'var>(&'var self, context: &'eval mut EvaluationContext) -> Result<ValueRef<'var>> {
        let mut value = match &self.stored {
            Some(stored) => ValueRef::from(stored),
            None => ValueRef::Undefined,
        };
        for member in &self.member_access {
            // SAFETY: For changing the lifetime of context: the mutable reference inside of
            // `value_ref` is only mutated at the very end of this method. Thus, this entire for
            // loop is actually immutable. Furthermore, since we only hold one
            // `value_ref` at a time, so there is no way to hold multiple mutable borrow
            // of values inside `context` at the same time.
            //
            // We can imagine that `value_ref` really does correctly mutably borrow `context`. If
            // the next iteration somehow also needs to borrow from `context`, the previous
            // `value_ref` value (i.e., the mutable borrow inside of it) is dropped.
            let value_type = value.value_type();

            match *member {
                "is_boolean" => {
                    value = ValueRef::Boolean(value.is_boolean());
                }
                "is_defined" => {
                    value = ValueRef::Boolean(!value.is_undefined());
                }
                "is_empty" => {
                    value = ValueRef::Boolean(value.is_empty());
                }
                "is_undefined" => {
                    value = ValueRef::Boolean(value.is_undefined());
                }
                "length" => {
                    value = match value.len() {
                        Some(len) => ValueRef::UFraction(
                            TryInto::<u64>::try_into(len)
                                .map_err(integer_overflow_error)?
                                .into(),
                        ),
                        None => ValueRef::Undefined,
                    }
                }
                "to_string" => {
                    value = ValueRef::TempString(
                        MaybeReferenceValueForOperation::from(value).for_formatted_string()?,
                    )
                }
                _ => {
                    let mut effect_matched = false;
                    if let Some(effect_handle) = value.effect_handle() {
                        effect_matched = true;
                        let context = unsafe { context.unsafely_detach_borrow_mut() };
                        value = match *member {
                            "base_power" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| ValueRef::UFraction(mov.data.base_power.into()))
                            .unwrap_or(ValueRef::Undefined),
                            "category" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| ValueRef::MoveCategory(mov.data.category))
                            .unwrap_or(ValueRef::Undefined),
                            "condition" => ValueRef::TempEffect(
                                effect_handle
                                    .condition_handle(context.battle_context())?
                                    .wrap_expectation("effect has no associated condition")?,
                            ),
                            "id" => ValueRef::TempString(
                                CoreBattle::get_effect_by_handle(
                                    context.battle_context(),
                                    &effect_handle,
                                )?
                                .id()
                                .as_ref()
                                .to_owned(),
                            ),
                            "is_ability" => ValueRef::Boolean(effect_handle.is_ability()),
                            "is_item" => ValueRef::Boolean(effect_handle.is_item()),
                            "is_move" => ValueRef::Boolean(effect_handle.is_active_move()),
                            "is_move_secondary" => {
                                ValueRef::Boolean(effect_handle.is_active_move_secondary())
                            }
                            "is_raining" => ValueRef::Boolean(weather_states::is_raining(
                                context.effect_context_for_handle(&effect_handle)?.as_mut(),
                            )),
                            "is_snowing" => ValueRef::Boolean(weather_states::is_snowing(
                                context.effect_context_for_handle(&effect_handle)?.as_mut(),
                            )),
                            "is_sunny" => ValueRef::Boolean(weather_states::is_sunny(
                                context.effect_context_for_handle(&effect_handle)?.as_mut(),
                            )),
                            "move_target" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| ValueRef::MoveTarget(mov.data.target))
                            .unwrap_or(ValueRef::Undefined),
                            "name" => ValueRef::TempString(
                                CoreBattle::get_effect_by_handle(
                                    context.battle_context(),
                                    &effect_handle,
                                )?
                                .name()
                                .to_owned(),
                            ),
                            "ohko" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| ValueRef::Boolean(mov.data.ohko_type.is_some()))
                            .unwrap_or(ValueRef::Undefined),
                            "type" => CoreBattle::get_effect_by_handle(
                                context.battle_context(),
                                &effect_handle,
                            )?
                            .move_effect()
                            .map(|mov| ValueRef::Type(mov.data.primary_type))
                            .unwrap_or(ValueRef::Undefined),
                            _ => {
                                if effect_handle.is_active_move() {
                                    // Allow active move to fall through.
                                    effect_matched = false;
                                    value
                                } else {
                                    return Err(Self::bad_member_access(member, value_type));
                                }
                            }
                        }
                    }

                    if effect_matched {
                        continue;
                    }

                    if let Some(active_move_handle) = value.active_move_handle() {
                        let context = unsafe { context.unsafely_detach_borrow_mut() };
                        value = match *member {
                            "accuracy" => ValueRef::Accuracy(
                                context.active_move(active_move_handle)?.data.accuracy,
                            ),
                            "base_power" => ValueRef::UFraction(
                                context
                                    .active_move(active_move_handle)?
                                    .data
                                    .base_power
                                    .into(),
                            ),
                            "category" => ValueRef::MoveCategory(
                                context.active_move(active_move_handle)?.data.category,
                            ),
                            "damage" => {
                                match context.active_move(active_move_handle)?.data.damage {
                                    Some(damage) => ValueRef::UFraction(damage.into()),
                                    None => ValueRef::Undefined,
                                }
                            }
                            "drain_percent" => ValueRef::UFraction(
                                context
                                    .active_move(active_move_handle)?
                                    .data
                                    .drain_percent
                                    .unwrap_or(Fraction::from(0u16))
                                    .convert(),
                            ),
                            "effect_state" => ValueRef::EffectState(
                                ActiveMoveEffectStateConnector::new(active_move_handle)
                                    .make_dynamic(),
                            ),
                            "hit" => ValueRef::UFraction(
                                context.active_move(active_move_handle)?.hit.into(),
                            ),
                            "hit_effect" => context
                                .active_move(active_move_handle)?
                                .data
                                .hit_effect
                                .as_ref()
                                .map(ValueRef::HitEffect)
                                .unwrap_or(ValueRef::Undefined),
                            "multiaccuracy" => ValueRef::Boolean(
                                context.active_move(active_move_handle)?.data.multiaccuracy,
                            ),
                            "multihit" => context
                                .active_move(active_move_handle)?
                                .data
                                .multihit
                                .map(|val| ValueRef::MultihitType(val))
                                .unwrap_or(ValueRef::Undefined),
                            "ohko" => ValueRef::Boolean(
                                context
                                    .active_move(active_move_handle)?
                                    .data
                                    .ohko_type
                                    .is_some(),
                            ),
                            "priority" => ValueRef::Fraction(
                                context
                                    .active_move(active_move_handle)?
                                    .data
                                    .priority
                                    .into(),
                            ),
                            "recoil_percent" => ValueRef::UFraction(
                                context
                                    .active_move(active_move_handle)?
                                    .data
                                    .recoil_percent
                                    .unwrap_or(Fraction::from(0u16))
                                    .convert(),
                            ),
                            "secondary_effects" => ValueRef::TempList(
                                context
                                    .active_move(active_move_handle)?
                                    .data
                                    .secondary_effects
                                    .iter()
                                    .map(|val| {
                                        ValueRefToStoredValue::new(
                                            self.stored.clone(),
                                            ValueRef::SecondaryHitEffect(val),
                                        )
                                    })
                                    .collect(),
                            ),
                            "source" | "user" => {
                                match context.active_move(active_move_handle)?.used_by {
                                    Some(mon) => ValueRef::Mon(mon),
                                    None => ValueRef::Undefined,
                                }
                            }
                            "target" => ValueRef::MoveTarget(
                                context.active_move(active_move_handle)?.data.target,
                            ),
                            "total_damage" => ValueRef::UFraction(
                                context.active_move(active_move_handle)?.total_damage.into(),
                            ),
                            "type" => ValueRef::Type(
                                context.active_move(active_move_handle)?.data.primary_type,
                            ),
                            "typeless" => ValueRef::Boolean(
                                context.active_move(active_move_handle)?.data.typeless,
                            ),
                            "user_effect" => context
                                .active_move(active_move_handle)?
                                .data
                                .user_effect
                                .as_ref()
                                .map(ValueRef::HitEffect)
                                .unwrap_or(ValueRef::Undefined),
                            "user_effect_chance" => context
                                .active_move(active_move_handle)?
                                .data
                                .user_effect_chance
                                .map(|val| ValueRef::UFraction(val.convert()))
                                .unwrap_or(ValueRef::Undefined),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let Some(mon_handle) = value.mon_handle() {
                        let context = unsafe { context.unsafely_detach_borrow_mut() };
                        value = match *member {
                            "ability" => ValueRef::TempString(
                                context.mon(mon_handle)?.ability.id.to_string(),
                            ),
                            "active" => ValueRef::Boolean(context.mon(mon_handle)?.active),
                            "active_move" => context
                                .mon(mon_handle)?
                                .active_move
                                .map(|active_move| ValueRef::ActiveMove(active_move))
                                .unwrap_or(ValueRef::Undefined),
                            "active_move_actions" => ValueRef::UFraction(
                                context.mon(mon_handle)?.active_move_actions.into(),
                            ),
                            "active_turns" => {
                                ValueRef::UFraction(context.mon(mon_handle)?.active_turns.into())
                            }
                            "affection_level" => ValueRef::UFraction(
                                context.mon(mon_handle)?.affection_level().into(),
                            ),
                            "base_max_hp" => {
                                ValueRef::UFraction(context.mon(mon_handle)?.base_max_hp.into())
                            }
                            "base_stats" => {
                                ValueRef::StatTable(&context.mon(mon_handle)?.base_stored_stats)
                            }
                            "being_called_back" => {
                                ValueRef::Boolean(context.mon(mon_handle)?.being_called_back)
                            }
                            "berry_eating_health" => ValueRef::UFraction(
                                mon_states::berry_eating_health(
                                    &mut context.mon_context(mon_handle)?,
                                )
                                .into(),
                            ),
                            "boosts" => ValueRef::BoostTable(&context.mon(mon_handle)?.boosts),
                            "can_heal" => ValueRef::Boolean(mon_states::can_heal(
                                &mut context.mon_context(mon_handle)?,
                            )),
                            "effective_ability" => {
                                match mon_states::effective_ability(
                                    &mut context.mon_context(mon_handle)?,
                                ) {
                                    Some(ability) => {
                                        ValueRef::TempEffect(EffectHandle::Ability(ability))
                                    }
                                    None => ValueRef::Undefined,
                                }
                            }
                            "effective_item" => {
                                match mon_states::effective_item(
                                    &mut context.mon_context(mon_handle)?,
                                ) {
                                    Some(item) => ValueRef::TempEffect(EffectHandle::Item(item)),
                                    None => ValueRef::Undefined,
                                }
                            }
                            "effective_weather" => {
                                match mon_states::effective_weather(
                                    &mut context.mon_context(mon_handle)?,
                                ) {
                                    Some(weather) => ValueRef::Effect(
                                        context
                                            .battle_context_mut()
                                            .battle_mut()
                                            .get_effect_handle_by_id(&weather)?,
                                    ),
                                    None => ValueRef::Undefined,
                                }
                            }
                            "effective_terrain" => {
                                match mon_states::effective_terrain(
                                    &mut context.mon_context(mon_handle)?,
                                ) {
                                    Some(weather) => ValueRef::Effect(
                                        context
                                            .battle_context_mut()
                                            .battle_mut()
                                            .get_effect_handle_by_id(&weather)?,
                                    ),
                                    None => ValueRef::Undefined,
                                }
                            }
                            "exited" => {
                                ValueRef::Boolean(context.mon(mon_handle)?.exited.is_some())
                            }
                            "fainted" => ValueRef::Boolean(
                                context.mon(mon_handle)?.exited == Some(MonExitType::Fainted),
                            ),
                            "foe_side" => {
                                ValueRef::Side(context.mon_context(mon_handle)?.foe_side().index)
                            }
                            "force_switch" => {
                                ValueRef::Boolean(context.mon(mon_handle)?.force_switch.is_some())
                            }
                            "friendship" => {
                                ValueRef::UFraction(context.mon(mon_handle)?.friendship.into())
                            }
                            "gender" => ValueRef::Gender(context.mon(mon_handle)?.gender),
                            "hidden_power_type" => {
                                ValueRef::Type(context.mon(mon_handle)?.hidden_power_type)
                            }
                            "hp" => ValueRef::UFraction(context.mon(mon_handle)?.hp.into()),
                            "damaged_this_turn" => {
                                ValueRef::Boolean(context.mon(mon_handle)?.damaged_this_turn)
                            }
                            "is_asleep" => ValueRef::Boolean(mon_states::is_asleep(
                                &mut context.mon_context(mon_handle)?,
                            )),
                            "is_behind_substitute" => {
                                ValueRef::Boolean(mon_states::is_behind_substitute(
                                    &mut context.mon_context(mon_handle)?,
                                ))
                            }
                            "is_grounded" => ValueRef::Boolean(mon_states::is_grounded(
                                &mut context.mon_context(mon_handle)?,
                            )),
                            "is_away_from_field" => {
                                ValueRef::Boolean(mon_states::is_away_from_field(
                                    &mut context.mon_context(mon_handle)?,
                                ))
                            }
                            "is_soundproof" => ValueRef::Boolean(mon_states::is_soundproof(
                                &mut context.mon_context(mon_handle)?,
                            )),
                            "is_immune_to_entry_hazards" => {
                                ValueRef::Boolean(mon_states::is_immune_to_entry_hazards(
                                    &mut context.mon_context(mon_handle)?,
                                ))
                            }
                            "item" => match context.mon(mon_handle)?.item.as_ref() {
                                Some(item) => ValueRef::TempString(item.id.to_string()),
                                None => ValueRef::Undefined,
                            },
                            "item_used_this_turn" => {
                                ValueRef::Boolean(context.mon(mon_handle)?.item_used_this_turn)
                            }
                            "last_item" => match context.mon(mon_handle)?.last_item.as_ref() {
                                Some(item) => ValueRef::TempString(item.to_string()),
                                None => ValueRef::Undefined,
                            },
                            "last_move" => match context.mon(mon_handle)?.last_move {
                                Some(last_move) => ValueRef::ActiveMove(last_move),
                                _ => ValueRef::Undefined,
                            },
                            "last_move_used" => match context.mon(mon_handle)?.last_move_used {
                                Some(last_move_used) => ValueRef::ActiveMove(last_move_used),
                                _ => ValueRef::Undefined,
                            },
                            "last_target_location" => {
                                match context.mon(mon_handle)?.last_move_target_location {
                                    Some(last_target_location) => ValueRef::Fraction(
                                        TryInto::<i32>::try_into(last_target_location)
                                            .map_err(integer_overflow_error)?
                                            .into(),
                                    ),
                                    None => ValueRef::Undefined,
                                }
                            }
                            "level" => ValueRef::UFraction(context.mon(mon_handle)?.level.into()),
                            "max_hp" => ValueRef::UFraction(context.mon(mon_handle)?.max_hp.into()),
                            "move_last_turn_succeeded" => ValueRef::Boolean(
                                context
                                    .mon(mon_handle)?
                                    .move_last_turn_outcome
                                    .map(|outcome| outcome.success())
                                    .unwrap_or(false),
                            ),
                            "move_slots" => ValueRef::TempList(
                                context
                                    .mon(mon_handle)?
                                    .move_slots
                                    .iter()
                                    .map(|move_slot| {
                                        ValueRefToStoredValue::new(
                                            self.stored.clone(),
                                            ValueRef::MoveSlot(move_slot),
                                        )
                                    })
                                    .collect(),
                            ),
                            "move_this_turn_failed" => ValueRef::Boolean(
                                context
                                    .mon(mon_handle)?
                                    .move_this_turn_outcome
                                    .map(|outcome| !outcome.success())
                                    .unwrap_or(false),
                            ),
                            "name" => ValueRef::String(&context.mon(mon_handle)?.name),
                            "nature" => ValueRef::Nature(context.mon(mon_handle)?.nature),
                            "needs_switch" => {
                                ValueRef::Boolean(context.mon(mon_handle)?.needs_switch.is_some())
                            }
                            "newly_switched" => {
                                ValueRef::Boolean(context.mon(mon_handle)?.newly_switched)
                            }
                            "player" => ValueRef::Player(context.mon(mon_handle)?.player),
                            "position" => {
                                match Mon::position_on_side(&context.mon_context(mon_handle)?) {
                                    Some(position) => ValueRef::UFraction(
                                        TryInto::<u32>::try_into(position)
                                            .map_err(integer_overflow_error)?
                                            .into(),
                                    ),
                                    None => ValueRef::Undefined,
                                }
                            }
                            "position_details" => ValueRef::TempString(format!(
                                "{}",
                                Mon::position_details(&context.mon_context(mon_handle)?)?
                            )),
                            "positive_boosts" => ValueRef::UFraction(
                                Mon::positive_boosts(&context.mon_context(mon_handle)?).into(),
                            ),
                            "side" => ValueRef::Side(context.mon(mon_handle)?.side),
                            "species" => ValueRef::Str(&context.mon(mon_handle)?.species.as_ref()),
                            "stats" => ValueRef::StatTable(&context.mon(mon_handle)?.stats),
                            "status" => match context.mon(mon_handle)?.status.as_ref() {
                                Some(status) => ValueRef::TempString(status.as_ref().to_owned()),
                                None => ValueRef::Undefined,
                            },
                            "transformed" => {
                                ValueRef::Boolean(context.mon(mon_handle)?.transformed)
                            }
                            "true_nature" => ValueRef::Nature(context.mon(mon_handle)?.true_nature),
                            "types" => ValueRef::TempList(
                                context
                                    .mon(mon_handle)?
                                    .types
                                    .iter()
                                    .map(|val| {
                                        ValueRefToStoredValue::new(None, ValueRef::Type(*val))
                                    })
                                    .collect(),
                            ),
                            "weight" => ValueRef::UFraction(
                                Mon::get_weight(&mut context.mon_context(mon_handle)?).into(),
                            ),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::Player(player) = value {
                        let context = unsafe { context.unsafely_detach_borrow_mut() };
                        value = match *member {
                            "can_escape" => ValueRef::Boolean(Player::can_escape(
                                &context.battle_context_mut().player_context(player)?,
                            )),
                            "mons" | "team" => ValueRef::TempList(
                                context
                                    .battle_context_mut()
                                    .player_context(player)?
                                    .player()
                                    .mons
                                    .iter()
                                    .cloned()
                                    .map(|mon| ValueRefToStoredValue::new(None, ValueRef::Mon(mon)))
                                    .collect(),
                            ),
                            "wild_encounter_type" => Player::wild_encounter_type(
                                &mut context.battle_context_mut().player_context(player)?,
                            )
                            .map(|val| ValueRef::WildEncounterType(val))
                            .unwrap_or(ValueRef::Undefined),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::MoveSlot(move_slot) = value {
                        value = match *member {
                            "id" => ValueRef::Str(move_slot.id.as_ref()),
                            "max_pp" => ValueRef::UFraction(move_slot.max_pp.into()),
                            "name" => ValueRef::String(&move_slot.name),
                            "pp" => ValueRef::UFraction(move_slot.pp.into()),
                            "used" => ValueRef::Boolean(move_slot.used),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::Battle = value {
                        let context = unsafe { context.unsafely_detach_borrow_mut() };
                        value = match *member {
                            "last_move" => context
                                .battle_context()
                                .battle()
                                .last_move()
                                .map(|move_handle| ValueRef::ActiveMove(move_handle))
                                .unwrap_or(ValueRef::Undefined),
                            "turn" => {
                                ValueRef::UFraction(context.battle_context().battle().turn().into())
                            }
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::Field = value {
                        let context = unsafe { context.unsafely_detach_borrow_mut() };
                        value = match *member {
                            "effective_terrain" => {
                                match Field::effective_terrain(context.battle_context_mut()) {
                                    Some(weather) => ValueRef::Effect(
                                        context
                                            .battle_context_mut()
                                            .battle_mut()
                                            .get_effect_handle_by_id(&weather)?,
                                    ),
                                    None => ValueRef::Undefined,
                                }
                            }
                            "effective_weather" => {
                                match Field::effective_weather(context.battle_context_mut()) {
                                    Some(weather) => ValueRef::Effect(
                                        context
                                            .battle_context_mut()
                                            .battle_mut()
                                            .get_effect_handle_by_id(&weather)?,
                                    ),
                                    None => ValueRef::Undefined,
                                }
                            }
                            "environment" => ValueRef::FieldEnvironment(
                                context.battle_context().battle().field.environment,
                            ),
                            "sides" => ValueRef::TempList(
                                context
                                    .battle_context()
                                    .battle()
                                    .side_indices()
                                    .map(|side_index| {
                                        ValueRefToStoredValue::new(None, ValueRef::Side(side_index))
                                    })
                                    .collect(),
                            ),
                            "time" => {
                                ValueRef::TimeOfDay(context.battle_context().battle().field.time)
                            }
                            "weather" => {
                                match context.battle_context().battle().field.weather.clone() {
                                    Some(weather) => ValueRef::Effect(
                                        context
                                            .battle_context_mut()
                                            .battle_mut()
                                            .get_effect_handle_by_id(&weather)?,
                                    ),
                                    None => ValueRef::Undefined,
                                }
                            }
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::Format = value {
                        let context = unsafe { context.unsafely_detach_borrow_mut() };
                        value = match *member {
                            "mons_per_side" => ValueRef::UFraction(
                                TryInto::<u64>::try_into(
                                    context.battle_context().battle().max_side_length(),
                                )
                                .map_err(integer_overflow_error)?
                                .into(),
                            ),
                            "obedience_cap" => ValueRef::UFraction(
                                context
                                    .battle_context()
                                    .battle()
                                    .format
                                    .options
                                    .obedience_cap
                                    .into(),
                            ),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        };
                    } else if let ValueRef::HitEffect(hit_effect) = value {
                        value = match *member {
                            "boosts" => hit_effect
                                .boosts
                                .as_ref()
                                .map(ValueRef::BoostTable)
                                .unwrap_or(ValueRef::Undefined),
                            "volatile_status" => hit_effect
                                .volatile_status
                                .as_ref()
                                .map(ValueRef::String)
                                .unwrap_or(ValueRef::Undefined),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::SecondaryHitEffect(secondary_effect) = value {
                        value = match *member {
                            "chance" => secondary_effect
                                .chance
                                .as_ref()
                                .map(|val| ValueRef::UFraction(val.convert()))
                                .unwrap_or(ValueRef::Undefined),
                            "target" => secondary_effect
                                .target
                                .as_ref()
                                .map(ValueRef::HitEffect)
                                .unwrap_or(ValueRef::Undefined),
                            "user" => secondary_effect
                                .user
                                .as_ref()
                                .map(ValueRef::HitEffect)
                                .unwrap_or(ValueRef::Undefined),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::BoostTable(boosts) = value {
                        value = match *member {
                            "acc" => ValueRef::Fraction(boosts.acc.into()),
                            "atk" => ValueRef::Fraction(boosts.atk.into()),
                            "def" => ValueRef::Fraction(boosts.def.into()),
                            "eva" => ValueRef::Fraction(boosts.eva.into()),
                            "spa" => ValueRef::Fraction(boosts.spa.into()),
                            "spd" => ValueRef::Fraction(boosts.spd.into()),
                            "spe" => ValueRef::Fraction(boosts.spe.into()),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::StatTable(stats) = value {
                        value = match *member {
                            "atk" => ValueRef::Fraction(stats.atk.into()),
                            "def" => ValueRef::Fraction(stats.def.into()),
                            "hp" => ValueRef::Fraction(stats.hp.into()),
                            "spa" => ValueRef::Fraction(stats.spa.into()),
                            "spd" => ValueRef::Fraction(stats.spd.into()),
                            "spe" => ValueRef::Fraction(stats.spe.into()),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::Nature(nature) = value {
                        value = match *member {
                            "boosts" => {
                                ValueRef::Boost(nature.boosts().try_into().map_err(general_error)?)
                            }
                            "drops" => {
                                ValueRef::Boost(nature.drops().try_into().map_err(general_error)?)
                            }
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::SpecialItemData(special_item_data) = value {
                        value = match *member {
                            "fling" => special_item_data
                                .fling
                                .as_ref()
                                .map(ValueRef::FlingData)
                                .unwrap_or(ValueRef::Undefined),
                            "judgment" => special_item_data
                                .judgment
                                .as_ref()
                                .map(ValueRef::JudgmentData)
                                .unwrap_or(ValueRef::Undefined),
                            "natural_gift" => special_item_data
                                .natural_gift
                                .as_ref()
                                .map(ValueRef::NaturalGiftData)
                                .unwrap_or(ValueRef::Undefined),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::FlingData(fling_data) = value {
                        value = match *member {
                            "hit_effect" => fling_data
                                .hit_effect
                                .as_ref()
                                .map(|hit_effect| ValueRef::HitEffect(hit_effect))
                                .unwrap_or(ValueRef::Undefined),
                            "power" => ValueRef::UFraction(fling_data.power.into()),
                            "use_item" => ValueRef::Boolean(fling_data.use_item),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::NaturalGiftData(natural_gift_data) = value {
                        value = match *member {
                            "power" => ValueRef::UFraction(natural_gift_data.power.into()),
                            "type" => ValueRef::Type(natural_gift_data.typ),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::MultihitType(multihit) = value {
                        value = match *member {
                            "max" => ValueRef::UFraction(multihit.max().into()),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::JudgmentData(judgment_data) = value {
                        value = match *member {
                            "type" => ValueRef::Type(judgment_data.typ),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::EffectState(connector) = value {
                        let context = unsafe { context.unsafely_detach_borrow_mut() };
                        value = connector
                            .get_mut(context.battle_context_mut())?
                            .get(*member)
                            .map(ValueRef::from)
                            .unwrap_or(ValueRef::Undefined);
                    } else if let ValueRef::Object(object) = value {
                        value = match object.get(*member) {
                            Some(value) => ValueRef::from(value),
                            _ => ValueRef::Undefined,
                        };
                    } else {
                        return Err(Self::bad_member_access(member, value_type));
                    }
                }
            }
        }

        Ok(value)
    }

    fn get(self, context: &'eval mut EvaluationContext) -> Result<ValueRefToStoredValue<'eval>> {
        let value_ref = self.get_ref(context)?;
        // SAFETY: This ValueRef references some internal part of `self.stored`. Since we are
        // bundling this reference alongside the owner object (which has runtime borrow checking),
        // we promote this reference to its lifetime.
        //
        // An added bonus is that we know we only use this value for an immutable operation.
        let value_ref: ValueRef<'eval> = unsafe { mem::transmute(value_ref) };
        Ok(ValueRefToStoredValue::new(self.stored, value_ref))
    }
}

/// A mutable fxlang variable.
///
/// Acts as a wrapper for a mutable access of a variable that can be consumed at some later time.
struct VariableMut<'eval, 'program> {
    stored: ElementRefMut<'eval, Value>,
    member_access: Vec<&'program str>,
}

impl<'eval, 'program> VariableMut<'eval, 'program>
where
    'program: 'eval,
{
    fn new(stored: ElementRefMut<'eval, Value>, member_access: Vec<&'program str>) -> Self {
        Self {
            stored,
            member_access,
        }
    }

    fn bad_member_or_mutable_access(member: &str, value_type: ValueType) -> Error {
        general_error(format!(
            "value of type {value_type} has no member {member} or the member is immutable",
        ))
    }

    fn get_ref_mut<'var>(
        &'var mut self,
        context: &'eval mut EvaluationContext,
    ) -> Result<ValueRefMut<'var>> {
        let mut value = ValueRefMut::from(self.stored.as_mut());

        for member in &self.member_access {
            let value_type = value.value_type();

            // SAFETY: For changing the lifetime of context: the mutable reference inside of
            // `value_ref` is only mutated at the very end of this method. Thus, this entire for
            // loop is actually immutable. Furthermore, since we only hold one
            // `value_ref` at a time, so there is no way to hold multiple mutable borrow
            // of values inside `context` at the same time.
            //
            // We can imagine that `value_ref` really does correctly mutably borrow `context`. If
            // the next iteration somehow also needs to borrow from `context`, the previous
            // `value_ref` value (i.e., the mutable borrow inside of it) is dropped.
            match value {
                ValueRefMut::Mon(ref mon_handle) => {
                    let context = unsafe { context.unsafely_detach_borrow_mut() };
                    value = match *member {
                        "boosts" => {
                            ValueRefMut::BoostTable(&mut context.mon_mut(**mon_handle)?.boosts)
                        }
                        "last_item" => {
                            ValueRefMut::OptionalId(&mut context.mon_mut(**mon_handle)?.last_item)
                        }
                        "last_target_location" => ValueRefMut::OptionalISize(
                            &mut context.mon_mut(**mon_handle)?.last_move_target_location,
                        ),
                        "skip_before_switch_out" => ValueRefMut::Boolean(
                            &mut context.mon_mut(**mon_handle)?.skip_before_switch_out,
                        ),
                        "stats" => {
                            ValueRefMut::StatTable(&mut context.mon_mut(**mon_handle)?.stats)
                        }
                        "status_state" => ValueRefMut::TempEffectState(
                            MonStatusEffectStateConnector::new(**mon_handle).make_dynamic(),
                        ),
                        _ => return Err(Self::bad_member_or_mutable_access(member, value_type)),
                    }
                }
                ValueRefMut::ActiveMove(ref active_move_handle) => {
                    let context = unsafe { context.unsafely_detach_borrow_mut() };
                    value = match *member {
                        "accuracy" => ValueRefMut::Accuracy(
                            &mut context.active_move_mut(**active_move_handle)?.data.accuracy,
                        ),
                        "base_power" => ValueRefMut::U32(
                            &mut context
                                .active_move_mut(**active_move_handle)?
                                .data
                                .base_power,
                        ),
                        "damage" => ValueRefMut::OptionalU16(
                            &mut context.active_move_mut(**active_move_handle)?.data.damage,
                        ),
                        "effect_state" => ValueRefMut::TempEffectState(
                            ActiveMoveEffectStateConnector::new(**active_move_handle)
                                .make_dynamic(),
                        ),
                        "hit_effect" => ValueRefMut::OptionalHitEffect(
                            &mut context
                                .active_move_mut(**active_move_handle)?
                                .data
                                .hit_effect,
                        ),
                        "multiaccuracy" => ValueRefMut::Boolean(
                            &mut context
                                .active_move_mut(**active_move_handle)?
                                .data
                                .multiaccuracy,
                        ),
                        "multihit" => ValueRefMut::OptionalMultihitType(
                            &mut context.active_move_mut(**active_move_handle)?.data.multihit,
                        ),
                        "secondary_effects" => ValueRefMut::SecondaryHitEffectList(
                            &mut context
                                .active_move_mut(**active_move_handle)?
                                .data
                                .secondary_effects,
                        ),
                        "target" => ValueRefMut::MoveTarget(
                            &mut context.active_move_mut(**active_move_handle)?.data.target,
                        ),
                        "total_damage" => ValueRefMut::U64(
                            &mut context.active_move_mut(**active_move_handle)?.total_damage,
                        ),
                        "type" => ValueRefMut::Type(
                            &mut context
                                .active_move_mut(**active_move_handle)?
                                .data
                                .primary_type,
                        ),
                        "user_effect" => ValueRefMut::OptionalHitEffect(
                            &mut context
                                .active_move_mut(**active_move_handle)?
                                .data
                                .user_effect,
                        ),
                        "user_effect_chance" => ValueRefMut::OptionalFractionU16(
                            &mut context
                                .active_move_mut(**active_move_handle)?
                                .data
                                .user_effect_chance,
                        ),
                        _ => return Err(Self::bad_member_or_mutable_access(member, value_type)),
                    }
                }
                ValueRefMut::HitEffect(hit_effect)
                | ValueRefMut::OptionalHitEffect(Some(hit_effect)) => {
                    value = match *member {
                        "boosts" => ValueRefMut::OptionalBoostTable(&mut hit_effect.boosts),
                        "heal_percent" => {
                            ValueRefMut::OptionalFractionU16(&mut hit_effect.heal_percent)
                        }
                        "status" => ValueRefMut::OptionalString(&mut hit_effect.status),
                        "volatile_status" => {
                            ValueRefMut::OptionalString(&mut hit_effect.volatile_status)
                        }
                        _ => return Err(Self::bad_member_or_mutable_access(member, value_type)),
                    }
                }
                ValueRefMut::SecondaryHitEffect(secondary_effect) => {
                    value = match *member {
                        "chance" => ValueRefMut::OptionalFractionU16(&mut secondary_effect.chance),
                        "target" => ValueRefMut::OptionalHitEffect(&mut secondary_effect.target),
                        "user" => ValueRefMut::OptionalHitEffect(&mut secondary_effect.user),
                        _ => return Err(Self::bad_member_or_mutable_access(member, value_type)),
                    }
                }
                ValueRefMut::BoostTable(boosts) => {
                    value = match *member {
                        "acc" => ValueRefMut::I8(&mut boosts.acc),
                        "atk" => ValueRefMut::I8(&mut boosts.atk),
                        "def" => ValueRefMut::I8(&mut boosts.def),
                        "eva" => ValueRefMut::I8(&mut boosts.eva),
                        "spa" => ValueRefMut::I8(&mut boosts.spa),
                        "spd" => ValueRefMut::I8(&mut boosts.spd),
                        "spe" => ValueRefMut::I8(&mut boosts.spe),
                        _ => return Err(Self::bad_member_or_mutable_access(member, value_type)),
                    }
                }
                ValueRefMut::StatTable(stats) => {
                    value = match *member {
                        "atk" => ValueRefMut::U16(&mut stats.atk),
                        "def" => ValueRefMut::U16(&mut stats.def),
                        "spa" => ValueRefMut::U16(&mut stats.spa),
                        "spd" => ValueRefMut::U16(&mut stats.spd),
                        "spe" => ValueRefMut::U16(&mut stats.spe),
                        _ => return Err(Self::bad_member_or_mutable_access(member, value_type)),
                    }
                }
                ValueRefMut::EffectState(connector) => {
                    let context = unsafe { context.unsafely_detach_borrow_mut() };
                    value = ValueRefMut::from(
                        connector
                            .get_mut(context.battle_context_mut())?
                            .get_mut(*member),
                    );
                }
                ValueRefMut::TempEffectState(connector) => {
                    let context = unsafe { context.unsafely_detach_borrow_mut() };
                    value = ValueRefMut::from(
                        connector
                            .get_mut(context.battle_context_mut())?
                            .get_mut(*member),
                    );
                }
                ValueRefMut::Object(ref mut object) => {
                    // SAFETY: Mutably borrowing the object requires mutably borrowing this entire
                    // variable, so this can only happen once. If an object contains other objects,
                    // we are grabbing a different mutable borrow at each layer.
                    //
                    // When assigning to this mutable borrow, we take ownership of the assigning
                    // value first, so no operation will alter the object between grabbing this
                    // borrow and consuming it with an assignment.
                    let object = unsafe { object.unsafely_detach_borrow_mut() };
                    let entry = object
                        .entry((*member).to_owned())
                        .or_insert(Value::Undefined);
                    value = ValueRefMut::from(entry);
                }
                _ => return Err(Self::bad_member_or_mutable_access(member, value_type)),
            }
        }
        Ok(value)
    }

    fn get_mut<'var>(
        &'var mut self,
        context: &'eval mut EvaluationContext,
    ) -> Result<ValueRefMut<'var>> {
        self.get_ref_mut(context)
    }
}

/// Input variables to an fxlang program.
///
/// Values are assigned to a named variable based on the [`BattleEvent`] configuration.
#[derive(Clone, Default)]
pub struct VariableInput {
    values: Vec<Value>,
}

impl VariableInput {
    pub fn get(&self, index: usize) -> Option<&Value> {
        self.values.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut Value> {
        self.values.get_mut(index)
    }
}

impl FromIterator<Value> for VariableInput {
    fn from_iter<T: IntoIterator<Item = Value>>(iter: T) -> Self {
        Self {
            values: iter.into_iter().collect(),
        }
    }
}

impl IntoIterator for VariableInput {
    type Item = Value;
    type IntoIter = <Vec<Value> as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        self.values.into_iter()
    }
}

/// Context for executing a [`ParsedProgramBlock`] over a list.
///
/// The list itself must be evaluated once at the beginning of the loop.
struct ExecuteProgramBlockOverListContext<'program> {
    item: &'program str,
    list: &'program tree::Value,
}

impl<'eval, 'program> ExecuteProgramBlockOverListContext<'program> {
    fn new(item: &'program str, list: &'program tree::Value) -> Self {
        Self { item, list }
    }
}

/// The evaluation state of a [`ParsedProgramBlock`].
struct ProgramBlockEvalState<'program> {
    skip_next_block: bool,
    last_if_statement_result: Option<bool>,
    for_each_context: Option<ExecuteProgramBlockOverListContext<'program>>,
}

impl ProgramBlockEvalState<'_> {
    fn new() -> Self {
        Self {
            skip_next_block: false,
            last_if_statement_result: None,
            for_each_context: None,
        }
    }
}

/// The result of evaluating a [`ParsedProgramBlock`].
enum ProgramStatementEvalResult<'program> {
    None,
    Skipped,
    IfStatement(bool),
    ElseIfStatement(bool),
    ForEachStatement(&'program str, &'program tree::Value),
    ReturnStatement(Option<Value>),
    ContinueStatement,
}

/// The result of evaluating a [`ParsedProgram`].
#[derive(Default)]
pub struct ProgramEvalResult {
    pub value: Option<Value>,
}

impl ProgramEvalResult {
    pub fn new(value: Option<Value>) -> Self {
        Self { value }
    }
}

/// An fxlang evaluator.
///
/// Holds the global state of an fxlang [`ParsedProgram`] during evaluation. Individual blocks
/// ([`ParsedProgramBlock`]) are evaluated recursively and get their own local state.
pub struct Evaluator<'event_state> {
    statement: usize,
    vars: VariableRegistry,
    event: BattleEvent,
    event_state: &'event_state EventState,
}

impl<'event_state> Evaluator<'event_state> {
    /// Creates a new evaluator.
    pub fn new(event: BattleEvent, event_state: &'event_state EventState) -> Self {
        Self {
            statement: 0,
            vars: VariableRegistry::new(),
            event,
            event_state,
        }
    }

    fn initialize_vars(
        &self,
        context: &mut EvaluationContext,
        mut input: VariableInput,
        effect_state_connector: Option<DynamicEffectStateConnector>,
    ) -> Result<()> {
        if let Some(effect_state_connector) = effect_state_connector {
            if effect_state_connector.exists(context.battle_context_mut())? {
                self.vars
                    .set("effect_state", Value::EffectState(effect_state_connector))?;
            }
        }

        self.vars
            .set("this", Value::Effect(context.effect_handle().clone()))?;
        self.vars.set("battle", Value::Battle)?;
        self.vars.set("field", Value::Field)?;
        self.vars.set("format", Value::Format)?;

        if self.event.has_flag(CallbackFlag::TakesGeneralMon) {
            self.vars.set(
                "mon",
                Value::Mon(
                    context
                        .target_handle()
                        .wrap_expectation("context has no mon")?,
                ),
            )?;
        }
        if self.event.has_flag(CallbackFlag::TakesTargetMon) {
            match context.target_handle() {
                Some(target_handle) => self.vars.set("target", Value::Mon(target_handle))?,
                None => (),
            }
        }
        if self.event.has_flag(CallbackFlag::TakesSourceMon) {
            match context.source_handle() {
                Some(source_handle) => self.vars.set("source", Value::Mon(source_handle))?,
                None => (),
            }
        }
        if self.event.has_flag(CallbackFlag::TakesUserMon) {
            // The user is the target of the effect.
            self.vars.set(
                "user",
                Value::Mon(
                    context
                        .target_handle()
                        .wrap_expectation("context has no user")?,
                ),
            )?;
        }
        if self.event.has_flag(CallbackFlag::TakesSourceTargetMon) {
            // The target is the source of the effect.
            match context.source_handle() {
                Some(source_handle) => self.vars.set("target", Value::Mon(source_handle))?,
                None => (),
            }
        }
        if self
            .event
            .has_flag(CallbackFlag::TakesEffect | CallbackFlag::TakesSourceEffect)
        {
            let effect_name = if self.event.has_flag(CallbackFlag::TakesEffect) {
                "effect"
            } else if self.event.has_flag(CallbackFlag::TakesSourceEffect) {
                "source_effect"
            } else {
                unreachable!()
            };
            self.vars.set(
                effect_name,
                Value::Effect(
                    context
                        .source_effect_handle()
                        .cloned()
                        .wrap_expectation("context has no effect")?,
                ),
            )?;
        }
        if self.event.has_flag(CallbackFlag::TakesActiveMove) {
            self.vars.set(
                "move",
                Value::ActiveMove(
                    context
                        .source_active_move_handle()
                        .wrap_expectation("context has no active move")?,
                ),
            )?;
        }
        if self.event.has_flag(CallbackFlag::TakesOptionalEffect) {
            if let Some(source_effect_handle) = context.source_effect_handle().cloned() {
                self.vars
                    .set("effect", Value::Effect(source_effect_handle))?;
            }
        }
        if self.event.has_flag(CallbackFlag::TakesSide) {
            self.vars.set(
                "side",
                Value::Side(
                    context
                        .side_index()
                        .wrap_expectation("context has no side")?,
                ),
            )?;
        }
        if self.event.has_flag(CallbackFlag::TakesPlayer) {
            self.vars.set(
                "player",
                Value::Player(
                    context
                        .player_index()
                        .wrap_expectation("context has no player")?,
                ),
            )?;
        }

        // Reverse the input so we can efficiently pop elements out of it.
        input.values.reverse();
        for (i, (name, value_type, required)) in self.event.input_vars().iter().enumerate() {
            match input.values.pop() {
                None | Some(Value::Undefined) => {
                    if *required {
                        return Err(general_error(format!(
                            "missing {value_type} input at position {} for variable {name}",
                            i + 1,
                        )));
                    }
                }
                Some(value) => {
                    let real_value_type = value.value_type();
                    // Undefined means we do not enforce the type of the input variable.
                    let value = if *value_type == ValueType::Undefined {
                        value
                    } else {
                        value.convert_to(*value_type).wrap_error_with_format(format_args!("input at position {} for variable {name} of type {real_value_type} cannot be converted to {value_type}", i + 1))?
                    };
                    self.vars.set(name, value)?;
                }
            }
        }

        if !input.values.is_empty() {
            return Err(general_error(format!(
                "too many input values: found {} extra values",
                input.values.len(),
            )));
        }

        Ok(())
    }

    /// Evaluates the given program.
    pub fn evaluate_program(
        &mut self,
        context: &mut EvaluationContext,
        input: VariableInput,
        callback: &ParsedCallback,
        effect_state_connector: Option<DynamicEffectStateConnector>,
    ) -> Result<ProgramEvalResult> {
        self.initialize_vars(context, input, effect_state_connector)?;
        let root_state = ProgramBlockEvalState::new();
        let value = match self
            .evaluate_program_block(context, &callback.program.block, &root_state)
            .wrap_error_with_format(format_args!("error on statement {}", self.statement))?
        {
            ProgramStatementEvalResult::ReturnStatement(value) => value,
            _ => None,
        };
        if !self
            .event
            .output_type_allowed(value.as_ref().map(|val| val.value_type()))
        {
            match value {
                Some(val) => {
                    return Err(general_error(format!(
                        "{:?} cannot return a {}",
                        self.event,
                        val.value_type(),
                    )));
                }
                None => {
                    return Err(general_error(format!(
                        "{:?} must return a value",
                        self.event
                    )));
                }
            }
        }
        Ok(ProgramEvalResult::new(value))
    }

    fn evaluate_program_block<'eval, 'program>(
        &'eval mut self,
        context: &mut EvaluationContext,
        block: &'program ParsedProgramBlock,
        parent_state: &'eval ProgramBlockEvalState,
    ) -> Result<ProgramStatementEvalResult<'program>>
    where
        'program: 'eval,
    {
        match block {
            ParsedProgramBlock::Leaf(statement) => {
                self.evaluate_statement(context, statement, parent_state)
            }
            ParsedProgramBlock::Branch(blocks) => {
                if parent_state.skip_next_block {
                    self.statement += block.len() as usize;
                    return Ok(ProgramStatementEvalResult::Skipped);
                }

                if let Some(for_each_context) = &parent_state.for_each_context {
                    let list = self.resolve_value(context, for_each_context.list)?;
                    if !list.supports_list_iteration() {
                        return Err(general_error(format!(
                            "cannot iterate over a {}",
                            list.value_type()
                        )));
                    }
                    let len = list
                        .len()
                        .wrap_expectation("value supports iteration but is missing a length")?;
                    // SAFETY: We only use this immutable borrow at the beginning of each loop, at
                    // the start of each execution.
                    //
                    // This list value can only potentially contain a reference to a stored
                    // variable. If so, we are also storing the object that does runtime borrow
                    // checking, so borrow errors will trigger during evaluation.
                    let list: MaybeReferenceValue = unsafe { mem::transmute(list) };
                    for i in 0..len {
                        let current_item = list.list_index(i).wrap_expectation_with_format(format_args!(
                            "list has no element at index {i}, but length at beginning of foreach loop was {len}"
                        ))?.to_owned();
                        self.vars.set(for_each_context.item, current_item)?;
                        match self.evaluate_program_blocks_once(context, blocks.as_slice())? {
                            result @ ProgramStatementEvalResult::ReturnStatement(_) => {
                                // Early return.
                                return Ok(result);
                            }
                            ProgramStatementEvalResult::ContinueStatement => {
                                continue;
                            }
                            _ => (),
                        }
                    }

                    return Ok(ProgramStatementEvalResult::None);
                }

                self.evaluate_program_blocks_once(context, blocks.as_slice())
            }
        }
    }

    fn evaluate_program_blocks_once<'eval, 'program>(
        &'eval mut self,
        context: &mut EvaluationContext,
        blocks: &'program [ParsedProgramBlock],
    ) -> Result<ProgramStatementEvalResult<'program>>
    where
        'program: 'eval,
    {
        let mut state = ProgramBlockEvalState::new();
        for block in blocks {
            match self.evaluate_program_block(context, block, &state)? {
                result @ ProgramStatementEvalResult::ReturnStatement(_)
                | result @ ProgramStatementEvalResult::ContinueStatement => {
                    // Early return.
                    return Ok(result);
                }
                ProgramStatementEvalResult::None => {
                    // Reset the state.
                    state.last_if_statement_result = None;
                    state.skip_next_block = false;
                    state.for_each_context = None;
                }
                ProgramStatementEvalResult::Skipped => (),
                ProgramStatementEvalResult::IfStatement(condition_met) => {
                    state.for_each_context = None;
                    // Remember this result in case we find an associated else statement.
                    state.last_if_statement_result = Some(condition_met);
                    // Skip the next block if the condition was not met.
                    state.skip_next_block = !condition_met;
                }
                ProgramStatementEvalResult::ElseIfStatement(condition_met) => {
                    state.for_each_context = None;
                    // Only remember this result if we have evaluated an if statement before.
                    //
                    // This prevents else blocks from being run on their own, without a leading if
                    // statement.
                    if state.last_if_statement_result.is_some() {
                        state.last_if_statement_result = Some(condition_met);
                    }
                    // Skip the next block if the condition was not met.
                    //
                    // This will always be false if last_if_statement_result is true.
                    state.skip_next_block = !condition_met;
                }
                ProgramStatementEvalResult::ForEachStatement(item, list) => {
                    // Reset the state.
                    state.last_if_statement_result = None;
                    state.skip_next_block = false;
                    state.for_each_context = None;
                    // Prepare the context for the next block.
                    state.for_each_context =
                        Some(ExecuteProgramBlockOverListContext::new(item, list))
                }
            }
        }
        Ok(ProgramStatementEvalResult::None)
    }

    fn evaluate_statement<'eval, 'program>(
        &'eval mut self,
        context: &'eval mut EvaluationContext,
        statement: &'program tree::Statement,
        parent_state: &'eval ProgramBlockEvalState,
    ) -> Result<ProgramStatementEvalResult<'program>>
    where
        'program: 'eval,
    {
        self.statement += 1;
        match statement {
            tree::Statement::Empty => Ok(ProgramStatementEvalResult::None),
            tree::Statement::Assignment(assignment) => {
                let value = self.evaluate_expr(context, &assignment.rhs)?;
                // SAFETY: The value produced by the expression should be some newly generated
                // value. If it is a reference to the variable that is being assigned to, the
                // program evaluation will error out because the variable registry has runtime
                // borrow checking. Thus, we allow the context to be borrowed again.
                let value = unsafe { mem::transmute(value) };
                self.assign_var(context, &assignment.lhs, value)?;
                Ok(ProgramStatementEvalResult::None)
            }
            tree::Statement::FunctionCall(statement) => {
                self.evaluate_function_call(context, &statement)?;
                Ok(ProgramStatementEvalResult::None)
            }
            tree::Statement::IfStatement(statement) => Ok(ProgramStatementEvalResult::IfStatement(
                self.evaluate_if_statement(context, statement)?,
            )),
            tree::Statement::ElseIfStatement(statement) => {
                let condition_met = if let Some(false) = parent_state.last_if_statement_result {
                    // The last if statement was false, so this else block might apply.
                    if let Some(statement) = &statement.0 {
                        self.evaluate_if_statement(context, statement)?
                    } else {
                        true
                    }
                } else {
                    // The last if statement was true (or doesn't exist), so this else block does
                    // not apply and is not evaluated, even if there is a condition.
                    false
                };
                Ok(ProgramStatementEvalResult::ElseIfStatement(condition_met))
            }
            tree::Statement::ForEachStatement(statement) => {
                if !statement.var.member_access.is_empty() {
                    return Err(general_error(format!(
                        "invalid variable in foreach statement: ${}",
                        statement.var.full_name(),
                    )));
                }
                Ok(ProgramStatementEvalResult::ForEachStatement(
                    &statement.var.name.0,
                    &statement.range,
                ))
            }
            tree::Statement::ReturnStatement(statement) => {
                let value = match &statement.0 {
                    None => None,
                    Some(expr) => Some(self.evaluate_expr(context, expr)?),
                };
                Ok(ProgramStatementEvalResult::ReturnStatement(
                    value.map(|value| value.to_owned()),
                ))
            }
            tree::Statement::Continue(_) => Ok(ProgramStatementEvalResult::ContinueStatement),
        }
    }

    fn evaluate_if_statement<'eval, 'program>(
        &'eval self,
        context: &'eval mut EvaluationContext,
        statement: &'program tree::IfStatement,
    ) -> Result<bool> {
        let condition = self.evaluate_expr(context, &statement.0)?;
        let condition = match condition.boolean() {
            Some(value) => value,
            _ => {
                return Err(general_error(format!(
                    "if statement condition must return a boolean, got {}",
                    condition.value_type(),
                )));
            }
        };
        Ok(condition)
    }

    fn evaluate_function_call<'eval, 'program>(
        &'eval self,
        context: &'eval mut EvaluationContext,
        function_call: &'program tree::FunctionCall,
    ) -> Result<Option<MaybeReferenceValue<'eval>>>
    where
        'program: 'eval,
    {
        let args = self.resolve_values(context, &function_call.args)?;
        // Functions call code outside of the evaluator, so there can be no internal references.
        let args = args.into_iter().map(|arg| arg.to_owned()).collect();
        self.run_function(context, &function_call.function.0, args)
    }

    fn run_function<'eval, 'program>(
        &'eval self,
        context: &mut EvaluationContext,
        function_name: &'program str,
        args: VecDeque<Value>,
    ) -> Result<Option<MaybeReferenceValue<'eval>>> {
        let effect_state = self
            .vars
            .get("effect_state")?
            .map(|val| (*val).clone().effect_state().ok())
            .flatten();
        run_function(
            context,
            function_name,
            args,
            self.event,
            self.event_state,
            effect_state,
        )
        .map(|val| val.map(|val| MaybeReferenceValue::from(val)))
    }

    fn evaluate_prefix_operator<'eval>(
        op: tree::Operator,
        value: MaybeReferenceValueForOperation<'eval>,
    ) -> Result<MaybeReferenceValue<'eval>> {
        match op {
            tree::Operator::Not => value.negate(),
            tree::Operator::UnaryPlus => value.unary_plus(),
            _ => Err(general_error(format!("invalid prefix operator: {op}"))),
        }
    }

    fn evaluate_binary_operator<'eval>(
        lhs: MaybeReferenceValueForOperation<'eval>,
        op: tree::Operator,
        rhs: MaybeReferenceValueForOperation<'eval>,
    ) -> Result<MaybeReferenceValue<'eval>> {
        match op {
            tree::Operator::Exponent => lhs.pow(rhs),
            tree::Operator::Multiply => lhs.multiply(rhs),
            tree::Operator::Divide => lhs.divide(rhs),
            tree::Operator::Modulo => lhs.modulo(rhs),
            tree::Operator::Add => lhs.add(rhs),
            tree::Operator::Subtract => lhs.subtract(rhs),
            tree::Operator::LessThan => lhs.less_than(rhs),
            tree::Operator::LessThanOrEqual => lhs.less_than_or_equal(rhs),
            tree::Operator::GreaterThan => lhs.greater_than(rhs),
            tree::Operator::GreaterThanOrEqual => lhs.greater_than_or_equal(rhs),
            tree::Operator::Has => lhs.has(rhs),
            tree::Operator::HasAny => lhs.has_any(rhs),
            tree::Operator::Equal => lhs.equal(rhs),
            tree::Operator::NotEqual => lhs.not_equal(rhs),
            tree::Operator::And => lhs.and(rhs),
            tree::Operator::Or => lhs.or(rhs),
            _ => Err(general_error(format!("invalid binary operator: {op}"))),
        }
    }

    fn evaluate_expr<'eval, 'program>(
        &'eval self,
        context: &'eval mut EvaluationContext,
        expr: &'program tree::Expr,
    ) -> Result<MaybeReferenceValue<'eval>>
    where
        'program: 'eval,
    {
        match expr {
            tree::Expr::Value(value) => self.resolve_value(context, value),
            tree::Expr::PrefixUnaryExpr(prefix_expr) => {
                let mut value = self.evaluate_expr(context, prefix_expr.expr.as_ref())?;
                for op in &prefix_expr.ops {
                    let value_for_operation = MaybeReferenceValueForOperation::from(&value);
                    let result = Self::evaluate_prefix_operator(*op, value_for_operation)?;
                    // SAFETY: `value_for_operation` was consumed by `evaluate_prefix_operator`.
                    let result: MaybeReferenceValue<'eval> = unsafe { mem::transmute(result) };
                    value = result;
                }
                Ok(value)
            }
            tree::Expr::BinaryExpr(binary_expr) => {
                let value = self.evaluate_expr(context, binary_expr.lhs.as_ref())?;
                // SAFETY: `context` is not really borrowed mutably when we hold an immutable
                // reference to some value in the battle or evaluation state.
                let mut value: MaybeReferenceValue = unsafe { mem::transmute(value) };
                for rhs_expr in &binary_expr.rhs {
                    let lhs = MaybeReferenceValueForOperation::from(&value);

                    // Short-circuiting logic.
                    //
                    // Important for cases where we might check if a variable exists before
                    // accessing it.
                    match (&lhs, rhs_expr.op) {
                        (MaybeReferenceValueForOperation::Boolean(true), tree::Operator::Or) => {
                            value = MaybeReferenceValue::Boolean(true);
                            continue;
                        }
                        (MaybeReferenceValueForOperation::Boolean(false), tree::Operator::And) => {
                            value = MaybeReferenceValue::Boolean(false);
                            continue;
                        }
                        _ => (),
                    }

                    let rhs_value = self.evaluate_expr(context, rhs_expr.expr.as_ref())?;
                    let rhs = MaybeReferenceValueForOperation::from(&rhs_value);
                    let result = Self::evaluate_binary_operator(lhs, rhs_expr.op, rhs)?;
                    // SAFETY: Both `lhs` and `rhs` were consumed by `evaluate_binary_operator`.
                    let result: MaybeReferenceValue<'eval> = unsafe { mem::transmute(result) };
                    value = result;
                }
                Ok(value)
            }
        }
    }

    fn evaluate_formatted_string<'eval, 'program>(
        &'eval self,
        context: &'eval mut EvaluationContext,
        formatted_string: &'program tree::FormattedString,
    ) -> Result<MaybeReferenceValue<'eval>>
    where
        'program: 'eval,
    {
        let args = self.resolve_values(context, &formatted_string.args)?;
        let template = formatted_string.template.0.as_str();
        let mut string = String::new();
        string.reserve(template.len());

        let mut group = String::new();
        let mut group_start = None;
        let mut next_arg_index = 0;

        for (i, c) in template.char_indices() {
            match c {
                '{' => {
                    if i > 0 && group_start == Some(i - 1) {
                        // Two left brackets in a row result in an escape.
                        group_start = None;
                        string.push(c);
                    } else {
                        // Open a new group.
                        group_start = Some(i);
                    }
                }
                '}' if group_start.is_some() => {
                    if group.is_empty() {
                        // Use next positional argument.
                        let next_arg = args
                            .get(next_arg_index)
                            .wrap_expectation_with_format(format_args!("formatted string is missing positional argument for index {next_arg_index}"))?;
                        next_arg_index += 1;
                        group = MaybeReferenceValueForOperation::from(next_arg)
                            .for_formatted_string()?;
                    } else {
                        return Err(general_error(format!("invalid format group: {group}")));
                    }

                    // Add the replaced group to the string.
                    string.push_str(&group);

                    // Reset the state, since the group was closed.
                    group_start = None;
                    group.clear();
                }
                _ => {
                    if group_start.is_some() {
                        group.push(c);
                    } else {
                        string.push(c);
                    }
                }
            }
        }

        Ok(MaybeReferenceValue::String(string))
    }

    fn resolve_value<'eval, 'program>(
        &'eval self,
        context: &'eval mut EvaluationContext,
        value: &'program tree::Value,
    ) -> Result<MaybeReferenceValue<'eval>>
    where
        'program: 'eval,
    {
        match value {
            tree::Value::UndefinedLiteral => Ok(MaybeReferenceValue::Undefined),
            tree::Value::BoolLiteral(bool) => Ok(MaybeReferenceValue::Boolean(bool.0)),
            tree::Value::NumberLiteral(tree::NumberLiteral::Unsigned(number)) => {
                Ok(MaybeReferenceValue::UFraction(*number))
            }
            tree::Value::NumberLiteral(tree::NumberLiteral::Signed(number)) => {
                Ok(MaybeReferenceValue::Fraction(*number))
            }
            tree::Value::StringLiteral(string) => Ok(MaybeReferenceValue::String(string.0.clone())),
            tree::Value::List(list) => Ok(MaybeReferenceValue::List(
                self.resolve_values(context, &list.0)?,
            )),
            tree::Value::Var(var) => {
                let var = self.create_var(var)?;
                Ok(MaybeReferenceValue::from(var.get(context)?))
            }
            tree::Value::ValueExpr(expr) => Ok(MaybeReferenceValue::from(
                self.evaluate_expr(context, &expr.0)?,
            )),
            tree::Value::ValueFunctionCall(function_call) => {
                match self.evaluate_function_call(context, &function_call.0)? {
                    Some(value) => Ok(MaybeReferenceValue::from(value)),
                    None => Ok(MaybeReferenceValue::Undefined),
                }
            }
            tree::Value::FormattedString(formatted_string) => {
                self.evaluate_formatted_string(context, formatted_string)
            }
        }
    }

    fn resolve_values<'eval, 'program>(
        &'eval self,
        context: &'eval mut EvaluationContext,
        values: &'program tree::Values,
    ) -> Result<Vec<MaybeReferenceValue<'eval>>>
    where
        'program: 'eval,
    {
        let mut resolved = Vec::new();
        for value in &values.0 {
            // SAFETY: It is safe to have an immutable reference into the battle state. The
            // context is not really borrowed mutably.
            let value = self.resolve_value(context, value)?;
            let value: MaybeReferenceValue<'eval> = unsafe { mem::transmute(value) };
            resolved.push(value);
        }
        Ok(resolved)
    }

    fn assign_var<'eval, 'program>(
        &'eval self,
        context: &mut EvaluationContext,
        var: &'program tree::Var,
        value: MaybeReferenceValue<'eval>,
    ) -> Result<()> {
        // Drop the reference as soon as possible, because holding it might block a mutable
        // reference to what we want to assign to.
        //
        // For instance, assigning one property of an object to another property on the same object
        // results in a borrow error without this drop.
        let owned_value = value.to_owned();
        drop(value);

        let mut runtime_var = self.create_var_mut(var)?;
        let runtime_var_ref = runtime_var.get_mut(context)?;

        runtime_var_ref
            .assign(owned_value)
            .wrap_error_with_format(format_args!("failed to assign to ${}", var.full_name()))
    }

    fn create_var<'eval, 'program>(
        &'eval self,
        var: &'program tree::Var,
    ) -> Result<Variable<'eval, 'program>>
    where
        'program: 'eval,
    {
        let value = self.vars.get(&var.name.0)?;
        let member_access = var
            .member_access
            .iter()
            .map(|ident| ident.0.as_str())
            .collect();
        Ok(Variable::new(value, member_access))
    }

    fn create_var_mut<'eval, 'program>(
        &'eval self,
        var: &'program tree::Var,
    ) -> Result<VariableMut<'eval, 'program>>
    where
        'program: 'eval,
    {
        let value = match self.vars.get_mut(&var.name.0)? {
            None => {
                self.vars.set(&var.name.0, Value::Undefined)?;
                self.vars
                    .get_mut(&var.name.0)?
                    .wrap_expectation_with_format(format_args!(
                        "variable ${} is undefined even after initialization",
                        var.name.0
                    ))?
            }
            Some(value) => value,
        };
        let member_access = var
            .member_access
            .iter()
            .map(|ident| ident.0.as_str())
            .collect();
        Ok(VariableMut::new(value, member_access))
    }
}
