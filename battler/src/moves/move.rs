use std::collections::hash_map::Entry;

use ahash::{
    HashMap,
    HashSet,
};
use anyhow::Error;
use battler_data::{
    Fraction,
    HitEffect,
    Id,
    Identifiable,
    MaxMoveData,
    MoveCategory,
    MoveData,
    MoveFlag,
    SecondaryEffectData,
    Type,
};

use crate::{
    battle::MonHandle,
    effect::fxlang,
    general_error,
};

/// Dynamic data on how a move hit a target.
#[derive(Clone)]
pub struct MoveHitData {
    /// Did the move critical hit?
    pub crit: bool,
    /// Type modifier on the damage calculation.
    pub type_modifier: i8,
    /// Arbitrary flags that can be set by moves.
    pub flags: HashSet<Id>,
}

impl MoveHitData {
    pub fn new() -> Self {
        Self {
            crit: false,
            type_modifier: 0,
            flags: HashSet::default(),
        }
    }
}

/// The current type of [`HitEffect`] being applied on an active [`Move`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MoveHitEffectType {
    PrimaryEffect,
    SecondaryEffect(MonHandle, u8, usize),
}

impl MoveHitEffectType {
    /// The index of the secondary effect, if any.
    pub fn secondary_index(&self) -> Option<(MonHandle, u8, usize)> {
        match self {
            Self::PrimaryEffect => None,
            Self::SecondaryEffect(mon, hit, index) => Some((*mon, *hit, *index)),
        }
    }
}

/// Secondary effect that occurs after a move is used.
#[derive(Clone)]
pub struct SecondaryEffect {
    pub data: SecondaryEffectData,
    pub effect: fxlang::Effect,
}

impl SecondaryEffect {
    pub fn new(data: SecondaryEffectData) -> Self {
        let effect = data.effect.clone().try_into().unwrap_or_default();
        Self { data, effect }
    }
}

/// The source of an upgraded move.
#[derive(Debug, Clone)]
pub enum UpgradedMoveSource {
    MaxMove { base_move: Id },
}

impl UpgradedMoveSource {
    /// The base move of the upgraded move.
    pub fn base_move(&self) -> Option<Id> {
        match self {
            Self::MaxMove { base_move, .. } => Some(base_move.clone()),
        }
    }
}

fn default_max_move(
    id: &Id,
    category: MoveCategory,
    primary_type: Type,
    base_power: u32,
) -> Option<MaxMoveData> {
    if category == MoveCategory::Status || id == "struggle" {
        return None;
    }

    if primary_type == Type::Fighting || primary_type == Type::Poison {
        if base_power >= 150 {
            Some(MaxMoveData { base_power: 100 })
        } else if base_power >= 110 {
            Some(MaxMoveData { base_power: 95 })
        } else if base_power >= 75 {
            Some(MaxMoveData { base_power: 90 })
        } else if base_power >= 65 {
            Some(MaxMoveData { base_power: 85 })
        } else if base_power >= 55 {
            Some(MaxMoveData { base_power: 80 })
        } else if base_power >= 45 {
            Some(MaxMoveData { base_power: 75 })
        } else {
            Some(MaxMoveData { base_power: 70 })
        }
    } else {
        if base_power >= 150 {
            Some(MaxMoveData { base_power: 150 })
        } else if base_power >= 110 {
            Some(MaxMoveData { base_power: 140 })
        } else if base_power >= 75 {
            Some(MaxMoveData { base_power: 130 })
        } else if base_power >= 65 {
            Some(MaxMoveData { base_power: 120 })
        } else if base_power >= 55 {
            Some(MaxMoveData { base_power: 110 })
        } else if base_power >= 45 {
            Some(MaxMoveData { base_power: 100 })
        } else {
            Some(MaxMoveData { base_power: 90 })
        }
    }
}

/// An individual move, which can be used by a Mon in battle.
///
/// Unlike other move effects, [`Move`]s are mutable across multiple Mons and turns. A move used by
/// one Mon can have different effects than the ame move used by another Mon.
#[derive(Clone)]
pub struct Move {
    id: Id,
    pub data: MoveData,
    pub effect: fxlang::Effect,
    pub condition: fxlang::Effect,

    /// Custom STAB modifier, if any.
    pub stab_modifier: Option<Fraction<u32>>,

    /// The Mon that used the move.
    pub used_by: Option<MonHandle>,
    /// The move was used externally, rather than directly by a Mon through its moveset.
    pub external: bool,
    /// Whether or not this move hit multiple targets.
    pub spread_hit: bool,
    /// Number of hits dealt by the move.
    pub hit: u8,
    /// Total damage dealt by the move.
    pub total_damage: u64,
    /// Have the primary user effect been applied?
    pub primary_user_effect_applied: bool,
    /// Is the move upgraded?
    pub upgraded: Option<UpgradedMoveSource>,

    /// Fxlang effect state.
    pub effect_state: fxlang::EffectState,
    /// Whether or not the move is unlinked from the original data.
    ///
    /// If set to true, fxlang effect programs will be parsed and cached relative to this
    /// individual move instance, rather than relative to the original move data. In other words,
    /// the effects of this move are "unlinked" from the effects of the original move, allowing
    /// this move to specify different callbacks than the original move, even though they share the
    /// same ID.
    pub unlinked: bool,
    /// Secondary effects for each target.
    ///
    /// Secondary effects can be modified by effects on the user and the individual target.
    pub secondary_effects: HashMap<(MonHandle, u8), Vec<SecondaryEffect>>,

    hit_data: HashMap<(MonHandle, u8), MoveHitData>,
}

impl Move {
    fn apply_defaults_to_data(mut data: MoveData, id: &Id) -> MoveData {
        if data.max_move.is_none() && !data.flags.contains(&MoveFlag::Max) {
            data.max_move = default_max_move(id, data.category, data.primary_type, data.base_power);
        }
        data
    }

    fn new_internal(id: Id, data: MoveData, unlinked: bool) -> Self {
        let data = Self::apply_defaults_to_data(data, &id);
        let effect = data.effect.clone().try_into().unwrap_or_default();
        let condition = data.condition.clone().try_into().unwrap_or_default();
        Self {
            id,
            data,
            effect,
            condition,
            used_by: None,
            stab_modifier: None,
            external: false,
            spread_hit: false,
            hit: 0,
            total_damage: 0,
            primary_user_effect_applied: false,
            upgraded: None,
            effect_state: fxlang::EffectState::default(),
            unlinked,
            secondary_effects: HashMap::default(),
            hit_data: HashMap::default(),
        }
    }

    /// Creates a new active move, which can be modified for the use of the move.
    pub fn new(id: Id, data: MoveData) -> Self {
        Self::new_internal(id, data, false)
    }

    /// Creates a new active move, with unlinked effect callbacks.
    pub fn new_unlinked(id: Id, data: MoveData) -> Self {
        Self::new_internal(id, data, true)
    }

    /// Clones an active move for use in battle.
    ///
    /// Only some fields are truly cloned.
    pub fn clone_for_battle(&self) -> Self {
        let mut clone = Self::new(self.id.clone(), self.data.clone());
        clone.total_damage = self.total_damage;
        clone.effect_state = self.effect_state.clone();
        clone
    }

    /// Returns the hit data for the target, if any.
    pub fn hit_data(&self, target: MonHandle) -> Option<&MoveHitData> {
        self.hit_data.get(&(target, self.hit))
    }

    /// Returns the hit data for the target.
    pub fn hit_data_mut(&mut self, target: MonHandle) -> &mut MoveHitData {
        self.hit_data
            .entry((target, self.hit))
            .or_insert(MoveHitData::new())
    }

    /// Returns a reference to the hit effect.
    pub fn target_hit_effect(&self, hit_effect_type: MoveHitEffectType) -> Option<&HitEffect> {
        match hit_effect_type {
            MoveHitEffectType::PrimaryEffect => self.data.hit_effect.as_ref(),
            MoveHitEffectType::SecondaryEffect(target, hit, index) => self
                .secondary_effects
                .get(&(target, hit))?
                .get(index)?
                .data
                .target
                .as_ref(),
        }
    }

    /// Returns a mutable reference to the hit effect.
    pub fn target_hit_effect_mut(
        &mut self,
        hit_effect_type: MoveHitEffectType,
    ) -> Option<&mut HitEffect> {
        match hit_effect_type {
            MoveHitEffectType::PrimaryEffect => self.data.hit_effect.as_mut(),
            MoveHitEffectType::SecondaryEffect(target, hit, index) => self
                .secondary_effects
                .get_mut(&(target, hit))?
                .get_mut(index)?
                .data
                .target
                .as_mut(),
        }
    }

    /// Returns a reference to the hit effect on the user.
    pub fn user_hit_effect(&self, hit_effect_type: MoveHitEffectType) -> Option<&HitEffect> {
        match hit_effect_type {
            MoveHitEffectType::PrimaryEffect => self.data.user_effect.as_ref(),
            MoveHitEffectType::SecondaryEffect(target, hit, index) => self
                .secondary_effects
                .get(&(target, hit))?
                .get(index)?
                .data
                .user
                .as_ref(),
        }
    }

    /// Returns a mutable reference to the hit effect on the user.
    pub fn user_hit_effect_mut(
        &mut self,
        hit_effect_type: MoveHitEffectType,
    ) -> Option<&mut HitEffect> {
        match hit_effect_type {
            MoveHitEffectType::PrimaryEffect => self.data.user_effect.as_mut(),
            MoveHitEffectType::SecondaryEffect(target, hit, index) => self
                .secondary_effects
                .get_mut(&(target, hit))?
                .get_mut(index)?
                .data
                .user
                .as_mut(),
        }
    }

    /// Returns the corresponding fxlang effect for the hit effect.
    pub fn fxlang_effect(&self, hit_effect_type: MoveHitEffectType) -> Option<&fxlang::Effect> {
        match hit_effect_type {
            MoveHitEffectType::PrimaryEffect => Some(&self.effect),
            MoveHitEffectType::SecondaryEffect(target, hit, index) => Some(
                &self
                    .secondary_effects
                    .get(&(target, hit))?
                    .get(index)?
                    .effect,
            ),
        }
    }

    /// Saves secondary effects for the given target.
    ///
    /// Fails if there are already secondary effects for the target.
    ///
    /// Returns a copy of the secondary effects.
    pub fn save_secondary_effects(
        &mut self,
        target: MonHandle,
        secondary_effects: Vec<SecondaryEffect>,
    ) -> Result<(), Error> {
        match self.secondary_effects.entry((target, self.hit)) {
            Entry::Occupied(_) => Err(general_error(format!(
                "target {target} already has secondary effects saved for hit {}",
                self.hit,
            ))),
            Entry::Vacant(entry) => {
                entry.insert(secondary_effects);
                Ok(())
            }
        }
    }

    /// Returns an iterator over the secondary effect chances that should be run for applying the
    /// secondary effect at the given index.
    pub fn secondary_effect_chances<'a>(
        &'a self,
        target: MonHandle,
    ) -> Box<dyn Iterator<Item = (usize, Option<Fraction<u16>>)> + 'a> {
        match self.secondary_effects.get(&(target, self.hit)) {
            Some(secondary_effects) => Box::new(
                secondary_effects
                    .iter()
                    .map(|secondary_effect| secondary_effect.data.chance)
                    .enumerate(),
            ),
            None => Box::new(std::iter::empty::<(usize, Option<Fraction<u16>>)>()),
        }
    }

    /// This move is callable from other moves.
    pub fn callable(&self) -> bool {
        !self.data.flags.contains(&MoveFlag::Max)
    }
}

impl Identifiable for Move {
    fn id(&self) -> &Id {
        &self.id
    }
}
