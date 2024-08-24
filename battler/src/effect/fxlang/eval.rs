use std::{
    collections::VecDeque,
    mem,
    str::FromStr,
};

use zone_alloc::{
    BorrowError,
    ElementRef,
    ElementRefMut,
    KeyedRegistry,
};

use crate::{
    battle::{
        mon_states,
        weather_states,
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
        MoveHandle,
        Player,
        SideEffectContext,
    },
    battler_error,
    common::{
        Error,
        Fraction,
        Identifiable,
        MaybeOwnedMut,
        UnsafelyDetachBorrowMut,
        WrapResultError,
    },
    effect::{
        fxlang::{
            run_function,
            tree,
            BattleEvent,
            CallbackFlag,
            DynamicEffectStateConnector,
            EffectStateConnector,
            MaybeReferenceValue,
            MaybeReferenceValueForOperation,
            ParsedProgram,
            ParsedProgramBlock,
            Value,
            ValueRef,
            ValueRefMut,
            ValueRefToStoredValue,
            ValueType,
        },
        EffectHandle,
        MonStatusEffectStateConnector,
    },
    moves::{
        Accuracy,
        Move,
        MoveTarget,
        MultihitType,
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
    SideEffect(SideEffectContext<'effect, 'context, 'battle, 'data>),
    FieldEffect(FieldEffectContext<'effect, 'context, 'battle, 'data>),
}

impl<'effect, 'context, 'battle, 'data> EvaluationContext<'effect, 'context, 'battle, 'data> {
    pub fn battle_context<'eval>(&'eval self) -> &'eval Context<'battle, 'data> {
        match self {
            Self::ApplyingEffect(context) => context.as_battle_context(),
            Self::Effect(context) => context.as_battle_context(),
            Self::SideEffect(context) => context.as_battle_context(),
            Self::FieldEffect(context) => context.as_battle_context(),
        }
    }

    pub fn battle_context_mut<'eval>(&'eval mut self) -> &'eval mut Context<'battle, 'data> {
        match self {
            Self::ApplyingEffect(context) => context.as_battle_context_mut(),
            Self::Effect(context) => context.as_battle_context_mut(),
            Self::SideEffect(context) => context.as_battle_context_mut(),
            Self::FieldEffect(context) => context.as_battle_context_mut(),
        }
    }

    pub fn effect_context<'eval>(&'eval self) -> &'eval EffectContext<'context, 'battle, 'data> {
        match self {
            Self::ApplyingEffect(context) => context.as_effect_context(),
            Self::Effect(context) => context,
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
            Self::SideEffect(context) => context.as_effect_context_mut(),
            Self::FieldEffect(context) => context.as_effect_context_mut(),
        }
    }

    pub fn source_effect_context<'eval>(
        &'eval mut self,
    ) -> Result<Option<EffectContext<'eval, 'battle, 'data>>, Error> {
        self.effect_context_mut().source_effect_context()
    }

    pub fn forward_effect_to_applying_effect<'eval>(
        &'eval mut self,
        target_handle: MonHandle,
        use_target_as_source: bool,
    ) -> Result<ApplyingEffectContext<'eval, 'eval, 'battle, 'data>, Error> {
        let source_handle = if use_target_as_source {
            self.target_handle()
        } else {
            self.source_handle()
        };
        let context: ApplyingEffectContext<'eval, 'context, 'battle, 'data> = self
            .effect_context_mut()
            .applying_effect_context(source_handle, target_handle)?;
        // SAFETY: 'context lives longer than 'eval, so we are shortening the lifetime of this
        // context.
        let context: ApplyingEffectContext<'eval, 'eval, 'battle, 'data> =
            unsafe { mem::transmute(context) };
        Ok(context)
    }

    pub fn forward_source_effect_to_applying_effect<'eval>(
        &'eval mut self,
        target_handle: MonHandle,
    ) -> Result<ApplyingEffectContext<'eval, 'eval, 'battle, 'data>, Error> {
        let source_handle = self.source_handle();
        let source_effect = self
            .source_effect_handle()
            .wrap_error_with_message("context has no source effect")?
            .clone();
        self.battle_context_mut().applying_effect_context(
            source_effect,
            source_handle,
            target_handle,
            None,
        )
    }

    pub fn forward_effect_to_side_effect<'eval>(
        &'eval mut self,
        side: usize,
        use_target_as_source: bool,
    ) -> Result<SideEffectContext<'eval, 'eval, 'battle, 'data>, Error> {
        let source_handle = if use_target_as_source {
            self.target_handle()
        } else {
            self.source_handle()
        };
        let context: SideEffectContext<'eval, 'context, 'battle, 'data> = self
            .effect_context_mut()
            .side_effect_context(side, source_handle)?;
        // SAFETY: 'context lives longer than 'eval, so we are shortening the lifetime of this
        // context.
        let context: SideEffectContext<'eval, 'eval, 'battle, 'data> =
            unsafe { mem::transmute(context) };
        Ok(context)
    }

    pub fn forward_source_effect_to_side_effect<'eval>(
        &'eval mut self,
        side: usize,
    ) -> Result<SideEffectContext<'eval, 'eval, 'battle, 'data>, Error> {
        let source_handle = self.source_handle();
        let source_effect = self
            .source_effect_handle()
            .wrap_error_with_message("context has no source effect")?
            .clone();
        self.battle_context_mut()
            .side_effect_context(source_effect, side, source_handle, None)
    }

    pub fn forward_effect_to_field_effect<'eval>(
        &'eval mut self,
        use_target_as_source: bool,
    ) -> Result<FieldEffectContext<'eval, 'eval, 'battle, 'data>, Error> {
        let source_handle = if use_target_as_source {
            self.target_handle()
        } else {
            self.source_handle()
        };
        let context: FieldEffectContext<'eval, 'context, 'battle, 'data> = self
            .effect_context_mut()
            .field_effect_context(source_handle)?;
        // SAFETY: 'context lives longer than 'eval, so we are shortening the lifetime of this
        // context.
        let context: FieldEffectContext<'eval, 'eval, 'battle, 'data> =
            unsafe { mem::transmute(context) };
        Ok(context)
    }

    pub fn forward_source_effect_to_field_effect<'eval>(
        &'eval mut self,
    ) -> Result<FieldEffectContext<'eval, 'eval, 'battle, 'data>, Error> {
        let source_handle = self.source_handle();
        let source_effect = self
            .source_effect_handle()
            .wrap_error_with_message("context has no source effect")?
            .clone();
        self.battle_context_mut()
            .field_effect_context(source_effect, source_handle, None)
    }

    pub fn applying_effect_context<'eval>(
        &'eval self,
    ) -> Result<&'eval ApplyingEffectContext<'effect, 'context, 'battle, 'data>, Error> {
        match self {
            Self::ApplyingEffect(context) => Ok(context),
            _ => Err(battler_error!("context is not an applying effect")),
        }
    }

    pub fn applying_effect_context_mut<'eval>(
        &'eval mut self,
    ) -> Result<&'eval mut ApplyingEffectContext<'effect, 'context, 'battle, 'data>, Error> {
        match self {
            Self::ApplyingEffect(context) => Ok(context),
            _ => Err(battler_error!("context is not an applying effect")),
        }
    }

    pub fn source_applying_effect_context<'eval>(
        &'eval mut self,
    ) -> Result<Option<ApplyingEffectContext<'eval, 'eval, 'battle, 'data>>, Error> {
        match self {
            Self::ApplyingEffect(context) => context.source_applying_effect_context(),
            _ => Err(battler_error!("context is not an applying effect")),
        }
    }

    pub fn source_active_move_context<'eval>(
        &'eval mut self,
    ) -> Result<Option<ActiveMoveContext<'eval, 'eval, 'eval, 'eval, 'battle, 'data>>, Error> {
        self.effect_context_mut().source_active_move_context()
    }

    pub fn target_context<'eval>(
        &'eval mut self,
    ) -> Result<MonContext<'eval, 'eval, 'eval, 'battle, 'data>, Error> {
        match self {
            Self::ApplyingEffect(context) => context.target_context(),
            _ => Err(battler_error!("effect cannot have a target")),
        }
    }

    pub fn source_context<'eval>(
        &'eval mut self,
    ) -> Result<Option<MonContext<'eval, 'eval, 'eval, 'battle, 'data>>, Error> {
        match self {
            Self::ApplyingEffect(context) => context.source_context(),
            Self::SideEffect(context) => context.source_context(),
            Self::FieldEffect(context) => context.source_context(),
            _ => Err(battler_error!("effect cannot have a source")),
        }
    }

    pub fn mon_context<'eval>(
        &'eval mut self,
        mon_handle: MonHandle,
    ) -> Result<MonContext<'eval, 'eval, 'eval, 'battle, 'data>, Error> {
        match self {
            Self::ApplyingEffect(context) => {
                if context
                    .source_handle()
                    .is_some_and(|source_handle| source_handle == mon_handle)
                {
                    context
                        .source_context()?
                        .wrap_error_with_message("expected source mon")
                } else if mon_handle == context.target_handle() {
                    context.target_context()
                } else {
                    context.as_battle_context_mut().mon_context(mon_handle)
                }
            }
            Self::Effect(context) => context.as_battle_context_mut().mon_context(mon_handle),
            Self::SideEffect(context) => {
                if context
                    .source_handle()
                    .is_some_and(|source_handle| source_handle == mon_handle)
                {
                    context
                        .source_context()?
                        .wrap_error_with_message("expected source mon")
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
                        .wrap_error_with_message("expected source mon")
                } else {
                    context.as_battle_context_mut().mon_context(mon_handle)
                }
            }
        }
    }

    pub fn mon<'eval>(&'eval self, mon_handle: MonHandle) -> Result<&'eval Mon, Error> {
        match self {
            Self::ApplyingEffect(context) => {
                if context
                    .source_handle()
                    .is_some_and(|source_handle| source_handle == mon_handle)
                {
                    context
                        .source()
                        .wrap_error_with_message("expected source mon")
                } else if mon_handle == context.target_handle() {
                    Ok(context.target())
                } else {
                    context.as_battle_context().mon(mon_handle)
                }
            }
            Self::Effect(context) => context.as_battle_context().mon(mon_handle),
            Self::SideEffect(context) => {
                if context
                    .source_handle()
                    .is_some_and(|source_handle| source_handle == mon_handle)
                {
                    context
                        .source()
                        .wrap_error_with_message("expected source mon")
                } else {
                    context.as_battle_context().mon(mon_handle)
                }
            }
            Self::FieldEffect(context) => {
                if context
                    .source_handle()
                    .is_some_and(|source_handle| source_handle == mon_handle)
                {
                    context
                        .source()
                        .wrap_error_with_message("expected source mon")
                } else {
                    context.as_battle_context().mon(mon_handle)
                }
            }
        }
    }

    fn mon_mut<'eval>(&'eval mut self, mon_handle: MonHandle) -> Result<&'eval mut Mon, Error> {
        match self {
            Self::ApplyingEffect(context) => {
                if context
                    .source_handle()
                    .is_some_and(|source_handle| source_handle == mon_handle)
                {
                    context
                        .source_mut()
                        .wrap_error_with_message("expected source mon")
                } else if mon_handle == context.target_handle() {
                    Ok(context.target_mut())
                } else {
                    context.as_battle_context_mut().mon_mut(mon_handle)
                }
            }
            Self::Effect(context) => context.as_battle_context_mut().mon_mut(mon_handle),
            Self::SideEffect(context) => {
                if context
                    .source_handle()
                    .is_some_and(|source_handle| source_handle == mon_handle)
                {
                    context
                        .source_mut()
                        .wrap_error_with_message("expected source mon")
                } else {
                    context.as_battle_context_mut().mon_mut(mon_handle)
                }
            }
            Self::FieldEffect(context) => {
                if context
                    .source_handle()
                    .is_some_and(|source_handle| source_handle == mon_handle)
                {
                    context
                        .source_mut()
                        .wrap_error_with_message("expected source mon")
                } else {
                    context.as_battle_context_mut().mon_mut(mon_handle)
                }
            }
        }
    }

    pub fn effect_context_for_handle<'eval>(
        &'eval mut self,
        effect_handle: &EffectHandle,
    ) -> Result<MaybeOwnedMut<'eval, EffectContext<'eval, 'battle, 'data>>, Error> {
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

    pub fn active_move<'eval>(
        &'eval self,
        active_move_handle: MoveHandle,
    ) -> Result<&'eval Move, Error> {
        self.battle_context().active_move(active_move_handle)
    }

    fn active_move_mut<'eval>(
        &'eval mut self,
        active_move_handle: MoveHandle,
    ) -> Result<&'eval mut Move, Error> {
        self.battle_context_mut()
            .active_move_mut(active_move_handle)
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
            Self::SideEffect(context) => context.source_handle(),
            Self::FieldEffect(context) => context.source_handle(),
            _ => None,
        }
    }

    pub fn effect_handle(&self) -> &EffectHandle {
        match self {
            Self::ApplyingEffect(context) => context.effect_handle(),
            Self::Effect(context) => context.effect_handle(),
            Self::SideEffect(context) => context.effect_handle(),
            Self::FieldEffect(context) => context.effect_handle(),
        }
    }

    pub fn source_effect_handle(&self) -> Option<&EffectHandle> {
        match self {
            Self::ApplyingEffect(context) => context.source_effect_handle(),
            Self::Effect(context) => context.source_effect_handle(),
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
            Self::SideEffect(context) => Some(context.side().index),
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

    fn get(&self, var: &str) -> Result<Option<ElementRef<Value>>, Error> {
        match self.vars.get(var) {
            Ok(val) => Ok(Some(val)),
            Err(BorrowError::OutOfBounds) => Ok(None),
            Err(_) => Err(battler_error!("failed to borrow variable ${var}")),
        }
    }

    fn get_mut(&self, var: &str) -> Result<Option<ElementRefMut<Value>>, Error> {
        match self.vars.get_mut(var) {
            Ok(val) => Ok(Some(val)),
            Err(BorrowError::OutOfBounds) => Ok(None),
            Err(_) => Err(battler_error!("failed to borrow variable ${var}")),
        }
    }

    fn set(&self, var: &str, value: Value) -> Result<(), Error> {
        match self.vars.get_mut(var) {
            Ok(mut var) => {
                *var = value;
                Ok(())
            }
            Err(BorrowError::OutOfBounds) => {
                self.vars.register(var.to_owned(), value);
                Ok(())
            }
            Err(_) => Err(battler_error!("failed to mutably borrow variable ${var}")),
        }
    }
}

/// An fxlang variable.
///
/// Acts as a wrapper for an immutale access of a variable that can be consumed at some later time.
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
        battler_error!("value of type {value_type} has no member {member}")
    }

    fn get_ref<'var>(
        &'var self,
        context: &'eval mut EvaluationContext,
    ) -> Result<ValueRef<'var>, Error> {
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
                                .wrap_error_with_message("integer overflow")?
                                .into(),
                        ),
                        None => ValueRef::Undefined,
                    }
                }
                _ => {
                    if let Some(mon_handle) = value.mon_handle() {
                        let context = unsafe { context.unsafely_detach_borrow_mut() };
                        value = match *member {
                            "active" => ValueRef::Boolean(context.mon(mon_handle)?.active),
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
                            "boosts" => ValueRef::BoostTable(&context.mon(mon_handle)?.boosts),
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
                            "fainted" => ValueRef::Boolean(context.mon(mon_handle)?.fainted),
                            "foe_side" => {
                                ValueRef::Side(context.mon_context(mon_handle)?.foe_side().index)
                            }
                            "gender" => ValueRef::Gender(context.mon(mon_handle)?.gender),
                            "happiness" => {
                                ValueRef::UFraction(context.mon(mon_handle)?.happiness.into())
                            }
                            "hidden_power_type" => {
                                ValueRef::Type(context.mon(mon_handle)?.hidden_power_type)
                            }
                            "hp" => ValueRef::UFraction(context.mon(mon_handle)?.hp.into()),
                            "is_asleep" => ValueRef::Boolean(mon_states::is_asleep(
                                &mut context.mon_context(mon_handle)?,
                            )),
                            "is_grounded" => ValueRef::Boolean(mon_states::is_grounded(
                                &mut context.mon_context(mon_handle)?,
                            )),
                            "is_sky_dropped" => ValueRef::Boolean(mon_states::is_sky_dropped(
                                &mut context.mon_context(mon_handle)?,
                            )),
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
                                            .wrap_error_with_message("integer overflow")?
                                            .into(),
                                    ),
                                    None => ValueRef::Undefined,
                                }
                            }
                            "level" => ValueRef::UFraction(context.mon(mon_handle)?.level.into()),
                            "max_hp" => ValueRef::UFraction(context.mon(mon_handle)?.max_hp.into()),
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
                            "needs_switch" => {
                                ValueRef::Boolean(context.mon(mon_handle)?.needs_switch.is_some())
                            }
                            "newly_switched" => {
                                ValueRef::Boolean(context.mon(mon_handle)?.newly_switched)
                            }
                            "player" => ValueRef::Player(context.mon(mon_handle)?.player),
                            "position" => ValueRef::UFraction(
                                TryInto::<u32>::try_into(Mon::position_on_side(
                                    &context.mon_context(mon_handle)?,
                                )?)
                                .wrap_error_with_message("integer overflow")?
                                .into(),
                            ),
                            "position_details" => ValueRef::TempString(format!(
                                "{}",
                                Mon::position_details(&context.mon_context(mon_handle)?)?
                            )),
                            "side" => ValueRef::Side(context.mon(mon_handle)?.side),
                            "status" => match context.mon(mon_handle)?.status.as_ref() {
                                Some(status) => ValueRef::TempString(status.as_ref().to_owned()),
                                None => ValueRef::Undefined,
                            },
                            "status_state" => ValueRef::EffectState(
                                MonStatusEffectStateConnector::new(mon_handle).make_dynamic(),
                            ),
                            "transformed" => {
                                ValueRef::Boolean(context.mon(mon_handle)?.transformed)
                            }
                            "weight" => ValueRef::UFraction(
                                Mon::get_weight(&mut context.mon_context(mon_handle)?).into(),
                            ),
                            "will_move_this_turn" => ValueRef::Boolean(
                                context
                                    .battle_context()
                                    .battle()
                                    .queue
                                    .will_move_this_turn(mon_handle),
                            ),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let Some(effect_handle) = value.effect_handle() {
                        let context = unsafe { context.unsafely_detach_borrow_mut() };
                        value = match *member {
                            "category" => ValueRef::MoveCategory(
                                CoreBattle::get_effect_by_handle(
                                    context.battle_context(),
                                    &effect_handle,
                                )?
                                .move_effect()
                                .wrap_error_with_message("effect is not a move")?
                                .data
                                .category,
                            ),
                            "condition" => ValueRef::TempEffect(
                                effect_handle
                                    .condition_handle(context.battle_context())?
                                    .wrap_error_with_message(
                                        "effect has no associated condition",
                                    )?,
                            ),
                            "has_source_effect" => ValueRef::Boolean(
                                CoreBattle::get_effect_by_handle(
                                    context.battle_context(),
                                    &effect_handle,
                                )?
                                .source_effect_handle()
                                .is_some(),
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
                            "infiltrates" => ValueRef::Boolean(
                                CoreBattle::get_effect_by_handle(
                                    context.battle_context(),
                                    &effect_handle,
                                )?
                                .infiltrates(),
                            ),
                            "is_ability" => ValueRef::Boolean(effect_handle.is_ability()),
                            "is_move" => ValueRef::Boolean(effect_handle.is_active_move()),
                            "is_raining" => ValueRef::Boolean(weather_states::is_raining(
                                context.effect_context_for_handle(effect_handle)?.as_mut(),
                            )),
                            "is_sunny" => ValueRef::Boolean(weather_states::is_sunny(
                                context.effect_context_for_handle(effect_handle)?.as_mut(),
                            )),
                            "move_target" => ValueRef::MoveTarget(
                                CoreBattle::get_effect_by_handle(
                                    context.battle_context(),
                                    &effect_handle,
                                )?
                                .move_effect()
                                .wrap_error_with_message("effect is not a move")?
                                .data
                                .target,
                            ),
                            "name" => ValueRef::TempString(
                                CoreBattle::get_effect_by_handle(
                                    context.battle_context(),
                                    &effect_handle,
                                )?
                                .name()
                                .to_owned(),
                            ),
                            "type" => ValueRef::Type(
                                CoreBattle::get_effect_by_handle(
                                    context.battle_context(),
                                    &effect_handle,
                                )?
                                .move_effect()
                                .wrap_error_with_message("effect is not a move")?
                                .data
                                .primary_type,
                            ),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let Some(active_move_handle) = value.active_move_handle() {
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
                            "id" => ValueRef::Str(
                                context.active_move(active_move_handle)?.id().as_ref(),
                            ),
                            "infiltrates" => ValueRef::Boolean(
                                context.active_move(active_move_handle)?.infiltrates,
                            ),
                            "name" => ValueRef::Str(
                                context.active_move(active_move_handle)?.data.name.as_ref(),
                            ),
                            "non_ghost_target" => {
                                let active_move = context.active_move(active_move_handle)?;
                                ValueRef::MoveTarget(
                                    active_move
                                        .data
                                        .non_ghost_target
                                        .unwrap_or(active_move.data.target),
                                )
                            }
                            "ohko" => ValueRef::Boolean(
                                context
                                    .active_move(active_move_handle)?
                                    .data
                                    .ohko_type
                                    .is_some(),
                            ),
                            "recoil_percent" => ValueRef::UFraction(
                                context
                                    .active_move(active_move_handle)?
                                    .data
                                    .recoil_percent
                                    .unwrap_or(Fraction::from(0u16))
                                    .convert(),
                            ),
                            "target" => ValueRef::MoveTarget(
                                context.active_move(active_move_handle)?.data.target,
                            ),
                            "thaws_target" => ValueRef::Boolean(
                                context.active_move(active_move_handle)?.data.thaws_target,
                            ),
                            "type" => ValueRef::Type(
                                context.active_move(active_move_handle)?.data.primary_type,
                            ),
                            "user_effect" => context
                                .active_move(active_move_handle)?
                                .data
                                .user_effect
                                .as_ref()
                                .map(ValueRef::HitEffect)
                                .unwrap_or(ValueRef::Undefined),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::Player(player) = value {
                        value = match *member {
                            "can_escape" => ValueRef::Boolean(Player::can_escape(
                                &context.battle_context_mut().player_context(player)?,
                            )),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::MoveSlot(move_slot) = value {
                        value = match *member {
                            "id" => ValueRef::Str(move_slot.id.as_ref()),
                            "pp" => ValueRef::UFraction(move_slot.pp.into()),
                            _ => return Err(Self::bad_member_access(member, value_type)),
                        }
                    } else if let ValueRef::Field = value {
                        let context = unsafe { context.unsafely_detach_borrow_mut() };
                        value = match *member {
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
                                .wrap_error_with_message("integer overflow")?
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

    fn get(
        self,
        context: &'eval mut EvaluationContext,
    ) -> Result<ValueRefToStoredValue<'eval>, Error> {
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
        battler_error!(
            "value of type {value_type} has no member {member} or the member is immutable"
        )
    }

    fn get_ref_mut<'var>(
        &'var mut self,
        context: &'eval mut EvaluationContext,
    ) -> Result<ValueRefMut<'var>, Error> {
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
                        "last_target_location" => ValueRefMut::OptionalISize(
                            &mut context.mon_mut(**mon_handle)?.last_move_target_location,
                        ),
                        "skip_before_switch_out" => ValueRefMut::Boolean(
                            &mut context.mon_mut(**mon_handle)?.skip_before_switch_out,
                        ),
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
                        "hit_effect" => ValueRefMut::OptionalHitEffect(
                            &mut context
                                .active_move_mut(**active_move_handle)?
                                .data
                                .hit_effect,
                        ),
                        "ignore_immunity" => ValueRefMut::OptionalBoolean(
                            &mut context
                                .active_move_mut(**active_move_handle)?
                                .data
                                .ignore_immunity,
                        ),
                        "infiltrates" => ValueRefMut::Boolean(
                            &mut context.active_move_mut(**active_move_handle)?.infiltrates,
                        ),
                        "multihit" => ValueRefMut::OptionalMultihitType(
                            &mut context.active_move_mut(**active_move_handle)?.data.multihit,
                        ),
                        "target" => ValueRefMut::MoveTarget(
                            &mut context.active_move_mut(**active_move_handle)?.data.target,
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
                        "volatile_status" => {
                            ValueRefMut::OptionalString(&mut hit_effect.volatile_status)
                        }
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
    ) -> Result<ValueRefMut<'var>, Error> {
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
pub struct Evaluator {
    statement: usize,
    vars: VariableRegistry,
}

impl Evaluator {
    /// Creates a new evaluator.
    pub fn new() -> Self {
        Self {
            statement: 0,
            vars: VariableRegistry::new(),
        }
    }

    fn initialize_vars(
        &self,
        context: &mut EvaluationContext,
        event: BattleEvent,
        mut input: VariableInput,
        effect_state_connector: Option<DynamicEffectStateConnector>,
    ) -> Result<(), Error> {
        if let Some(effect_state_connector) = effect_state_connector {
            if effect_state_connector.exists(context.battle_context_mut())? {
                self.vars
                    .set("effect_state", Value::EffectState(effect_state_connector))?;
            }
        }

        self.vars
            .set("this", Value::Effect(context.effect_handle().clone()))?;
        self.vars.set("field", Value::Field)?;
        self.vars.set("format", Value::Format)?;

        if event.has_flag(CallbackFlag::TakesGeneralMon) {
            self.vars.set(
                "mon",
                Value::Mon(
                    context
                        .target_handle()
                        .wrap_error_with_message("context has no mon")?,
                ),
            )?;
        }
        if event.has_flag(CallbackFlag::TakesTargetMon) {
            match context.target_handle() {
                Some(target_handle) => self.vars.set("target", Value::Mon(target_handle))?,
                None => (),
            }
        }
        if event.has_flag(CallbackFlag::TakesSourceMon) {
            match context.source_handle() {
                Some(source_handle) => self.vars.set("source", Value::Mon(source_handle))?,
                None => (),
            }
        }
        if event.has_flag(CallbackFlag::TakesUserMon) {
            // The user is the target of the effect.
            self.vars.set(
                "user",
                Value::Mon(
                    context
                        .target_handle()
                        .wrap_error_with_message("context has no user")?,
                ),
            )?;
        }
        if event.has_flag(CallbackFlag::TakesSourceTargetMon) {
            // The target is the source of the effect.
            match context.source_handle() {
                Some(source_handle) => self.vars.set("target", Value::Mon(source_handle))?,
                None => (),
            }
        }
        if event.has_flag(CallbackFlag::TakesEffect) {
            self.vars.set(
                "effect",
                Value::Effect(
                    context
                        .source_effect_handle()
                        .cloned()
                        .wrap_error_with_message("context has no effect")?,
                ),
            )?;
        }
        if event.has_flag(CallbackFlag::TakesOptionalEffect) {
            if let Some(source_effect_handle) = context.source_effect_handle().cloned() {
                self.vars
                    .set("effect", Value::Effect(source_effect_handle))?;
            }
        }
        if event.has_flag(CallbackFlag::TakesSourceEffect) {
            self.vars.set(
                "source_effect",
                Value::Effect(
                    context
                        .source_effect_handle()
                        .cloned()
                        .wrap_error_with_message("context has no source effect")?,
                ),
            )?;
        }
        if event.has_flag(CallbackFlag::TakesActiveMove) {
            self.vars.set(
                "move",
                Value::ActiveMove(
                    context
                        .source_active_move_handle()
                        .wrap_error_with_message("context has no active move")?,
                ),
            )?;
        }
        if event.has_flag(CallbackFlag::TakesSide) {
            self.vars.set(
                "side",
                Value::Side(
                    context
                        .side_index()
                        .wrap_error_with_message("context has no side")?,
                ),
            )?;
        }

        // Reverse the input so we can efficiently pop elements out of it.
        input.values.reverse();
        for (i, (name, value_type, required)) in event.input_vars().iter().enumerate() {
            match input.values.pop() {
                None | Some(Value::Undefined) => {
                    if *required {
                        return Err(battler_error!(
                            "missing {value_type} input at position {} for variable {name}",
                            i + 1
                        ));
                    }
                }
                Some(value) => {
                    let real_value_type = value.value_type();
                    let value = value.convert_to(*value_type).wrap_error_with_format(format_args!("input at position {} for variable {name} of type {real_value_type} cannot be converted to {value_type}", i + 1))?;
                    self.vars.set(name, value)?;
                }
            }
        }

        if !input.values.is_empty() {
            return Err(battler_error!(
                "too many input values: found {} extra values",
                input.values.len()
            ));
        }

        Ok(())
    }

    /// Evaluates the given program.
    pub fn evaluate_program(
        &mut self,
        context: &mut EvaluationContext,
        event: BattleEvent,
        input: VariableInput,
        program: &ParsedProgram,
        effect_state_connector: Option<DynamicEffectStateConnector>,
    ) -> Result<ProgramEvalResult, Error> {
        self.initialize_vars(context, event, input, effect_state_connector)?;
        let root_state = ProgramBlockEvalState::new();
        let value = match self
            .evaluate_program_block(context, &program.block, &root_state)
            .wrap_error_with_format(format_args!("error on statement {}", self.statement))?
        {
            ProgramStatementEvalResult::ReturnStatement(value) => value,
            _ => None,
        };
        if !event.output_type_allowed(value.as_ref().map(|val| val.value_type())) {
            match value {
                Some(val) => {
                    return Err(battler_error!(
                        "{event:?} cannot return a {}",
                        val.value_type()
                    ))
                }
                None => return Err(battler_error!("{event:?} must return a value")),
            }
        }
        Ok(ProgramEvalResult::new(value))
    }

    fn evaluate_program_block<'eval, 'program>(
        &'eval mut self,
        context: &mut EvaluationContext,
        block: &'program ParsedProgramBlock,
        parent_state: &'eval ProgramBlockEvalState,
    ) -> Result<ProgramStatementEvalResult<'program>, Error>
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
                        return Err(battler_error!(
                            "cannot iterate over a {}",
                            list.value_type()
                        ));
                    }
                    let len = list.len().wrap_error_with_message(
                        "value supports iteration but is missing a length",
                    )?;
                    // SAFETY: We only use this immutable borrow at the beginning of each loop, at
                    // the start of each execution.
                    //
                    // This list value can only potentially contian a reference to a stored
                    // variable. If so, we are also storing the object that does runtime borrow
                    // checking, so borrow errors will trigger during evaluation.
                    let list: MaybeReferenceValue = unsafe { mem::transmute(list) };
                    for i in 0..len {
                        let current_item = list.list_index(i).wrap_error_with_format(format_args!(
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
    ) -> Result<ProgramStatementEvalResult<'program>, Error>
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
    ) -> Result<ProgramStatementEvalResult<'program>, Error>
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
                    return Err(battler_error!(
                        "invalid variable in foreach statement: ${}",
                        statement.var.full_name()
                    ));
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
    ) -> Result<bool, Error> {
        let condition = self.evaluate_expr(context, &statement.0)?;
        let condition = match condition.boolean() {
            Some(value) => value,
            _ => {
                return Err(battler_error!(
                    "if statement condition must return a boolean, got {}",
                    condition.value_type()
                ))
            }
        };
        Ok(condition)
    }

    fn evaluate_function_call<'eval, 'program>(
        &'eval self,
        context: &'eval mut EvaluationContext,
        function_call: &'program tree::FunctionCall,
    ) -> Result<Option<MaybeReferenceValue<'eval>>, Error>
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
    ) -> Result<Option<MaybeReferenceValue<'eval>>, Error> {
        run_function(context, function_name, args)
            .map(|val| val.map(|val| MaybeReferenceValue::from(val)))
    }

    fn evaluate_prefix_operator<'eval>(
        op: tree::Operator,
        value: MaybeReferenceValueForOperation<'eval>,
    ) -> Result<MaybeReferenceValue<'eval>, Error> {
        match op {
            tree::Operator::Not => value.negate(),
            _ => Err(battler_error!("invalid prefix operator: {op}")),
        }
    }

    fn evaluate_binary_operator<'eval>(
        lhs: MaybeReferenceValueForOperation<'eval>,
        op: tree::Operator,
        rhs: MaybeReferenceValueForOperation<'eval>,
    ) -> Result<MaybeReferenceValue<'eval>, Error> {
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
            _ => Err(battler_error!("invalid binary operator: {op}")),
        }
    }

    fn evaluate_expr<'eval, 'program>(
        &'eval self,
        context: &'eval mut EvaluationContext,
        expr: &'program tree::Expr,
    ) -> Result<MaybeReferenceValue<'eval>, Error>
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
    ) -> Result<MaybeReferenceValue<'eval>, Error>
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
                            .wrap_error_with_format(format_args!("formatted string is missing positional argument for index {next_arg_index}"))?;
                        next_arg_index += 1;
                        group = MaybeReferenceValueForOperation::from(next_arg)
                            .for_formatted_string()?;
                    } else {
                        return Err(battler_error!("invalid format group: {group}"));
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
    ) -> Result<MaybeReferenceValue, Error>
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
    ) -> Result<Vec<MaybeReferenceValue<'eval>>, Error>
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
    ) -> Result<(), Error> {
        // Drop the reference as soon as possible, because holding it might block a mutable
        // reference to what we want to assign to.
        //
        // For instance, assigning one property of an object to another property on the same object
        // results in a borrow error without this drop.
        let owned_value = value.to_owned();
        drop(value);

        let mut runtime_var = self.create_var_mut(var)?;
        let runtime_var_ref = runtime_var.get_mut(context)?;

        let value_type = owned_value.value_type();
        let var_type = runtime_var_ref.value_type();

        match (runtime_var_ref, owned_value) {
            // The variable can be initialized to any value.
            (ValueRefMut::Undefined(var), val @ _) => *var = val,
            (ValueRefMut::Boolean(var), Value::Boolean(val)) => {
                *var = val;
            }
            (ValueRefMut::OptionalBoolean(var), Value::Boolean(val)) => {
                *var = Some(val);
            }
            (ValueRefMut::I8(var), Value::Fraction(val)) => {
                *var = val
                    .round()
                    .try_into()
                    .wrap_error_with_message("integer overflow")?;
            }
            (ValueRefMut::I8(var), Value::UFraction(val)) => {
                *var = val
                    .round()
                    .try_into()
                    .wrap_error_with_message("integer overflow")?;
            }
            (ValueRefMut::U16(var), Value::Fraction(val)) => {
                *var = val
                    .round()
                    .try_into()
                    .wrap_error_with_message("integer overflow")?;
            }
            (ValueRefMut::U16(var), Value::UFraction(val)) => {
                *var = val
                    .round()
                    .try_into()
                    .wrap_error_with_message("integer overflow")?;
            }
            (ValueRefMut::U32(var), Value::Fraction(val)) => {
                *var = val.round() as u32;
            }
            (ValueRefMut::U32(var), Value::UFraction(val)) => {
                *var = val.round() as u32;
            }
            (ValueRefMut::U64(var), Value::Fraction(val)) => {
                *var = val.round() as u64;
            }
            (ValueRefMut::U64(var), Value::UFraction(val)) => {
                *var = val.round() as u64;
            }
            (ValueRefMut::I64(var), Value::Fraction(val)) => {
                *var = val.round() as i64;
            }
            (ValueRefMut::I64(var), Value::UFraction(val)) => {
                *var = val.round() as i64;
            }
            (ValueRefMut::OptionalISize(var), Value::Fraction(val)) => {
                *var = Some(
                    val.round()
                        .try_into()
                        .wrap_error_with_message("integer overflow")?,
                );
            }
            (ValueRefMut::OptionalISize(var), Value::UFraction(val)) => {
                *var = Some(
                    val.floor()
                        .try_into()
                        .wrap_error_with_message("integer overflow")?,
                );
            }
            (ValueRefMut::OptionalU16(var), Value::Fraction(val)) => {
                *var = Some(
                    val.round()
                        .try_into()
                        .wrap_error_with_message("integer overflow")?,
                );
            }
            (ValueRefMut::OptionalU16(var), Value::UFraction(val)) => {
                *var = Some(
                    val.round()
                        .try_into()
                        .wrap_error_with_message("integer overflow")?,
                );
            }
            (ValueRefMut::Fraction(var), Value::Fraction(val)) => {
                *var = val;
            }
            (ValueRefMut::Fraction(var), Value::UFraction(val)) => {
                *var = val
                    .try_convert()
                    .wrap_error_with_message("integer overflow")?;
            }
            (ValueRefMut::UFraction(var), Value::Fraction(val)) => {
                *var = val
                    .try_convert()
                    .wrap_error_with_message("integer overflow")?;
            }
            (ValueRefMut::UFraction(var), Value::UFraction(val)) => {
                *var = val;
            }
            (ValueRefMut::OptionalFractionU16(var), Value::Fraction(val)) => {
                *var = Some(
                    val.try_convert()
                        .wrap_error_with_message("integer overflow")?,
                );
            }
            (ValueRefMut::OptionalFractionU16(var), Value::UFraction(val)) => {
                *var = Some(
                    val.try_convert()
                        .wrap_error_with_message("integer overflow")?,
                );
            }
            (ValueRefMut::String(var), Value::String(val)) => {
                *var = val;
            }
            (ValueRefMut::OptionalString(var), Value::Undefined) => {
                *var = None;
            }
            (ValueRefMut::OptionalString(var), Value::String(val)) => {
                *var = Some(val);
            }
            (ValueRefMut::Mon(var), Value::Mon(val)) => {
                *var = val;
            }
            (ValueRefMut::Effect(var), Value::Effect(val)) => {
                *var = val;
            }
            (ValueRefMut::ActiveMove(var), Value::ActiveMove(val)) => {
                *var = val;
            }
            (ValueRefMut::MoveCategory(var), Value::MoveCategory(val)) => {
                *var = val;
            }
            (ValueRefMut::MoveTarget(var), Value::MoveTarget(val)) => {
                *var = val;
            }
            (ValueRefMut::MoveTarget(var), Value::String(val)) => {
                *var = MoveTarget::from_str(&val).wrap_error_with_message("invalid move target")?;
            }
            (ValueRefMut::Type(var), Value::Type(val)) => {
                *var = val;
            }
            (ValueRefMut::Boost(var), Value::Boost(val)) => {
                *var = val;
            }
            (ValueRefMut::BoostTable(var), Value::BoostTable(val)) => {
                *var = val;
            }
            (ValueRefMut::OptionalBoostTable(var), Value::BoostTable(val)) => {
                *var = Some(val);
            }
            (ValueRefMut::Side(var), Value::Side(val)) => {
                *var = val;
            }
            (ValueRefMut::MoveSlot(var), Value::MoveSlot(val)) => {
                *var = val;
            }
            (ValueRefMut::Player(var), Value::Player(val)) => {
                *var = val;
            }
            (ValueRefMut::Accuracy(var), Value::Fraction(val)) => {
                *var = Accuracy::from(
                    TryInto::<u8>::try_into((val * 100).floor())
                        .wrap_error_with_message("invalid accuracy")?,
                );
            }
            (ValueRefMut::Accuracy(var), Value::UFraction(val)) => {
                *var = Accuracy::from(
                    TryInto::<u8>::try_into((val * 100).floor())
                        .wrap_error_with_message("invalid accuracy")?,
                );
            }
            (ValueRefMut::Accuracy(var), Value::String(val)) => {
                *var = Accuracy::from_str(&val).wrap_error_with_message("invalid accuracy")?;
            }
            (ValueRefMut::Accuracy(var), Value::Accuracy(val)) => {
                *var = val;
            }
            (ValueRefMut::HitEffect(var), Value::HitEffect(val)) => {
                *var = val;
            }
            (ValueRefMut::OptionalHitEffect(var), Value::Undefined) => {
                *var = None;
            }
            (ValueRefMut::OptionalHitEffect(var), Value::HitEffect(val)) => {
                *var = Some(val);
            }
            (ValueRefMut::Gender(var), Value::Gender(val)) => {
                *var = val;
            }
            (ValueRefMut::OptionalMultihitType(var), Value::Fraction(val)) => {
                *var = Some(MultihitType::Static(
                    val.floor()
                        .try_into()
                        .wrap_error_with_message("integer overflow")?,
                ));
            }
            (ValueRefMut::StatTable(var), Value::StatTable(val)) => {
                *var = val;
            }
            (ValueRefMut::OptionalMultihitType(var), Value::UFraction(val)) => {
                *var = Some(MultihitType::Static(
                    val.floor()
                        .try_into()
                        .wrap_error_with_message("integer overflow")?,
                ));
            }
            (ValueRefMut::EffectState(var), Value::EffectState(val)) => {
                *var = val;
            }
            (ValueRefMut::List(var), Value::List(val)) => {
                *var = val;
            }
            (ValueRefMut::Object(var), Value::Object(val)) => {
                *var = val;
            }
            _ => {
                return Err(battler_error!("invalid assignment of value of type {value_type} to variable ${} of type {var_type}", var.full_name()));
            }
        }

        Ok(())
    }

    fn create_var<'eval, 'program>(
        &'eval self,
        var: &'program tree::Var,
    ) -> Result<Variable<'eval, 'program>, Error>
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
    ) -> Result<VariableMut<'eval, 'program>, Error>
    where
        'program: 'eval,
    {
        let value = match self.vars.get_mut(&var.name.0)? {
            None => {
                self.vars.set(&var.name.0, Value::Undefined)?;
                self.vars
                    .get_mut(&var.name.0)?
                    .wrap_error_with_format(format_args!(
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
