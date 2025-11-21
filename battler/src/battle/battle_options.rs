use anyhow::Result;
use battler_prng::{
    PseudoRandomNumberGenerator,
    RealPseudoRandomNumberGenerator,
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    battle::{
        BattleType,
        FieldData,
        PlayerData,
        SideData,
    },
    config::FormatData,
    error::general_error,
};

/// Battle engine option for how base damage should be randomized in the damage calculation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum CoreBattleEngineRandomizeBaseDamage {
    /// Randomize the base damage.
    ///
    /// This is the default behavior.
    Randomize,
    /// Only use the maximum base damage value.
    Max,
    /// Only use the minimum base damage value.
    Min,
}

/// How the battle engine should resolve ties when sorting by speed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CoreBattleEngineSpeedSortTieResolution {
    /// Resolves ties randomly by advancing RNG.
    Random,
    /// Do not resolve ties and keep the original order of tied elements.
    Keep,
    /// Reverse the original order of tied elements.
    Reverse,
}

fn default_rng_factory() -> fn(seed: Option<u64>) -> Box<dyn PseudoRandomNumberGenerator> {
    |seed: Option<u64>| Box::new(RealPseudoRandomNumberGenerator::new(seed))
}

fn default_true() -> bool {
    true
}

fn default_100() -> u32 {
    100
}

fn default_randomize() -> CoreBattleEngineRandomizeBaseDamage {
    CoreBattleEngineRandomizeBaseDamage::Randomize
}

fn default_random() -> CoreBattleEngineSpeedSortTieResolution {
    CoreBattleEngineSpeedSortTieResolution::Random
}

/// Options that change how the battle engine itself behaves, which is not necessarily specific to
/// any individual battle.
///
/// Options defined here relate to how the battle engine is operated, so it is likely that these
/// options will be common across all battle instances.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreBattleEngineOptions {
    /// Should all teams be validated prior to the battle being able to start?
    #[serde(default = "default_true")]
    pub validate_teams: bool,

    /// Should the [`CoreBattle`][`crate::battle::CoreBattle`] automatically continue when it is
    /// able to?
    ///
    /// If set to `true`, a [`CoreBattle`][`crate::battle::CoreBattle`] object will continue the
    /// battle as soon as it finds that it is able to. The best example of this is when a
    /// player makes a choice: if all players have made responded to their request, then the
    /// battle can automatically continue in the same method as the last player's choice.
    ///
    /// If set to `false`,
    /// [`PublicCoreBattle::continue_battle`][`crate::battle::PublicCoreBattle::continue_battle`]
    /// must be called to manually continue the battle (even at the start of the battle).
    #[serde(default = "default_true")]
    pub auto_continue: bool,

    /// Should the [`CoreBattle`][`crate::battle::CoreBattle`] reveal the actual health of all Mons
    /// in the public battle log?
    ///
    /// By default, the public log will show the health of all Mons as a percentage (fraction out
    /// of 100). If this option is set to `true`, the battle will show the actual HP stat of each
    /// Mon.
    #[serde(default)]
    pub reveal_actual_health: bool,

    /// A custom denominator for public health logs.
    ///
    /// By default, public health is revealed as a percentage out of 100. A different base can be
    /// used for higher or lower precision.
    #[serde(default = "default_100")]
    pub public_health_base: u32,

    /// Function for creating the battle's random number generator.
    ///
    /// Primarily useful for tests where we wish to have fine-grained control over battle RNG.
    #[serde(skip, default = "default_rng_factory")]
    pub rng_factory: fn(seed: Option<u64>) -> Box<dyn PseudoRandomNumberGenerator>,

    /// Are players allowed to pass for unfainted Mons?
    ///
    /// By default, "pass" actions are forced when the player does not have enough Mons to fulfill
    /// all requirements. For example if a player has a team of 3 Mons for a doubles battle and 2
    /// faint at the same time, the player will is allowed to send one "switch" action and the
    /// other is forced to be a pass
    ///
    /// In all other cases, players cannot instruct their Mons to pass at the beginning of a turn.
    /// This prevents battles from getting into a stalemate position forever.
    ///
    /// If this property is set to `true`, players will be allowed to send "pass" actions. This is
    /// mostly useful for tests where we want to control one side while the other side sits
    /// passively.
    #[serde(default)]
    pub allow_pass_for_unfainted_mon: bool,

    /// Describes how base damage should be randomized in the damage calculation.
    ///
    /// By default, base damage is randomized early in the damage calculation. This property can
    /// control how the damage should be randomized. This is useful for tests against the damage
    /// calculator to discover the minimum and maximum damage values.
    #[serde(default = "default_randomize")]
    pub randomize_base_damage: CoreBattleEngineRandomizeBaseDamage,

    /// Describes how ties should be resolved when sorting elements by speed.
    ///
    /// By default, speed ties are resolved randomly. However, many tests involve a lot of speed
    /// ties, complicating test results when RNG shifts slightly. This property can be used to
    /// avoid using RNG in speed sorting completely.
    #[serde(default = "default_random")]
    pub speed_sort_tie_resolution: CoreBattleEngineSpeedSortTieResolution,

    /// Should volatile statuses be logged?
    ///
    /// By default, volatile statuses are invisible to Mons, since they are used to implement
    /// complex interactions in the battle system. It may be helpful, especially for debugging
    /// purposes, to view all volatile statuses added to and removed from Mons through the course
    /// of a battle.
    #[serde(default)]
    pub log_volatile_statuses: bool,

    /// Should side conditions be logged?
    ///
    /// By default, side conditions are invisible to Mons unless explicitly logged. It may be
    /// helpful, especially for debugging purposes, to view all side conditions added to and
    /// removed from sides through the course of a battle.
    #[serde(default)]
    pub log_side_conditions: bool,

    /// Should slot conditions be logged?
    ///
    /// By default, slot conditions are invisible to Mons unless explicitly logged. It may be
    /// helpful, especially for debugging purposes, to view all slot conditions added to and
    /// removed from sides through the course of a battle.
    #[serde(default)]
    pub log_slot_conditions: bool,

    /// Should identical Mon names for a single player be disambiguated?
    ///
    /// If set to true, Mons with the same name for a single player will have a disambiguation
    /// string appended to their name of the pattern `###N` (where `N` is a number). Clients can
    /// simply strip off this string when displaying the Mon name.
    #[serde(default)]
    pub disambiguate_identical_names: bool,

    /// Should catch rates and shake probabilities be logged?
    ///
    /// Helpful for debugging.
    #[serde(default)]
    pub log_catch_rate: bool,
}

impl Default for CoreBattleEngineOptions {
    fn default() -> Self {
        Self {
            validate_teams: true,
            auto_continue: true,
            reveal_actual_health: false,
            public_health_base: 100,
            rng_factory: default_rng_factory(),
            allow_pass_for_unfainted_mon: false,
            randomize_base_damage: CoreBattleEngineRandomizeBaseDamage::Randomize,
            speed_sort_tie_resolution: CoreBattleEngineSpeedSortTieResolution::Random,
            log_volatile_statuses: false,
            log_side_conditions: false,
            log_slot_conditions: false,
            disambiguate_identical_names: false,
            log_catch_rate: false,
        }
    }
}

/// Core options for a new battle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoreBattleOptions {
    /// The initial seed for random number generation.
    ///
    /// This can be used to effectively replay or control a battle.
    pub seed: Option<u64>,
    /// The format of the battle.
    pub format: FormatData,
    /// The field of the battle.
    #[serde(default)]
    pub field: FieldData,
    /// One side of the battle.
    pub side_1: SideData,
    /// The other side of the battle.
    pub side_2: SideData,
}

impl CoreBattleOptions {
    fn validate_side(&self, side: &SideData) -> Result<()> {
        let players_on_side = side.players.len();
        if players_on_side == 0 {
            return Err(general_error(format!("{} has no players", side.name)));
        }
        match self.format.battle_type {
            BattleType::Singles => {
                if players_on_side > 1 {
                    return Err(general_error(format!(
                        "{} has too many players for a singles battle",
                        side.name
                    )));
                }
            }
            BattleType::Doubles => {
                if players_on_side > 1 {
                    return Err(general_error(format!(
                        "{} has too many players for a doubles battle (did you mean to start a multi battle?)",
                        side.name
                    )));
                }
            }
            _ => (),
        }
        for player in &side.players {
            self.validate_player(side, player)?;
        }
        Ok(())
    }

    fn validate_player(&self, _: &SideData, _: &PlayerData) -> Result<()> {
        Ok(())
    }

    /// Validates the battle options.
    pub fn validate(&self) -> Result<()> {
        self.validate_side(&self.side_1)?;
        self.validate_side(&self.side_2)?;
        Ok(())
    }
}
