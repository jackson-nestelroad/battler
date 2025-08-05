use ahash::HashSet;
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    Accuracy,
    BoostTable,
    Fraction,
    MonOverride,
    MoveCategory,
    MoveFlag,
    MoveTarget,
    MultihitType,
    OhkoType,
    SelfDestructType,
    Stat,
    SwitchType,
    Type,
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

/// Data about a secondary effect that occurs after a move is used.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SecondaryEffectData {
    /// Chance of the effect occurring.
    pub chance: Option<Fraction<u16>>,
    /// Secondary hit effect on the target.
    pub target: Option<HitEffect>,
    /// Secondary hit effect on the user of the move.
    pub user: Option<HitEffect>,
    /// Dynamic battle effects.
    #[serde(default)]
    pub effect: serde_json::Value,
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
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
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
    pub flags: HashSet<MoveFlag>,

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
    pub secondary_effects: Vec<SecondaryEffectData>,

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
    pub effect: serde_json::Value,
    /// Dynamic battle effects of the condition created by this move.
    #[serde(default)]
    pub condition: serde_json::Value,
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
