use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    battle::{
        BattleEngineOptions,
        BattleOptions,
        CoreBattle,
        CoreBattleOptions,
        PlayerData,
        SideData,
        TimedBattleOptions,
        TimerOptions,
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
}

impl Into<PlayerData> for BattleBuilderPlayerData {
    fn into(self) -> PlayerData {
        PlayerData {
            id: self.id,
            name: self.name,
            team: TeamData::default(),
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
    /// One side of the battle.
    pub side_1: BattleBuilderSideData,
    /// The other side of the battle.
    pub side_2: BattleBuilderSideData,
    /// Timer settings that control the overall game timer.
    #[serde(default)]
    pub timer: TimerOptions,
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
    options: TimedBattleOptions,

    player_ids: FastHashMap<String, (usize, usize)>,
}

impl<'d> BattleBuilder<'d> {
    /// Constructs a new battle builder object.
    pub fn new(options: BattleBuilderOptions, data: &'d dyn DataStore) -> Result<Self, Error> {
        let dex = Dex::new(data);
        let format = Format::new(options.format, &dex)?;
        let options = TimedBattleOptions {
            core: CoreBattleOptions {
                seed: options.seed,
                format: None,
                side_1: options.side_1.into(),
                side_2: options.side_2.into(),
            },
            timer: TimerOptions::default(),
        };
        let sides = [&options.core.side_1, &options.core.side_2];
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
    pub fn build(self, engine_options: BattleEngineOptions) -> Result<CoreBattle<'d>, Error> {
        self.options.validate_with_format(&self.format.data())?;
        CoreBattle::from_builder(self.options.core, self.dex, self.format, engine_options)
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
            0 => Ok(&mut self.options.core.side_1),
            1 => Ok(&mut self.options.core.side_2),
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
mod battle_builder_test {
    use std::iter;

    use crate::{
        battle::{
            BattleBuilder,
            BattleBuilderOptions,
            BattleEngineOptions,
        },
        dex::LocalDataStore,
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
            .build(BattleEngineOptions::default())
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
            .build(BattleEngineOptions::default())
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
                        "ball": "Normal",
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
                        "ball": "Normal",
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
        assert!(iter::once(&builder.options.core.side_1)
            .chain(iter::once(&builder.options.core.side_2))
            .all(|side| side
                .players
                .iter()
                .all(|player| player.team.members.len() == 4)));
        assert!(builder.build(BattleEngineOptions::default()).is_ok());
    }
}
