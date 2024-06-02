use std::ops::{
    Deref,
    DerefMut,
};

use zone_alloc::{
    ElementRef,
    ElementRefMut,
};

use crate::{
    abilities::Ability,
    battle::MoveHandle,
    common::{
        Id,
        Identifiable,
    },
    conditions::Condition,
    effect::fxlang,
    moves::Move,
};

/// Similar to [`MaybeOwned`][`crate::common::MaybeOwned`], but for an optional mutable reference
/// backed by a [`ElementRefMut`].
///
/// If the reference is owned the [`ElementRefMut`] is stored directly. If the reference is unowned,
/// it is stored directly with the assumption that it originates from an [`ElementRefMut`].
pub enum MaybeElementRef<'a, T> {
    Owned(ElementRefMut<'a, T>),
    Unowned(&'a mut T),
}

impl<T> Deref for MaybeElementRef<'_, T> {
    type Target = T;
    fn deref(&self) -> &Self::Target {
        match self {
            Self::Owned(val) => val.deref(),
            Self::Unowned(val) => val,
        }
    }
}

impl<T> DerefMut for MaybeElementRef<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        match self {
            Self::Owned(val) => val.deref_mut(),
            Self::Unowned(val) => val,
        }
    }
}

impl<T> AsMut<T> for MaybeElementRef<'_, T> {
    fn as_mut(&mut self) -> &mut T {
        self.deref_mut()
    }
}

impl<'a, T> From<ElementRefMut<'a, T>> for MaybeElementRef<'a, T> {
    fn from(value: ElementRefMut<'a, T>) -> Self {
        Self::Owned(value)
    }
}

impl<'a, T> From<&'a mut T> for MaybeElementRef<'a, T> {
    fn from(value: &'a mut T) -> Self {
        Self::Unowned(value)
    }
}

/// The type of an [`Effect`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectType {
    Move,
    Ability,
    Condition,
    MoveCondition,
}

/// An [`Effect`] handle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EffectHandle {
    ActiveMove(MoveHandle),
    Ability(Id),
    Condition(Id),
    MoveCondition(Id),
}

/// A battle effect.
pub enum Effect<'borrow> {
    /// A move currently being used by a Mon.
    ActiveMove(&'borrow mut Move),
    /// An ability, which is permanently applied to a Mon.
    Ability(ElementRef<'borrow, Ability>),
    /// A condition, which is applied to a Mon for some number of turns.
    Condition(ElementRef<'borrow, Condition>),
    /// A condition induced by a previously-used move.
    MoveCondition(ElementRef<'borrow, Move>),
}

impl<'borrow> Effect<'borrow> {
    pub fn for_active_move(active_move: &'borrow mut Move) -> Self {
        Self::ActiveMove(active_move)
    }

    pub fn for_condition(condition: ElementRef<'borrow, Condition>) -> Self {
        Self::Condition(condition)
    }

    pub fn name(&self) -> &str {
        match self {
            Self::ActiveMove(active_move) => &active_move.data.name,
            Self::Ability(ability) => &ability.data.name,
            Self::Condition(condition) => &condition.data.name,
            Self::MoveCondition(mov) => &mov.data.name,
        }
    }

    pub fn effect_type(&self) -> EffectType {
        match self {
            Self::ActiveMove(_) => EffectType::Move,
            Self::Ability(_) => EffectType::Ability,
            Self::Condition(_) => EffectType::Condition,
            Self::MoveCondition(_) => EffectType::MoveCondition,
        }
    }

    fn effect_type_name(&self) -> &str {
        match self {
            Self::ActiveMove(_) => "move",
            Self::Ability(_) => "ability",
            Self::Condition(condition) => condition.condition_type_name(),
            Self::MoveCondition(_) => "move",
        }
    }

    pub fn full_name(&self) -> String {
        match self.effect_type_name() {
            "" => self.name().to_owned(),
            prefix => format!("{prefix}:{}", self.name()),
        }
    }

    fn internal_effect_type_name(&self) -> &str {
        match self {
            Self::ActiveMove(_) => "move",
            Self::Ability(_) => "ability",
            Self::Condition(condition) => condition.condition_type_name(),
            Self::MoveCondition(_) => "movecondition",
        }
    }

    pub fn internal_fxlang_id(&self) -> String {
        match self.internal_effect_type_name() {
            "" => format!("{}", self.id()),
            prefix => format!("{prefix}:{}", self.id()),
        }
    }

    pub fn active_move<'effect>(&'effect self) -> Option<&'effect Move> {
        match self {
            Self::ActiveMove(active_move) => Some(active_move.deref()),
            _ => None,
        }
    }

    pub fn active_move_mut<'effect>(&'effect mut self) -> Option<&'effect mut Move> {
        match self {
            Self::ActiveMove(active_move) => Some(active_move.deref_mut()),
            _ => None,
        }
    }

    pub fn condition<'effect>(&'effect self) -> Option<&'effect Condition> {
        match self {
            Self::Condition(condition) => Some(condition.deref()),
            _ => None,
        }
    }

    pub fn fxlang_callbacks<'effect>(&'effect self) -> &'effect fxlang::Callbacks {
        match self {
            Self::ActiveMove(active_move) => &active_move.data.effect.callbacks,
            Self::Ability(ability) => &ability.data.effect.callbacks,
            Self::Condition(condition) => &condition.data.condition.callbacks,
            Self::MoveCondition(mov) => &mov.data.condition.callbacks,
        }
    }
}

impl Identifiable for Effect<'_> {
    fn id(&self) -> &Id {
        match self {
            Self::ActiveMove(active_move) => active_move.id(),
            Self::Ability(ability) => ability.id(),
            Self::Condition(condition) => condition.id(),
            Self::MoveCondition(mov) => mov.id(),
        }
    }
}
