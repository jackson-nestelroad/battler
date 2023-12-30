use std::str::FromStr;

use ahash::{
    HashMapExt,
    HashSetExt,
};
use battler::{
    battle::{
        BattleBuilder,
        BattleBuilderOptions,
        BattleBuilderPlayerData,
        BattleBuilderSideData,
        BattleEngineOptions,
        BattleType,
        PublicCoreBattle,
        TimerOptions,
    },
    battler_error,
    common::{
        Error,
        FastHashMap,
    },
    config::{
        FormatData,
        Rule,
        SerializedRuleSet,
    },
    dex::DataStore,
    teams::TeamData,
};

/// [`BattleBuilder`] object for integration tests.
pub struct TestBattleBuilder {
    options: BattleBuilderOptions,
    engine_options: BattleEngineOptions,
    teams: FastHashMap<String, TeamData>,
}

impl TestBattleBuilder {
    /// Creates a new [`TestBattleBuilder`].
    pub fn new() -> Self {
        Self {
            options: BattleBuilderOptions {
                seed: None,
                format: FormatData {
                    battle_type: BattleType::Singles,
                    rules: SerializedRuleSet::new(),
                },
                side_1: BattleBuilderSideData {
                    name: "Side 1".to_string(),
                    players: Vec::new(),
                },
                side_2: BattleBuilderSideData {
                    name: "Side 2".to_string(),
                    players: Vec::new(),
                },
                timer: TimerOptions::default(),
            },
            engine_options: BattleEngineOptions::default(),
            teams: FastHashMap::new(),
        }
    }

    /// Builds a new [`CoreBattle`] from the battle builder.
    pub fn build(self, data: &dyn DataStore) -> Result<PublicCoreBattle, Error> {
        let mut builder = BattleBuilder::new(self.options, data)?;
        for (player_id, mut team) in self.teams {
            let validation = builder.validate_team(&mut team);
            if let Err(error) = validation {
                return Err(battler_error!(
                    "team for player {player_id} is invalid: {error}"
                ));
            }
            builder.update_team(&player_id, team)?;
        }
        builder.build(self.engine_options)
    }

    fn format(&mut self) -> &mut FormatData {
        &mut self.options.format
    }

    pub fn with_auto_continue(mut self, auto_continue: bool) -> Self {
        self.engine_options.auto_continue = auto_continue;
        self
    }

    pub fn with_battle_type(mut self, battle_type: BattleType) -> Self {
        self.format().battle_type = battle_type;
        self
    }

    pub fn with_rule(mut self, rule: &str) -> Self {
        let rule = Rule::from_str(rule).unwrap();
        self.format().rules.insert(rule);
        self
    }

    pub fn with_seed(mut self, seed: u64) -> Self {
        self.options.seed = Some(seed);
        self
    }

    pub fn add_player_to_side_1(mut self, id: &str, name: &str) -> Self {
        self.options.side_1.players.push(BattleBuilderPlayerData {
            id: id.to_owned(),
            name: name.to_owned(),
        });
        self
    }

    pub fn add_player_to_side_2(mut self, id: &str, name: &str) -> Self {
        self.options.side_2.players.push(BattleBuilderPlayerData {
            id: id.to_owned(),
            name: name.to_owned(),
        });
        self
    }

    pub fn with_team(mut self, player_id: &str, team: TeamData) -> Self {
        self.teams.insert(player_id.to_owned(), team);
        self
    }
}
