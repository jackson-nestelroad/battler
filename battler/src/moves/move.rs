use ahash::HashMapExt;
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    battle::{
        BoostTable,
        MonHandle,
    },
    common::{
        FastHashMap,
        FastHashSet,
        Fraction,
        Id,
        Identifiable,
    },
    effect::{
        fxlang,
        EffectHandle,
    },
    mons::{
        Stat,
        Type,
    },
    moves::{
        Accuracy,
        MonOverride,
        MoveCategory,
        MoveFlags,
        MoveTarget,
        MultihitType,
        OhkoType,
        SelfDestructType,
        SwitchType,
    },
};

/// The effect of being hit by a move.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HitEffect {
    /// Stat boosts.
    pub boosts: Option<BoostTable>,
    /// Percentage of target's HP to heal.
    pub heal_percent: Option<Fraction<u16>>,
    /// Status applied.
    pub status: Option<String>,
    /// Volatile status applied.
    pub volatile_status: Option<String>,
    /// Side condition applied.
    pub side_condition: Option<String>,
    /// Slot condition applied.
    pub slot_condition: Option<String>,
    /// Weather applied.
    pub weather: Option<String>,
    /// Pseudo-weather applied.
    pub pseudo_weather: Option<String>,
    /// Terrain applied.
    pub terrain: Option<String>,
    /// Force the target to switch out?
    #[serde(default)]
    pub force_switch: bool,
}

/// Secondary effect that occurs after a move is used.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SecondaryEffect {
    /// Chance of the effect occurring.
    pub chance: Option<Fraction<u16>>,
    /// Secondary hit effect on the target.
    pub target: Option<HitEffect>,
    /// Secondary hit effect on the user of the move.
    pub user: Option<HitEffect>,
    /// Dynamic battle effects.
    #[serde(default)]
    pub effect: Box<fxlang::Effect>,
}

fn default_crit_ratio() -> Option<u8> {
    Some(1)
}

/// Data about a particular move.
///
/// Moves are the primary effect that drive battle forward. Every Mon enters a battle with their
/// moveset. Each turn, a Mon uses one move to affect the battle. Moves can damage opposing Mons,
/// affect ally Mons or the user itself, boost or drop stats, apply conditions to Mons or the
/// battlefield itself, and more.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveData {
    /// Name of the move.
    pub name: String,
    /// Move category.
    pub category: MoveCategory,
    /// Move type.
    pub primary_type: Type,
    /// Base power.
    #[serde(default)]
    pub base_power: u32,
    /// Base accuracy.
    pub accuracy: Accuracy,
    /// Total power points, which is the number of times this move can be used.
    #[serde(default)]
    pub pp: u8,
    /// Move priority.
    #[serde(default)]
    pub priority: i8,
    /// Move target(s).
    pub target: MoveTarget,
    /// Move flags.
    pub flags: FastHashSet<MoveFlags>,

    /// Static damage dealt.
    pub damage: Option<u16>,
    /// Disallow PP boosts?
    #[serde(default)]
    pub no_pp_boosts: bool,

    /// Type of OHKO, if any.
    ///
    /// If a target has this type, it is immune.
    pub ohko_type: Option<OhkoType>,
    /// Thaws the target?
    #[serde(default)]
    pub thaws_target: bool,
    /// Type of switch that occurs on the user.
    pub user_switch: Option<SwitchType>,
    /// How the user self destructs.
    pub self_destruct: Option<SelfDestructType>,
    /// Does the move break protect?
    #[serde(default)]
    pub breaks_protect: bool,
    /// The percentage of damage dealt for recoil.
    pub recoil_percent: Option<Fraction<u16>>,
    /// Calculate recoil damage from user HP?
    #[serde(default)]
    pub recoil_from_user_hp: bool,
    /// The percentage of the target's HP to drain.
    pub drain_percent: Option<Fraction<u16>>,
    /// Apply Struggle recoil?
    #[serde(default)]
    pub struggle_recoil: bool,
    /// Typeless?
    #[serde(default)]
    pub typeless: bool,

    /// Primary effect applied to the target.
    ///
    /// Applied when the move hits.
    pub hit_effect: Option<HitEffect>,
    /// Primary effect on the user.
    ///
    /// Applied when the move hits.
    pub user_effect: Option<HitEffect>,
    /// Chance of the user effect occurring.
    pub user_effect_chance: Option<Fraction<u16>>,
    /// Secondary effects applied to the target.
    #[serde(default)]
    pub secondary_effects: Vec<SecondaryEffect>,

    /// Mon override for offensive stat calculations.
    ///
    /// By default, the move user is used.
    pub override_offensive_mon: Option<MonOverride>,
    /// Stat override for offensive stat calculations.
    ///
    /// By default, Atk is used for physical moves and SpA is used for special moves.
    pub override_offensive_stat: Option<Stat>,
    /// Mon override for defensive stat calculations.
    ///
    /// By default, the move target is used.
    pub override_defensive_mon: Option<MonOverride>,
    /// Stat override for defensive stat calculations.
    ///
    /// By default, Def is used for physical moves and SpD is used for special moves.
    pub override_defensive_stat: Option<Stat>,

    /// Critical hit ratio.
    #[serde(default = "default_crit_ratio")]
    pub crit_ratio: Option<u8>,
    /// Ignore ability effects?
    #[serde(default)]
    pub ignore_ability: bool,
    /// Ignore accuracy modifiers?
    #[serde(default)]
    pub ignore_accuracy: bool,
    /// Ignore defensive modifiers?
    #[serde(default)]
    pub ignore_defensive: bool,
    /// Ignore evasion modifiers?
    #[serde(default)]
    pub ignore_evasion: bool,
    /// Ignore immunity?
    #[serde(default)]
    pub ignore_immunity: Option<bool>,
    /// Ignore offensive modifiers?
    #[serde(default)]
    pub ignore_offensive: bool,
    /// Accuracy calculations should be run multiple times.
    #[serde(default)]
    pub multiaccuracy: bool,
    /// The move hits multiple times.
    pub multihit: Option<MultihitType>,
    /// Target used if the user is not Ghost type (used for Curse).
    pub non_ghost_target: Option<MoveTarget>,
    /// Does the move target automatically adjust when the original target is gone?
    #[serde(default)]
    pub smart_target: bool,
    /// Does the move track the target, even if they have moved?
    #[serde(default)]
    pub tracks_target: bool,
    /// The move will always critical hit.
    #[serde(default)]
    pub will_crit: bool,
    /// Does the move avoid random targets?
    #[serde(default)]
    pub no_random_target: bool,

    /// Dynamic battle effects.
    #[serde(default)]
    pub effect: fxlang::Effect,
    /// Dynamic battle effects of the condition created by this move.
    #[serde(default)]
    pub condition: fxlang::Condition,
}

impl MoveData {
    /// Does the move ignore immunity?
    ///
    /// The default value of this depends on the [`MoveCategory`].
    pub fn ignore_immunity(&self) -> bool {
        self.ignore_immunity
            .unwrap_or(self.category == MoveCategory::Status)
    }
}

/// Dynamic data on how a move hit a target.
#[derive(Clone)]
pub struct MoveHitData {
    /// Did the move critical hit?
    pub crit: bool,
    /// Type modifier on the damage calculation.
    pub type_modifier: i8,
}

impl MoveHitData {
    pub fn new() -> Self {
        Self {
            crit: false,
            type_modifier: 0,
        }
    }
}

/// The current type of [`HitEffect`] being applied on an active [`Move`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MoveHitEffectType {
    PrimaryEffect,
    SecondaryEffect(usize),
}

impl MoveHitEffectType {
    /// The index of the secondary effect, if any.
    pub fn secondary_index(&self) -> Option<usize> {
        match self {
            Self::PrimaryEffect => None,
            Self::SecondaryEffect(index) => Some(*index),
        }
    }
}

/// An inidividual move, which can be used by a Mon in battle.
///
/// Unlike other move effects, [`Move`]s are mutable across multiple Mons and turns. A move used by
/// one Mon can have different effects than the ame move used by another Mon.
#[derive(Clone)]
pub struct Move {
    id: Id,
    pub data: MoveData,

    /// Custom STAB modifier, if any.
    pub stab_modifier: Option<Fraction<u32>>,

    /// The Mon that used the move.
    pub used_by: Option<MonHandle>,
    /// The move was used externally, rather than directly by a Mon through its moveset.
    pub external: bool,
    /// Whether or not the move infiltrates effects.
    pub infiltrates: bool,
    /// The source of the move, if any.
    pub source_effect: Option<EffectHandle>,
    /// Whether or not this move hit multiple targets.
    pub spread_hit: bool,
    /// Number of hits dealt by the move.
    pub hit: u8,
    /// Total damage dealt by the move.
    pub total_damage: u64,
    /// Have the primary user effect been applied?
    pub primary_user_effect_applied: bool,
    /// Has the move been reflected back at the user?
    pub reflected: bool,

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

    hit_data: FastHashMap<MonHandle, MoveHitData>,
}

impl Move {
    /// Creates a new active move, which can be modified for the use of the move.
    pub fn new(id: Id, data: MoveData) -> Self {
        Self {
            id,
            data,
            used_by: None,
            stab_modifier: None,
            external: false,
            infiltrates: false,
            source_effect: None,
            spread_hit: false,
            hit: 0,
            total_damage: 0,
            primary_user_effect_applied: false,
            effect_state: fxlang::EffectState::new(),
            unlinked: false,
            hit_data: FastHashMap::new(),
            reflected: false,
        }
    }

    /// Creates a new active move, with unlinked effect callbacks.
    pub fn new_unlinked(id: Id, data: MoveData) -> Self {
        Self {
            id,
            data,
            used_by: None,
            stab_modifier: None,
            external: false,
            infiltrates: false,
            source_effect: None,
            spread_hit: false,
            hit: 0,
            total_damage: 0,
            primary_user_effect_applied: false,
            effect_state: fxlang::EffectState::new(),
            unlinked: true,
            hit_data: FastHashMap::new(),
            reflected: false,
        }
    }

    /// Returns the hit data for the target.
    pub fn hit_data(&mut self, target: MonHandle) -> &mut MoveHitData {
        self.hit_data.entry(target).or_insert(MoveHitData::new())
    }

    /// Returns the hit data for the target, if any.
    pub fn maybe_hit_data(&self, target: MonHandle) -> Option<&MoveHitData> {
        self.hit_data.get(&target)
    }

    /// Returns a reference to the hit effect.
    pub fn target_hit_effect(&self, hit_effect_type: MoveHitEffectType) -> Option<&HitEffect> {
        match hit_effect_type {
            MoveHitEffectType::PrimaryEffect => self.data.hit_effect.as_ref(),
            MoveHitEffectType::SecondaryEffect(index) => self
                .data
                .secondary_effects
                .get(index)
                .map(|effect| effect.target.as_ref())
                .flatten(),
        }
    }

    /// Returns a mutable reference to the hit effect.
    pub fn target_hit_effect_mut(
        &mut self,
        hit_effect_type: MoveHitEffectType,
    ) -> Option<&mut HitEffect> {
        match hit_effect_type {
            MoveHitEffectType::PrimaryEffect => self.data.hit_effect.as_mut(),
            MoveHitEffectType::SecondaryEffect(index) => self
                .data
                .secondary_effects
                .get_mut(index)
                .map(|effect| effect.target.as_mut())
                .flatten(),
        }
    }

    /// Returns a reference to the hit effect on the user.
    pub fn user_hit_effect(&self, hit_effect_type: MoveHitEffectType) -> Option<&HitEffect> {
        match hit_effect_type {
            MoveHitEffectType::PrimaryEffect => self.data.user_effect.as_ref(),
            MoveHitEffectType::SecondaryEffect(index) => self
                .data
                .secondary_effects
                .get(index)
                .map(|effect| effect.user.as_ref())
                .flatten(),
        }
    }

    /// Returns a mutable reference to the hit effect on the user.
    pub fn user_hit_effect_mut(
        &mut self,
        hit_effect_type: MoveHitEffectType,
    ) -> Option<&mut HitEffect> {
        match hit_effect_type {
            MoveHitEffectType::PrimaryEffect => self.data.user_effect.as_mut(),
            MoveHitEffectType::SecondaryEffect(index) => self
                .data
                .secondary_effects
                .get_mut(index)
                .map(|effect| effect.user.as_mut())
                .flatten(),
        }
    }

    /// Returns the corresponding fxlang effect for the hit effect.
    pub fn fxlang_effect(&self, hit_effect_type: MoveHitEffectType) -> Option<&fxlang::Effect> {
        match hit_effect_type {
            MoveHitEffectType::PrimaryEffect => Some(&self.data.effect),
            MoveHitEffectType::SecondaryEffect(secondary_index) => {
                Some(&self.data.secondary_effects.get(secondary_index)?.effect)
            }
        }
    }
}

impl Identifiable for Move {
    fn id(&self) -> &Id {
        &self.id
    }
}
