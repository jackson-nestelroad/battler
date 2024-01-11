use std::ops::{
    Deref,
    DerefMut,
};

use zone_alloc::ElementRefMut;

use crate::{
    abilities::Ability,
    battle::MoveHandle,
    common::{
        Id,
        Identifiable,
    },
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
}

/// An [`Effect`] handle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EffectHandle {
    ActiveMove(MoveHandle),
    Ability(Id),
}

/// A battle effect.
pub enum Effect<'borrow> {
    ActiveMove(&'borrow mut Move),
    Ability(&'borrow Ability),
}

impl<'borrow> Effect<'borrow> {
    pub fn for_active_move(active_move: &'borrow mut Move) -> Self {
        Self::ActiveMove(active_move)
    }

    pub fn name(&self) -> &str {
        match self {
            Self::ActiveMove(active_move) => &active_move.data.name,
            Self::Ability(ability) => &ability.data.name,
        }
    }

    pub fn effect_type(&self) -> EffectType {
        match self {
            Self::ActiveMove(_) => EffectType::Move,
            Self::Ability(_) => EffectType::Ability,
        }
    }

    fn effect_type_name(&self) -> &str {
        match self {
            Self::ActiveMove(_) => "move",
            Self::Ability(_) => "ability",
        }
    }

    pub fn full_name(&self) -> String {
        format!("{}: {}", self.effect_type_name(), self.name())
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
}

impl Identifiable for Effect<'_> {
    fn id(&self) -> &Id {
        match self {
            Self::ActiveMove(active_move) => active_move.id(),
            Self::Ability(ability) => ability.id(),
        }
    }
}
