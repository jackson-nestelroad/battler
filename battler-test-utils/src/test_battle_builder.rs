use std::str::FromStr;

use ahash::{
    HashMapExt,
    HashSetExt,
};
use battler::{
    battle::{
        BagData,
        BattleBuilder,
        BattleBuilderOptions,
        BattleBuilderPlayerData,
        BattleBuilderSideData,
        BattleType,
        CoreBattleEngineOptions,
        CoreBattleEngineRandomizeBaseDamage,
        CoreBattleEngineSpeedSortTieResolution,
        FieldData,
        FieldEnvironment,
        PlayerOptions,
        PlayerType,
        PublicCoreBattle,
        WildPlayerOptions,
    },
    battler_error,
    common::{
        Error,
        FastHashMap,
    },
    config::{
        FormatData,
        FormatOptions,
        Rule,
        SerializedRuleSet,
    },
    dex::DataStore,
    teams::TeamData,
};

use crate::ControlledRandomNumberGenerator;

/// [`BattleBuilder`] object for integration tests.
pub struct TestBattleBuilder {
    options: BattleBuilderOptions,
    engine_options: CoreBattleEngineOptions,
    teams: FastHashMap<String, TeamData>,
    bags: FastHashMap<String, BagData>,
    validate_team: bool,
    controlled_rng: bool,
    infinite_bags: bool,
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
                    options: FormatOptions::default(),
                },
                field: FieldData::default(),
                side_1: BattleBuilderSideData {
                    name: "Side 1".to_string(),
                    players: Vec::new(),
                },
                side_2: BattleBuilderSideData {
                    name: "Side 2".to_string(),
                    players: Vec::new(),
                },
            },
            engine_options: CoreBattleEngineOptions::default(),
            teams: FastHashMap::new(),
            bags: FastHashMap::new(),
            validate_team: true,
            controlled_rng: false,
            infinite_bags: false,
        }
    }

    /// Builds a new [`CoreBattle`] from the battle builder.
    pub fn build(mut self, data: &dyn DataStore) -> Result<PublicCoreBattle, Error> {
        if self.controlled_rng {
            self.engine_options.rng_factory =
                |seed: Option<u64>| Box::new(ControlledRandomNumberGenerator::new(seed));
        }

        let infinite_bags = self.infinite_bags;
        for player in self.players_mut() {
            player.player_options.has_strict_bag = !infinite_bags;
        }

        let mut builder = BattleBuilder::new(self.options, data)?;

        for (player_id, mut team) in self.teams {
            if self.validate_team {
                let validation = builder.validate_team(&mut team);
                if let Err(error) = validation {
                    return Err(battler_error!(
                        "team for player {player_id} is invalid: {error}"
                    ));
                }
            }
            builder.update_team(&player_id, team)?;
        }

        for (player_id, mut bag) in self.bags {
            if self.validate_team {
                let validation = builder.validate_bag(&mut bag);
                if let Err(error) = validation {
                    return Err(battler_error!(
                        "bag for player {player_id} is invalid: {error}"
                    ));
                }
            }
            builder.update_bag(&player_id, bag)?;
        }

        builder.build(self.engine_options)
    }

    fn players_mut(&mut self) -> impl Iterator<Item = &mut BattleBuilderPlayerData> {
        self.options
            .side_1
            .players
            .iter_mut()
            .chain(self.options.side_2.players.iter_mut())
    }

    pub fn with_auto_continue(mut self, auto_continue: bool) -> Self {
        self.engine_options.auto_continue = auto_continue;
        self
    }

    pub fn with_actual_health(mut self, actual_health: bool) -> Self {
        self.engine_options.reveal_actual_health = actual_health;
        self
    }

    pub fn with_pass_allowed(mut self, pass_allowed: bool) -> Self {
        self.engine_options.allow_pass_for_unfainted_mon = pass_allowed;
        self
    }

    pub fn with_controlled_rng(mut self, controlled_rng: bool) -> Self {
        self.controlled_rng = controlled_rng;
        self
    }

    pub fn with_base_damage_randomization(
        mut self,
        randomize: CoreBattleEngineRandomizeBaseDamage,
    ) -> Self {
        self.engine_options.randomize_base_damage = randomize;
        self
    }

    pub fn with_speed_sort_tie_resolution(
        mut self,
        tie_resolution: CoreBattleEngineSpeedSortTieResolution,
    ) -> Self {
        self.engine_options.speed_sort_tie_resolution = tie_resolution;
        self
    }

    pub fn with_volatile_status_logs(mut self, log_volatile_statuses: bool) -> Self {
        self.engine_options.log_volatile_statuses = log_volatile_statuses;
        self
    }

    pub fn with_team_validation(mut self, team_validation: bool) -> Self {
        self.validate_team = team_validation;
        self
    }

    pub fn with_battle_type(mut self, battle_type: BattleType) -> Self {
        self.options.format.battle_type = battle_type;
        self
    }

    pub fn with_rule(mut self, rule: &str) -> Self {
        let rule = Rule::from_str(rule).unwrap();
        self.options.format.rules.insert(rule);
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
            player_type: PlayerType::Trainer,
            player_options: PlayerOptions::default(),
        });
        self
    }

    pub fn add_protagonist_to_side_1(mut self, id: &str, name: &str) -> Self {
        self.options.side_1.players.push(BattleBuilderPlayerData {
            id: id.to_owned(),
            name: name.to_owned(),
            player_type: PlayerType::Protagonist,
            player_options: PlayerOptions {
                has_affection: true,
                has_strict_bag: true,
            },
        });
        self
    }

    pub fn add_player_to_side_2(mut self, id: &str, name: &str) -> Self {
        self.options.side_2.players.push(BattleBuilderPlayerData {
            id: id.to_owned(),
            name: name.to_owned(),
            player_type: PlayerType::Trainer,
            player_options: PlayerOptions::default(),
        });
        self
    }

    pub fn add_wild_mon_to_side_2(
        mut self,
        id: &str,
        name: &str,
        options: WildPlayerOptions,
    ) -> Self {
        self.options.side_2.players.push(BattleBuilderPlayerData {
            id: id.to_owned(),
            name: name.to_owned(),
            player_type: PlayerType::Wild(options),
            player_options: PlayerOptions::default(),
        });
        self
    }

    pub fn with_team(mut self, player_id: &str, team: TeamData) -> Self {
        self.teams.insert(player_id.to_owned(), team);
        self
    }

    pub fn with_bag(mut self, player_id: &str, bag: BagData) -> Self {
        self.bags.insert(player_id.to_owned(), bag);
        self
    }

    pub fn with_adjacenecy_reach(mut self, adjacenecy_reach: u8) -> Self {
        self.options.format.options.adjacency_reach = adjacenecy_reach;
        self
    }

    pub fn with_obedience_cap(mut self, obedience_cap: u8) -> Self {
        self.options.format.options.obedience_cap = obedience_cap;
        self
    }

    pub fn with_bag_items(mut self, bag_items: bool) -> Self {
        self.options.format.options.bag_items = bag_items;
        self
    }

    pub fn with_infinite_bags(mut self, infinite_bags: bool) -> Self {
        self.infinite_bags = infinite_bags;
        self
    }

    pub fn with_weather(mut self, weather: Option<String>) -> Self {
        self.options.field.weather = weather;
        self
    }

    pub fn with_terrain(mut self, terrain: Option<String>) -> Self {
        self.options.field.terrain = terrain;
        self
    }

    pub fn with_field_environment(mut self, environment: FieldEnvironment) -> Self {
        self.options.field.environment = environment;
        self
    }
}
