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
    battle::{
        Context,
        MoveHandle,
    },
    common::{
        Error,
        Id,
        Identifiable,
    },
    conditions::Condition,
    effect::fxlang,
    items::Item,
    moves::{
        Move,
        MoveHitEffectType,
    },
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
    Item,
}

/// An [`Effect`] handle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EffectHandle {
    ActiveMove(MoveHandle, MoveHitEffectType),
    MoveCondition(Id),
    InactiveMove(Id),
    Ability(Id),
    Condition(Id),
    Item(Id),
    NonExistent(Id),
}

impl EffectHandle {
    pub fn is_ability(&self) -> bool {
        match self {
            Self::Ability(_) => true,
            _ => false,
        }
    }

    pub fn is_active_move(&self) -> bool {
        match self {
            Self::ActiveMove(_, _) => true,
            _ => false,
        }
    }

    pub fn try_id(&self) -> Option<&Id> {
        match self {
            Self::ActiveMove(_, _) => None,
            Self::MoveCondition(id) => Some(&id),
            Self::InactiveMove(id) => Some(&id),
            Self::Ability(id) => Some(&id),
            Self::Condition(id) => Some(&id),
            Self::Item(id) => Some(&id),
            Self::NonExistent(id) => Some(&id),
        }
    }

    pub fn stable_effect_handle(&self, context: &Context) -> Result<EffectHandle, Error> {
        match self {
            Self::ActiveMove(active_move_handle, _) => Ok(EffectHandle::MoveCondition(
                context.active_move(*active_move_handle)?.id().clone(),
            )),
            val @ _ => Ok(val.clone()),
        }
    }

    pub fn condition_handle(&self, context: &Context) -> Result<Option<EffectHandle>, Error> {
        match self {
            Self::ActiveMove(active_move_handle, _) => Ok(Some(EffectHandle::MoveCondition(
                context.active_move(*active_move_handle)?.id().clone(),
            ))),
            _ => Ok(None),
        }
    }
}

/// A battle effect.
pub enum Effect<'borrow> {
    /// A move currently being used by a Mon.
    ActiveMove(&'borrow mut Move, MoveHitEffectType),
    /// A condition induced by a previously-used move.
    MoveCondition(ElementRef<'borrow, Move>),
    /// An inactive move, which is not currently being used by a Mon.
    InactiveMove(ElementRef<'borrow, Move>),
    /// An ability, which is permanently applied to a Mon.
    Ability(ElementRef<'borrow, Ability>),
    /// A condition, which is applied to a Mon for some number of turns.
    Condition(ElementRef<'borrow, Condition>),
    /// An item, which is held by a Mon.
    Item(ElementRef<'borrow, Item>),
    /// A non-existent effect, which does nothing.
    NonExistent(Id),
}

impl<'borrow> Effect<'borrow> {
    pub fn for_active_move(
        active_move: &'borrow mut Move,
        hit_effect_type: MoveHitEffectType,
    ) -> Self {
        Self::ActiveMove(active_move, hit_effect_type)
    }

    pub fn for_ability(ability: ElementRef<'borrow, Ability>) -> Self {
        Self::Ability(ability)
    }

    pub fn for_condition(condition: ElementRef<'borrow, Condition>) -> Self {
        Self::Condition(condition)
    }

    pub fn for_move_condition(mov: ElementRef<'borrow, Move>) -> Self {
        Self::MoveCondition(mov)
    }

    pub fn for_item(item: ElementRef<'borrow, Item>) -> Self {
        Self::Item(item)
    }

    pub fn for_inactive_move(mov: ElementRef<'borrow, Move>) -> Self {
        Self::InactiveMove(mov)
    }

    pub fn for_non_existent(id: Id) -> Self {
        Self::NonExistent(id)
    }

    pub fn name(&self) -> &str {
        match self {
            Self::ActiveMove(active_move, _) => &active_move.data.name,
            Self::MoveCondition(mov) => &mov.data.name,
            Self::InactiveMove(mov) => &mov.data.name,
            Self::Ability(ability) => &ability.data.name,
            Self::Condition(condition) => &condition.data.name,
            Self::Item(item) => &item.data.name,
            Self::NonExistent(id) => id.as_ref(),
        }
    }

    pub fn effect_type(&self) -> EffectType {
        match self {
            Self::ActiveMove(_, _) => EffectType::Move,
            Self::MoveCondition(_) => EffectType::MoveCondition,
            Self::InactiveMove(_) => EffectType::Move,
            Self::Ability(_) => EffectType::Ability,
            Self::Condition(_) => EffectType::Condition,
            Self::Item(_) => EffectType::Item,
            Self::NonExistent(_) => EffectType::Condition,
        }
    }

    fn effect_type_name(&self) -> &str {
        match self {
            Self::ActiveMove(_, _) => "move",
            Self::MoveCondition(_) => "move",
            Self::InactiveMove(_) => "move",
            Self::Ability(_) => "ability",
            Self::Condition(condition) => condition.condition_type_name(),
            Self::Item(_) => "item",
            Self::NonExistent(_) => "condition",
        }
    }

    pub fn full_name(&self) -> String {
        match self.effect_type_name() {
            "" => self.name().to_owned(),
            prefix => format!("{prefix}:{}", self.name()),
        }
    }

    fn internal_effect_type_name(&self) -> String {
        match self {
            Self::ActiveMove(_, hit_effect_type) => match hit_effect_type.secondary_index() {
                None => "move".to_owned(),
                Some(secondary_index) => format!("movesecondary{secondary_index}"),
            },
            Self::MoveCondition(_) => "movecondition".to_owned(),
            Self::InactiveMove(_) => "move".to_owned(),
            Self::Ability(_) => "ability".to_owned(),
            Self::Condition(condition) => condition.condition_type_name().to_owned(),
            Self::Item(_) => "item".to_owned(),
            Self::NonExistent(_) => "condition".to_owned(),
        }
    }

    pub fn internal_fxlang_id(&self) -> String {
        match self.internal_effect_type_name().as_str() {
            "" => format!("{}", self.id()),
            prefix => format!("{prefix}:{}", self.id()),
        }
    }

    pub fn active_move<'effect>(&'effect self) -> Option<&'effect Move> {
        match self {
            Self::ActiveMove(active_move, _) => Some(active_move.deref()),
            _ => None,
        }
    }

    pub fn active_move_mut<'effect>(&'effect mut self) -> Option<&'effect mut Move> {
        match self {
            Self::ActiveMove(active_move, _) => Some(active_move.deref_mut()),
            _ => None,
        }
    }

    pub fn condition<'effect>(&'effect self) -> Option<&'effect Condition> {
        match self {
            Self::Condition(condition) => Some(condition.deref()),
            _ => None,
        }
    }

    pub fn source_effect_handle(&self) -> Option<&EffectHandle> {
        match self {
            Self::ActiveMove(active_move, _) => active_move.source_effect.as_ref(),
            _ => None,
        }
    }

    pub fn fxlang_effect<'effect>(&'effect self) -> Option<&'effect fxlang::Effect> {
        match self {
            Self::ActiveMove(active_move, hit_effect_type) => {
                active_move.fxlang_effect(*hit_effect_type)
            }
            Self::MoveCondition(mov) => Some(&mov.data.condition.effect),
            Self::InactiveMove(mov) => Some(&mov.data.effect),
            Self::Ability(ability) => Some(&ability.data.effect),
            Self::Condition(condition) => Some(&condition.data.condition.effect),
            Self::Item(item) => Some(&item.data.effect),
            Self::NonExistent(_) => None,
        }
    }

    pub fn fxlang_condition<'effect>(&'effect self) -> Option<&'effect fxlang::Condition> {
        match self {
            Self::Condition(condition) => Some(&condition.data.condition),
            Self::MoveCondition(mov) => Some(&mov.data.condition),
            _ => None,
        }
    }

    pub fn infiltrates(&self) -> bool {
        match self {
            Self::ActiveMove(active_move, _) => active_move.infiltrates,
            _ => false,
        }
    }
}

impl Identifiable for Effect<'_> {
    fn id(&self) -> &Id {
        match self {
            Self::ActiveMove(active_move, _) => active_move.id(),
            Self::MoveCondition(mov) => mov.id(),
            Self::InactiveMove(mov) => mov.id(),
            Self::Ability(ability) => ability.id(),
            Self::Condition(condition) => condition.id(),
            Self::Item(item) => item.id(),
            Self::NonExistent(id) => id,
        }
    }
}
