use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    battle::PartialBoostTable,
    common::{
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HitEffect {
    /// Stat boosts.
    pub boosts: Option<PartialBoostTable>,
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
}

/// Secondary effect that occurs after a move is used.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecondaryEffect {
    /// Chance of the effect occurring.
    pub chance: Option<Fraction>,
    /// Secondary hit effect on the user of the move.
    pub user: HitEffect,
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
    pub ohko_type: Option<OhkoType>,
    /// Thaws the target?
    #[serde(default)]
    pub thaws_target: bool,
    /// Percentage of target's HP to heal.
    pub heal_percent: Option<u8>,
    /// Force the target to switch out?
    #[serde(default)]
    pub force_switch: bool,
    /// Type of switch that occurs on the user.
    pub user_switch: Option<UserSwitchType>,
    /// Type of stat boosts on the user.
    pub user_boosts: Option<PartialBoostTable>,
    /// How the user self destructs.
    pub self_destruct: Option<SelfDestructType>,
    /// Does the move break protect?
    #[serde(default)]
    pub breaks_protect: bool,
    /// The percentage of the target's HP to damage for recoil.
    pub recoil_percent: Option<u8>,
    /// The percentage of the target's HP to drain.
    pub drain_percent: Option<u8>,
    /// Apply Struggle recoil?
    #[serde(default)]
    pub struggle_recoil: bool,

    /// Secondary effects applied to the target.
    #[serde(default)]
    pub secondary_effects: Vec<SecondaryEffect>,
    /// A secondary effect on the user.
    pub user_secondary_effect: Option<SecondaryEffect>,

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
    pub crit_ratio: Option<u32>,
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

/// An inidividual move, which can be used by a Mon in battle.
#[derive(Clone)]
pub struct Move {
    /// Move data.
    pub data: MoveData,
    id: Id,
}

impl Move {
    /// Creates a new [`Move`] instance from [`MoveData`].
    pub fn new(data: MoveData) -> Self {
        let id = Id::from(data.name.as_ref());
        Self { data, id }
    }
}

impl Identifiable for Move {
    fn id(&self) -> &Id {
        &self.id
    }
}
