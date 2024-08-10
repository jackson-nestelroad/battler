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
    battler_error,
    common::Error,
    config::FormatData,
    rng::{
        PseudoRandomNumberGenerator,
        RealPseudoRandomNumberGenerator,
    },
};

/// Battle engine option for how base damage should be randomized in the damage calculation.
#[derive(Debug)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreBattleEngineSpeedSortTieResolution {
    /// Resolves ties randomly by advancing RNG.
    Random,
    /// Do not resolve ties and keep the original order of tied elements.
    Keep,
    /// Reverse the original order of tied elements.
    Reverse,
}

/// Options that change how the battle engine itself behaves, which is not necessarily specific to
/// any individual battle.
///
/// Options defined here relate to how the battle engine is operated, so it is likely that these
/// options will be common across all battle instances.
#[derive(Debug)]
pub struct CoreBattleEngineOptions {
    /// Should the [`Battle`][`crate::battle::CoreBattle`] automatically continue when it is able
    /// to?
    ///
    /// If set to `true`, a [`Battle`][`crate::battle::CoreBattle`] object will continue the battle
    /// as soon as it finds that it is able to. The best example of this is when a player makes
    /// a choice: if all players have made responded to their request, then the battle can
    /// automatically continue in the same method as the last player's choice.
    ///
    /// If set to `false`, [`Battle::continue_battle`] must be called to manually continue the
    /// battle (even at the start of the battle).
    pub auto_continue: bool,

    /// Should the [`Battle`][`crate::battle::CoreBattle`] reveal the actual health of all Mons in
    /// the public battle logs?
    ///
    /// By default, the public logs will show the health of all Mons as a percentage (fraction out
    /// of 100). If this option is set to `true`, the battle will show the actual HP stat of each
    /// Mon.
    pub reveal_actual_health: bool,

    /// Function for creating the battle's random number generator.
    ///
    /// Primarily useful for tests where we wish to have fine-grained control over battle RNG.
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
    pub allow_pass_for_unfainted_mon: bool,

    /// Describes how base damage should be randomized in the damage calculation.
    ///
    /// By default, base damage is randomized early in the damage calculation. This property can
    /// control how the damage should be randomized. This is useful for tests against the damage
    /// calculator to discover the minimum and maximum damage values.
    pub randomize_base_damage: CoreBattleEngineRandomizeBaseDamage,

    /// Describes how ties should be resolved when sorting elements by speed.
    ///
    /// By default, speed ties are resolved randomly. However, many tests involve a lot of speed
    /// ties, complicating test results when RNG shifts slightly. This property can be used to
    /// avoid using RNG in speed sorting completely.
    pub speed_sort_tie_resolution: CoreBattleEngineSpeedSortTieResolution,

    /// Should volatile statuses be logged?
    ///
    /// By default, volatile statuses are invisible to Mons, since they are used to implement
    /// complex interactions in the battle system. It may be helpful, especially for debugging
    /// purposes, to view all volatile statuses added to and removed from Mons through the course
    /// of a battle.
    pub log_volatile_statuses: bool,

    /// Should side conditions be logged?
    ///
    /// By default, side conditions are invisible to Mons unless explicitly logged. It may be
    /// helpful, especially for debugging purposes, to view all side conditions added to and
    /// removed from sides through the course of a battle.
    pub log_side_conditions: bool,

    /// Should slot conditions be logged?
    ///
    /// By default, slot conditions are invisible to Mons unless explicitly logged. It may be
    /// helpful, especially for debugging purposes, to view all slot conditions added to and
    /// removed from sides through the course of a battle.
    pub log_slot_conditions: bool,
}

impl Default for CoreBattleEngineOptions {
    fn default() -> Self {
        Self {
            auto_continue: true,
            reveal_actual_health: false,
            rng_factory: |seed: Option<u64>| Box::new(RealPseudoRandomNumberGenerator::new(seed)),
            allow_pass_for_unfainted_mon: false,
            randomize_base_damage: CoreBattleEngineRandomizeBaseDamage::Randomize,
            speed_sort_tie_resolution: CoreBattleEngineSpeedSortTieResolution::Random,
            log_volatile_statuses: false,
            log_side_conditions: false,
            log_slot_conditions: false,
        }
    }
}

/// Common trait for different battle options.
pub trait BattleOptions {
    /// Validates the battle options.
    fn validate(&self) -> Result<(), Error>;
    /// Validates the battle options for the given format.
    fn validate_with_format(&self, format: &FormatData) -> Result<(), Error>;
}

/// Core options for a new battle.
#[derive(Debug, Serialize, Deserialize)]
pub struct CoreBattleOptions {
    /// The initial seed for random number generation.
    ///
    /// This can be used to effectively replay or control a battle.
    pub seed: Option<u64>,
    /// The format of the battle.
    pub format: Option<FormatData>,
    /// The field of the battle.
    #[serde(default)]
    pub field: FieldData,
    /// One side of the battle.
    pub side_1: SideData,
    /// The other side of the battle.
    pub side_2: SideData,
}

impl CoreBattleOptions {
    fn validate_side(&self, format: &FormatData, side: &SideData) -> Result<(), Error> {
        let players_on_side = side.players.len();
        if players_on_side == 0 {
            return Err(battler_error!("side {} has no players", side.name));
        }
        match format.battle_type {
            BattleType::Singles => {
                if players_on_side > 1 {
                    return Err(battler_error!(
                        "side {} has too many players for a singles battle",
                        side.name
                    ));
                }
            }
            BattleType::Doubles => {
                if players_on_side > 1 {
                    return Err(battler_error!(
                        "side {} has too many players for a doubles battle (did you mean to start a multi battle?)",
                        side.name
                    ));
                }
            }
            _ => (),
        }
        for player in &side.players {
            self.validate_player(format, side, player)?;
        }
        Ok(())
    }

    fn validate_player(
        &self,
        _: &FormatData,
        _: &SideData,
        player: &PlayerData,
    ) -> Result<(), Error> {
        if player.team.members.is_empty() {
            return Err(battler_error!("a player has an empty team"));
        }
        Ok(())
    }
}

impl BattleOptions for CoreBattleOptions {
    fn validate(&self) -> Result<(), Error> {
        match &self.format {
            Some(format) => self.validate_with_format(format),
            None => Err(battler_error!("battle options has no format data")),
        }
    }

    fn validate_with_format(&self, format: &FormatData) -> Result<(), Error> {
        self.validate_side(format, &self.side_1)?;
        self.validate_side(format, &self.side_2)?;
        Ok(())
    }
}

#[cfg(test)]
mod battle_options_tests {
    use serde::Deserialize;

    use crate::{
        battle::{
            BattleOptions,
            CoreBattleOptions,
        },
        common::read_test_cases,
    };

    #[derive(Deserialize)]
    struct BattleOptionsValidateTestCase {
        options: CoreBattleOptions,
        ok: bool,
        expected_error_substr: Option<String>,
    }

    #[test]
    fn battle_options_validate_test_cases() {
        let test_cases =
            read_test_cases::<BattleOptionsValidateTestCase>("battle_options_validate_tests.json")
                .unwrap();
        for (test_name, test_case) in test_cases {
            let result = test_case.options.validate();
            assert_eq!(
                result.is_ok(),
                test_case.ok,
                "Invalid result for {test_name}: {result:?}"
            );
            if let Some(expected_error_susbtr) = test_case.expected_error_substr {
                assert!(
                    result
                        .clone()
                        .err()
                        .unwrap()
                        .to_string()
                        .contains(&expected_error_susbtr),
                    "Missing error substring for {test_name}: {result:?}"
                );
            }
        }
    }
}
