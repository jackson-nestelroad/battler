use ahash::HashMapExt;
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    battle::{
        MonHandle,
        PartialBoostTable,
    },
    common::{
        FastHashMap,
        FastHashSet,
        Fraction,
        Id,
        Identifiable,
    },
    mons::{
        Stat,
        Type,
    },
    moves::{
        Accuracy,
        DamageType,
        MonOverride,
        MoveCategory,
        MoveFlags,
        MoveTarget,
        MultihitType,
        OhkoType,
        SelfDestructType,
        UserSwitchType,
    },
};

/// The effect of being hit by a move.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct HitEffect {
    /// Stat boosts.
    pub boosts: Option<PartialBoostTable>,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecondaryEffect {
    /// Chance of the effect occurring.
    pub chance: Option<Fraction<u16>>,
    /// Secondary hit effect on the target.
    pub target: HitEffect,
    /// Secondary hit effect on the user of the move.
    pub user: HitEffect,
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
    pub pp: u8,
    /// Move priority.
    #[serde(default)]
    pub priority: i8,
    /// Move target(s).
    pub target: MoveTarget,
    /// Move flags.
    pub flags: FastHashSet<MoveFlags>,

    /// Static damage dealt.
    pub damage: Option<DamageType>,
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
    pub user_switch: Option<UserSwitchType>,
    /// How the user self destructs.
    pub self_destruct: Option<SelfDestructType>,
    /// Does the move break protect?
    #[serde(default)]
    pub breaks_protect: bool,
    /// The percentage of the target's HP to damage for recoil.
    pub recoil_percent: Option<Fraction<u16>>,
    /// The percentage of the target's HP to drain.
    pub drain_percent: Option<Fraction<u16>>,
    /// Apply Struggle recoil?
    #[serde(default)]
    pub struggle_recoil: bool,
    /// Ignore STAB?
    #[serde(default)]
    pub ignore_stab: bool,

    /// Primary effect applied to the target.
    ///
    /// Applied when the moev hits.
    pub hit_effect: Option<HitEffect>,
    /// Primary effect on the user.
    ///
    /// Applied when the move hits.
    pub user_effect: Option<HitEffect>,
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
    /// By default, Def is used  for physical moves and SpD is used for special moves.
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
    pub ignore_immunity: bool,
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
    /// Is the move usable when the user is asleep?
    #[serde(default)]
    pub sleep_usable: bool,
    /// Does the move target automatically adjust when the original target is gone?
    #[serde(default)]
    pub smart_target: bool,
    /// Does the move track the target, even if they have moved?
    #[serde(default)]
    pub tracks_target: bool,
    /// The move will always critical hit.
    #[serde(default)]
    pub will_crit: bool,

    /// Does the move have crash damage?
    #[serde(default)]
    pub has_crash_damage: bool,
    /// Moves that should be excluded from Metronome.
    #[serde(default)]
    pub no_metronome: Vec<String>,
    /// The move cannot be sketched.
    #[serde(default)]
    pub no_sketch: bool,
    /// The move stalls the battle.
    #[serde(default)]
    pub stalling_move: bool,
}

impl MoveData {
    pub fn ignore_immunity(&self) -> bool {
        self.category == MoveCategory::Status || self.ignore_immunity
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
#[derive(Clone, Copy)]
pub enum MoveHitEffectType {
    PrimaryEffect,
    SecondaryEffect(usize),
}

/// An inidividual move, which can be used by a Mon in battle.
///
/// Unlike other move effects, [`Move`]s are mutable across multiple Mons and turns. A move used by
/// one Mon can have different effects than the ame move used by another Mon.
#[derive(Clone)]
pub struct Move {
    /// Move data.
    pub data: MoveData,
    id: Id,

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

    hit_data: FastHashMap<MonHandle, MoveHitData>,
}

impl Move {
    /// Creates a new [`Move`] instance from [`MoveData`].
    pub fn new(data: MoveData) -> Self {
        let id = Id::from(data.name.as_ref());
        Self {
            data,
            id,
            used_by: None,
            stab_modifier: None,
            external: false,
            spread_hit: false,
            hit: 0,
            total_damage: 0,
            hit_data: FastHashMap::new(),
        }
    }

    pub fn hit_data(&mut self, target: MonHandle) -> &mut MoveHitData {
        self.hit_data.entry(target).or_insert(MoveHitData::new())
    }

    pub fn target_hit_effect(&self, hit_effect_type: MoveHitEffectType) -> Option<&HitEffect> {
        match hit_effect_type {
            MoveHitEffectType::PrimaryEffect => self.data.hit_effect.as_ref(),
            MoveHitEffectType::SecondaryEffect(index) => self
                .data
                .secondary_effects
                .get(index)
                .map(|effect| &effect.target),
        }
    }

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
                .map(|effect| &mut effect.target),
        }
    }

    pub fn user_hit_effect(&self, hit_effect_type: MoveHitEffectType) -> Option<&HitEffect> {
        match hit_effect_type {
            MoveHitEffectType::PrimaryEffect => self.data.user_effect.as_ref(),
            MoveHitEffectType::SecondaryEffect(index) => self
                .data
                .secondary_effects
                .get(index)
                .map(|effect| &effect.user),
        }
    }

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
                .map(|effect| &mut effect.user),
        }
    }
}

impl Identifiable for Move {
    fn id(&self) -> &Id {
        &self.id
    }
}
