use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    battle::{
        BattleOptions,
        CoreBattleEngineOptions,
        CoreBattleOptions,
        FieldData,
        PlayerData,
        PlayerOptions,
        PlayerType,
        PublicCoreBattle,
        SideData,
    },
    battler_error,
    common::{
        Error,
        FastHashMap,
        WrapResultError,
    },
    config::{
        Format,
        FormatData,
    },
    dex::{
        DataStore,
        Dex,
    },
    teams::{
        TeamData,
        TeamValidationError,
        TeamValidator,
    },
};

/// Player data for a [`BattleBuilder`].
#[derive(Debug, Serialize, Deserialize)]
pub struct BattleBuilderPlayerData {
    /// Unique identifier.
    pub id: String,
    /// Player's display name.
    pub name: String,
    /// Player type.
    #[serde(default)]
    pub player_type: PlayerType,
    /// Player options.
    #[serde(default)]
    pub player_options: PlayerOptions,
}

impl Into<PlayerData> for BattleBuilderPlayerData {
    fn into(self) -> PlayerData {
        PlayerData {
            id: self.id,
            name: self.name,
            team: TeamData::default(),
            player_type: self.player_type,
            player_options: self.player_options,
        }
    }
}

/// Side data for a [`BattleBuilder`].
#[derive(Debug, Serialize, Deserialize)]
pub struct BattleBuilderSideData {
    /// Side name.
    pub name: String,
    /// Players on the side.
    pub players: Vec<BattleBuilderPlayerData>,
}

impl Into<SideData> for BattleBuilderSideData {
    fn into(self) -> SideData {
        SideData {
            name: self.name,
            players: self
                .players
                .into_iter()
                .map(|player| player.into())
                .collect(),
        }
    }
}

/// Options for building a new battle.
#[derive(Debug, Serialize, Deserialize)]
pub struct BattleBuilderOptions {
    /// The initial seed for random number generation.
    ///
    /// This can be used to effectively replay or control a battle.
    pub seed: Option<u64>,
    /// The format of the battle.
    pub format: FormatData,
    /// The field of the battle.
    pub field: FieldData,
    /// One side of the battle.
    pub side_1: BattleBuilderSideData,
    /// The other side of the battle.
    pub side_2: BattleBuilderSideData,
}

/// Object for dynamically building a battle prior to starting it.
///
/// This object can be used primarily to validate things about the battle before actually starting
/// the battle. For example, a client may be interested in validating all teams prior to starting
/// a battle. Since it will be too late to change data once the battle has started, validation can
/// be done here instead.
pub struct BattleBuilder<'d> {
    dex: Dex<'d>,
    format: Format,
    options: CoreBattleOptions,

    player_ids: FastHashMap<String, (usize, usize)>,
}

impl<'d> BattleBuilder<'d> {
    /// Constructs a new battle builder object.
    pub fn new(options: BattleBuilderOptions, data: &'d dyn DataStore) -> Result<Self, Error> {
        let dex = Dex::new(data)?;
        let format = Format::new(options.format, &dex)?;
        let options = CoreBattleOptions {
            seed: options.seed,
            format: None,
            field: options.field,
            side_1: options.side_1.into(),
            side_2: options.side_2.into(),
        };
        let sides = [&options.side_1, &options.side_2];
        let player_ids = sides
            .iter()
            .enumerate()
            .map(|(side_index, side)| {
                side.players
                    .iter()
                    .enumerate()
                    .map(move |(player_index, player)| {
                        (player.id.clone(), (side_index, player_index))
                    })
            })
            .flatten()
            .collect();
        Ok(Self {
            dex,
            format,
            options,
            player_ids,
        })
    }

    /// Builds a new battle instance using data from the builder.
    pub fn build(
        mut self,
        engine_options: CoreBattleEngineOptions,
    ) -> Result<PublicCoreBattle<'d>, Error> {
        self.validate_battle_options()?;
        self.options.validate_with_format(&self.format.data())?;
        PublicCoreBattle::from_builder(self.options, self.dex, self.format, engine_options)
    }

    fn validate_battle_options(&mut self) -> Result<(), Error> {
        self.validate_core_battle_options()?;
        Ok(())
    }

    fn validate_core_battle_options(&mut self) -> Result<(), Error> {
        for clause in self.format.rules.clauses(&self.dex) {
            clause.on_validate_core_battle_options(&self.format.rules, &mut self.options)?;
        }
        Ok(())
    }

    /// Validates the given team against the battle format.
    ///
    /// Note that team validation can modify the team (for example, when forcing forme changes), so
    /// callers should be sure that the modified team is recorded properly.
    pub fn validate_team(&self, team: &mut TeamData) -> Result<(), TeamValidationError> {
        let validator = TeamValidator::new(&self.format, &self.dex);
        validator.validate_team(team.members.iter_mut().collect::<Vec<_>>().as_mut())
    }

    fn side_mut(&mut self, side: usize) -> Result<&mut SideData, Error> {
        match side {
            0 => Ok(&mut self.options.side_1),
            1 => Ok(&mut self.options.side_2),
            _ => Err(battler_error!("side {side} is invalid")),
        }
    }

    /// Updates a player's team.
    ///
    /// If validation should be done, [`Self::validate_team`] should be called
    /// first.
    pub fn update_team(&mut self, player: &str, team: TeamData) -> Result<(), Error> {
        let (side, player) = self
            .player_ids
            .get(player)
            .wrap_error_with_format(format_args!("player {player} does not exist"))?
            .clone();
        let player = self
            .side_mut(side)?
            .players
            .get_mut(player)
            .wrap_error_with_format(format_args!("index {player} is invalid for side {side}"))?;
        player.team = team;
        Ok(())
    }
}

#[cfg(test)]
mod battle_builder_tests {
    use std::iter;

    use serde::Deserialize;

    use crate::{
        battle::{
            BattleBuilder,
            BattleBuilderOptions,
            CoreBattleEngineOptions,
        },
        common::{
            read_test_cases,
            FastHashMap,
        },
        dex::LocalDataStore,
        teams::TeamData,
    };

    fn battle_builder_options() -> BattleBuilderOptions {
        serde_json::from_str(
            r#"{
                "format": {
                    "battle_type": "singles",
                    "rules": [
                        "Min Team Size = 4",
                        "Force Level = 50"
                    ]
                },
                "side_1": {
                    "name": "Side 1",
                    "players": [
                        {
                            "id": "1",
                            "name": "Player 1"
                        }
                    ]
                },
                "side_2": {
                    "name": "Side 2",
                    "players": [
                        {
                            "id": "2",
                            "name": "Player 2"
                        }
                    ]
                }
            }"#,
        )
        .unwrap()
    }

    #[test]
    fn fails_to_build_empty_team() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let builder = BattleBuilder::new(battle_builder_options(), &data).unwrap();
        assert!(builder
            .build(CoreBattleEngineOptions::default())
            .err()
            .unwrap()
            .to_string()
            .contains("a player has an empty team"));
    }

    #[test]
    fn fails_to_build_empty_side() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut options = battle_builder_options();
        options.side_1.players.clear();
        let builder = BattleBuilder::new(options, &data).unwrap();
        assert!(builder
            .build(CoreBattleEngineOptions::default())
            .err()
            .unwrap()
            .to_string()
            .contains("side Side 1 has no players"));
    }

    #[test]
    fn fails_to_update_team_for_nonexistent_player() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut builder = BattleBuilder::new(battle_builder_options(), &data).unwrap();
        let team = serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Bulba Fett",
                        "species": "Bulbasaur",
                        "ability": "Overgrow",
                        "moves": [],
                        "nature": "Adamant",
                        "gender": "M",
                        "level": 100
                    }
                ]
            }"#,
        )
        .unwrap();
        assert!(builder
            .update_team("unknown", team)
            .err()
            .unwrap()
            .to_string()
            .contains("player unknown does not exist"));
    }

    #[test]
    fn dynamically_validates_and_updates_team() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let mut builder = BattleBuilder::new(battle_builder_options(), &data).unwrap();
        let mut team = serde_json::from_str(
            r#"{
                "members": [
                    {
                        "name": "Bulba Fett",
                        "species": "Bulbasaur",
                        "ability": "Overgrow",
                        "moves": [],
                        "nature": "Adamant",
                        "gender": "M",
                        "level": 100
                    }
                ]
            }"#,
        )
        .unwrap();
        let result = builder.validate_team(&mut team);
        assert_eq!(
            result.err().unwrap().problems,
            vec!["You must bring at least 4 Mons (your team has 1)."]
        );
        let new_members = iter::repeat_with(|| team.members[0].clone())
            .take(4)
            .collect();
        team.members = new_members;
        let result = builder.validate_team(&mut team);
        assert!(result.is_ok());
        assert!(team.members.iter().all(|mon| mon.level == 50));
        // Update both teams so that we should be able to build a valid battle.
        assert!(builder.update_team("1", team.clone()).is_ok());
        assert!(builder.update_team("2", team).is_ok());
        assert!(iter::once(&builder.options.side_1)
            .chain(iter::once(&builder.options.side_2))
            .all(|side| side
                .players
                .iter()
                .all(|player| player.team.members.len() == 4)));
        assert!(builder.build(CoreBattleEngineOptions::default()).is_ok());
    }

    #[derive(Deserialize)]
    struct BattleBuilderBuildTestCases {
        options: BattleBuilderOptions,
        teams: FastHashMap<String, TeamData>,
        ok: bool,
        expected_error_substr: Option<String>,
    }

    #[test]
    fn battle_builder_build_test_cases() {
        let test_cases =
            read_test_cases::<BattleBuilderBuildTestCases>("battle_builder_build_tests.json")
                .unwrap();
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        for (test_name, test_case) in test_cases {
            let mut builder = BattleBuilder::new(test_case.options, &data).unwrap();
            for (player_id, team) in test_case.teams {
                assert!(builder.update_team(&player_id, team).is_ok());
            }
            let result = builder.build(CoreBattleEngineOptions::default());
            assert_eq!(
                result.is_ok(),
                test_case.ok,
                "Invalid result for {test_name}: error is {:?}",
                result.err()
            );
            if let Some(expected_error_susbtr) = test_case.expected_error_substr {
                assert!(
                    result
                        .as_ref()
                        .err()
                        .clone()
                        .unwrap()
                        .to_string()
                        .contains(&expected_error_susbtr),
                    "Missing error substring for {test_name}: error is {:?}",
                    result.err()
                );
            }
        }
    }
}
