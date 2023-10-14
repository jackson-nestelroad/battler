use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    battle::{
        BattleType,
        PlayerData,
        SideData,
        TimerOptions,
    },
    battler_error,
    common::Error,
    config::FormatData,
};

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

/// Options for a timed battle.
#[derive(Debug, Serialize, Deserialize)]
pub struct TimedBattleOptions {
    /// Core battle options.
    #[serde(flatten)]
    pub core: CoreBattleOptions,
    /// Timer options.
    pub timer: TimerOptions,
}

impl BattleOptions for TimedBattleOptions {
    fn validate(&self) -> Result<(), Error> {
        self.core.validate()
    }

    fn validate_with_format(&self, format: &FormatData) -> Result<(), Error> {
        self.core.validate_with_format(format)
    }
}

#[cfg(test)]
mod battle_options_test {
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
