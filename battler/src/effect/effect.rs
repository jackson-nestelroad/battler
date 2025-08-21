use std::ops::{
    Deref,
    DerefMut,
};

use anyhow::Result;
use battler_data::{
    Id,
    Identifiable,
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
    common::split_once_optional,
    conditions::Condition,
    config::Clause,
    effect::fxlang,
    items::Item,
    mons::Species,
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
#[allow(dead_code)]
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
    AbilityCondition,
    Item,
    ItemCondition,
    Clause,
    Species,
}

/// An [`Effect`] handle.
///
/// A stable way to identify an [`Effect`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EffectHandle {
    /// An active move, which is being used or was recently used by a Mon.
    ActiveMove(MoveHandle, MoveHitEffectType),
    /// A condition induced by a move.
    MoveCondition(Id),
    /// An inactive move, which is the move itself without reference to any individual use.
    InactiveMove(Id),
    /// An ability on a Mon.
    Ability(Id),
    /// A condition induced by an ability.
    AbilityCondition(Id),
    /// A condition on a Mon.
    Condition(Id),
    /// An item held by a Mon.
    Item(Id),
    /// A condition induced by an item.
    ItemCondition(Id),
    /// A clause applied to a battle format.
    Clause(Id),
    /// A species.
    Species(Id),
    /// Any effect that is applied to some part of the battle that does not really exist.
    NonExistent(Id),
}

impl EffectHandle {
    /// Creates an [`EffectHandle`] from the fxlang ID of the effect.
    pub fn from_fxlang_id(fxlang_id: &str) -> Self {
        let (effect_type, id) = split_once_optional(fxlang_id, ':');
        let (effect_type, id) = match id {
            Some(id) => (effect_type, id),
            None => ("condition", fxlang_id),
        };
        let id = Id::from(id);
        match effect_type {
            "ability" => Self::Ability(id),
            "abilitycondition" => Self::AbilityCondition(id),
            "clause" => Self::Clause(id),
            "item" => Self::Item(id),
            "itemcondition" => Self::ItemCondition(id),
            "move" => Self::InactiveMove(id),
            "movecondition" => Self::MoveCondition(id),
            "species" => Self::Species(id),
            _ => Self::Condition(id),
        }
    }

    /// Is the effect handle an ability?
    pub fn is_ability(&self) -> bool {
        match self {
            Self::Ability(_) | Self::AbilityCondition(_) => true,
            _ => false,
        }
    }

    /// Is the effect handle an active move?
    pub fn is_active_move(&self) -> bool {
        match self {
            Self::ActiveMove(_, _) => true,
            _ => false,
        }
    }

    /// Is the effect handle a secondary effect of an active move?
    pub fn is_active_move_secondary(&self) -> bool {
        match self {
            Self::ActiveMove(_, MoveHitEffectType::SecondaryEffect(_, _, _)) => true,
            _ => false,
        }
    }
    /// Is the effect handle an item?
    pub fn is_item(&self) -> bool {
        match self {
            Self::Item(_) | Self::ItemCondition(_) => true,
            _ => false,
        }
    }

    /// Returns the ID associated with the effect handle, if any.
    pub fn try_id(&self) -> Option<&Id> {
        match self {
            Self::ActiveMove(_, _) => None,
            Self::MoveCondition(id) => Some(&id),
            Self::InactiveMove(id) => Some(&id),
            Self::Ability(id) => Some(&id),
            Self::AbilityCondition(id) => Some(&id),
            Self::Condition(id) => Some(&id),
            Self::Item(id) => Some(&id),
            Self::ItemCondition(id) => Some(&id),
            Self::Clause(id) => Some(&id),
            Self::Species(id) => Some(&id),
            Self::NonExistent(id) => Some(&id),
        }
    }

    /// Constructs the stable effect handle for this effect handle.
    ///
    /// Every effect handle is stable except for active moves, since active moves can be destroyed
    /// after a few turns. Active move handles will reference their inactive version.
    pub fn stable_effect_handle(&self, context: &Context) -> Result<EffectHandle> {
        match self {
            Self::ActiveMove(active_move_handle, _) => Ok(EffectHandle::InactiveMove(
                context.active_move(*active_move_handle)?.id().clone(),
            )),
            val @ _ => Ok(val.clone()),
        }
    }

    /// Returns the associated condition handle.
    ///
    /// Only applicable for active moves.
    pub fn condition_handle(&self, context: &Context) -> Result<Option<EffectHandle>> {
        match self {
            Self::ActiveMove(active_move_handle, _) => Ok(Some(EffectHandle::MoveCondition(
                context.active_move(*active_move_handle)?.id().clone(),
            ))),
            _ => Ok(None),
        }
    }

    /// The ID for the effect for unlinked effects.
    ///
    /// Only applicable for active moves that use local data with modified effect callbacks. For
    /// example, the move "Bide" executes a special version of the move with custom effect
    /// callbacks. To avoid the cached "Bide" move effects from being used, this ID forces the
    /// evaluation of the custom effects.
    pub fn unlinked_fxlang_id(&self) -> Option<String> {
        match self {
            Self::ActiveMove(active_move_handle, _) => {
                Some(format!("activemove:{active_move_handle}"))
            }
            _ => None,
        }
    }
}

/// A battle effect.
///
/// Contains the borrowed data for the effect.
pub enum Effect<'borrow> {
    /// A move currently being used by a Mon.
    ActiveMove(&'borrow mut Move, MoveHitEffectType),
    /// A condition induced by a previously-used move.
    MoveCondition(ElementRef<'borrow, Move>),
    /// An inactive move, which is not currently being used by a Mon.
    InactiveMove(ElementRef<'borrow, Move>),
    /// An ability, which is permanently applied to a Mon.
    Ability(ElementRef<'borrow, Ability>),
    /// A condition induced by an ability.
    AbilityCondition(ElementRef<'borrow, Ability>),
    /// A condition, which is applied to a Mon for some number of turns.
    Condition(ElementRef<'borrow, Condition>),
    /// An item, which is held by a Mon.
    Item(ElementRef<'borrow, Item>),
    /// A condition induced by an item.
    ItemCondition(ElementRef<'borrow, Item>),
    /// A clause applied to a battle format.
    Clause(ElementRef<'borrow, Clause>),
    /// A species.
    Species(ElementRef<'borrow, Species>),
    /// A non-existent effect, which does nothing.
    NonExistent(Id),
}

impl<'borrow> Effect<'borrow> {
    /// Creates a new effect for the active move.
    pub fn for_active_move(
        active_move: &'borrow mut Move,
        hit_effect_type: MoveHitEffectType,
    ) -> Self {
        Self::ActiveMove(active_move, hit_effect_type)
    }

    /// Creates a new effect for the ability.
    pub fn for_ability(ability: ElementRef<'borrow, Ability>) -> Self {
        Self::Ability(ability)
    }

    /// Creates a new effect for the ability condition.
    pub fn for_ability_condition(ability: ElementRef<'borrow, Ability>) -> Self {
        Self::AbilityCondition(ability)
    }

    /// Creates a new effect for the condition.
    pub fn for_condition(condition: ElementRef<'borrow, Condition>) -> Self {
        Self::Condition(condition)
    }

    /// Creates a new effect for the move condition.
    pub fn for_move_condition(mov: ElementRef<'borrow, Move>) -> Self {
        Self::MoveCondition(mov)
    }

    /// Creates a new effect for the item.
    pub fn for_item(item: ElementRef<'borrow, Item>) -> Self {
        Self::Item(item)
    }

    /// Creates a new effect for the item condition.
    pub fn for_item_condition(item: ElementRef<'borrow, Item>) -> Self {
        Self::ItemCondition(item)
    }

    /// Creates a new effect for the clause.
    pub fn for_clause(clause: ElementRef<'borrow, Clause>) -> Self {
        Self::Clause(clause)
    }

    /// Creates a new effect for the species.
    pub fn for_species(species: ElementRef<'borrow, Species>) -> Self {
        Self::Species(species)
    }

    /// Creates a new effect for the move.
    pub fn for_inactive_move(mov: ElementRef<'borrow, Move>) -> Self {
        Self::InactiveMove(mov)
    }

    /// Creates a new effect for some non-existent effect.
    pub fn for_non_existent(id: Id) -> Self {
        Self::NonExistent(id)
    }

    /// The name of the effect.
    pub fn name(&self) -> &str {
        match self {
            Self::ActiveMove(active_move, _) => &active_move.data.name,
            Self::MoveCondition(mov) | Self::InactiveMove(mov) => &mov.data.name,
            Self::Ability(ability) | Self::AbilityCondition(ability) => &ability.data.name,
            Self::Condition(condition) => &condition.data.name,
            Self::Item(item) | Self::ItemCondition(item) => &item.data.name,
            Self::Clause(clause) => &clause.data.name,
            Self::Species(species) => &species.data.name,
            Self::NonExistent(id) => id.as_ref(),
        }
    }

    /// The type of the effect.
    pub fn effect_type(&self) -> EffectType {
        match self {
            Self::ActiveMove(_, _) => EffectType::Move,
            Self::MoveCondition(_) => EffectType::MoveCondition,
            Self::InactiveMove(_) => EffectType::Move,
            Self::Ability(_) => EffectType::Ability,
            Self::AbilityCondition(_) => EffectType::AbilityCondition,
            Self::Condition(_) => EffectType::Condition,
            Self::Item(_) => EffectType::Item,
            Self::ItemCondition(_) => EffectType::ItemCondition,
            Self::Clause(_) => EffectType::Clause,
            Self::Species(_) => EffectType::Species,
            Self::NonExistent(_) => EffectType::Condition,
        }
    }

    fn effect_type_name(&self) -> &str {
        match self {
            Self::ActiveMove(_, _) | Self::MoveCondition(_) | Self::InactiveMove(_) => "move",
            Self::Ability(_) | Self::AbilityCondition(_) => "ability",
            Self::Condition(condition) => condition.condition_type_name(),
            Self::Item(_) | Self::ItemCondition(_) => "item",
            Self::Clause(_) => "clause",
            Self::Species(_) => "species",
            Self::NonExistent(_) => "",
        }
    }

    /// The full name of the effect, which is prefixed by its type.
    pub fn full_name(&self) -> String {
        match self.effect_type_name() {
            "" => self.name().to_owned(),
            prefix => format!("{prefix}:{}", self.name()),
        }
    }

    fn fxlang_id_effect_type_name(&self) -> String {
        match self {
            Self::ActiveMove(_, hit_effect_type) => match hit_effect_type.secondary_index() {
                None => "move".to_owned(),
                Some((target, hit, secondary_index)) => {
                    format!("movesecondary-{hit}-{target}-{secondary_index}")
                }
            },
            Self::MoveCondition(_) => "movecondition".to_owned(),
            Self::InactiveMove(_) => "move".to_owned(),
            Self::Ability(_) => "ability".to_owned(),
            Self::AbilityCondition(_) => "abilitycondition".to_owned(),
            Self::Condition(condition) => condition.condition_type_name().to_owned(),
            Self::Item(_) => "item".to_owned(),
            Self::ItemCondition(_) => "itemcondition".to_owned(),
            Self::Clause(_) => "clause".to_owned(),
            Self::Species(_) => "species".to_owned(),
            Self::NonExistent(_) => "condition".to_owned(),
        }
    }

    /// The ID of the effect, used for caching fxlang effect callbacks.
    pub fn fxlang_id(&self) -> String {
        match self.fxlang_id_effect_type_name().as_str() {
            "" => format!("{}", self.id()),
            prefix => format!("{prefix}:{}", self.id()),
        }
    }

    /// The underlying move, if any.
    pub fn move_effect<'effect>(&'effect self) -> Option<&'effect Move> {
        match self {
            Self::ActiveMove(active_move, _) => Some(active_move.deref()),
            Self::InactiveMove(mov) => Some(mov),
            Self::MoveCondition(mov) => Some(mov),
            _ => None,
        }
    }

    /// The underlying condition, if any.
    pub fn condition<'effect>(&'effect self) -> Option<&'effect Condition> {
        match self {
            Self::Condition(condition) => Some(condition.deref()),
            _ => None,
        }
    }

    /// The associated [`fxlang::Effect`].
    pub fn fxlang_effect<'effect>(&'effect self) -> Option<&'effect fxlang::Effect> {
        match self {
            Self::ActiveMove(active_move, hit_effect_type) => {
                active_move.fxlang_effect(*hit_effect_type)
            }
            Self::MoveCondition(mov) => Some(&mov.condition),
            Self::InactiveMove(mov) => Some(&mov.effect),
            Self::Ability(ability) => Some(&ability.effect),
            Self::AbilityCondition(ability) => Some(&ability.condition),
            Self::Condition(condition) => Some(&condition.condition),
            Self::Item(item) => Some(&item.effect),
            Self::ItemCondition(item) => Some(&item.condition),
            Self::Clause(clause) => Some(&clause.effect),
            Self::Species(_) => None,
            Self::NonExistent(_) => None,
        }
    }

    /// Whether the effect is marked as unlinked from its static data.
    pub fn unlinked(&self) -> bool {
        match self {
            Self::ActiveMove(active_move, _) => active_move.unlinked,
            _ => false,
        }
    }
}

impl Identifiable for Effect<'_> {
    fn id(&self) -> &Id {
        match self {
            Self::ActiveMove(active_move, _) => active_move.id(),
            Self::MoveCondition(mov) | Self::InactiveMove(mov) => mov.id(),
            Self::Ability(ability) | Self::AbilityCondition(ability) => ability.id(),
            Self::Condition(condition) => condition.id(),
            Self::Item(item) | Self::ItemCondition(item) => item.id(),
            Self::Clause(clause) => clause.id(),
            Self::Species(species) => species.id(),
            Self::NonExistent(id) => id,
        }
    }
}
