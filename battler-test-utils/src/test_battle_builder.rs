use std::str::FromStr;

use ahash::HashMap;
use anyhow::Result;
use battler::{
    BattleType,
    CoreBattleEngineOptions,
    CoreBattleEngineRandomizeBaseDamage,
    CoreBattleEngineSpeedSortTieResolution,
    CoreBattleOptions,
    DataStore,
    FieldData,
    FieldEnvironment,
    FormatData,
    Id,
    PlayerData,
    PlayerDex,
    PlayerOptions,
    PlayerType,
    PublicCoreBattle,
    Rule,
    SerializedRuleSet,
    SideData,
    TeamData,
    TimeOfDay,
    WildPlayerOptions,
};

use crate::ControlledRandomNumberGenerator;

/// Battle builder object for integration tests.
pub struct TestBattleBuilder {
    options: CoreBattleOptions,
    engine_options: CoreBattleEngineOptions,
    teams: HashMap<String, TeamData>,
    controlled_rng: bool,
    infinite_bags: bool,
}

impl TestBattleBuilder {
    /// Creates a new [`TestBattleBuilder`].
    pub fn new() -> Self {
        Self {
            options: CoreBattleOptions {
                seed: None,
                format: FormatData {
                    battle_type: BattleType::Singles,
                    rules: SerializedRuleSet::new(),
                },
                field: FieldData::default(),
                side_1: SideData {
                    name: "Side 1".to_string(),
                    players: Vec::new(),
                },
                side_2: SideData {
                    name: "Side 2".to_string(),
                    players: Vec::new(),
                },
            },
            engine_options: CoreBattleEngineOptions {
                log_time: false,
                ..Default::default()
            },
            teams: HashMap::default(),
            controlled_rng: false,
            infinite_bags: false,
        }
    }

    fn modify_options_for_build(&mut self) {
        if self.controlled_rng {
            self.engine_options.rng_factory =
                |seed: Option<u64>| Box::new(ControlledRandomNumberGenerator::new(seed));
        }

        let infinite_bags = self.infinite_bags;
        for player in self.players_mut() {
            player.player_options.has_strict_bag = !infinite_bags;
        }
    }

    /// Builds a new [`PublicCoreBattle`] from the battle builder.
    pub fn build<'d>(mut self, data: &'d dyn DataStore) -> Result<PublicCoreBattle<'d>> {
        self.modify_options_for_build();
        let mut battle = PublicCoreBattle::new(self.options, data, self.engine_options)?;
        for (player_id, team) in self.teams {
            battle.update_team(&player_id, team)?;
        }
        Ok(battle)
    }

    fn players_mut(&mut self) -> impl Iterator<Item = &mut PlayerData> {
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

    pub fn with_catch_rate_logs(mut self, log_catch_rate: bool) -> Self {
        self.engine_options.log_catch_rate = log_catch_rate;
        self
    }

    pub fn with_team_validation(mut self, team_validation: bool) -> Self {
        self.engine_options.validate_teams = team_validation;
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
        self.options.side_1.players.push(PlayerData {
            id: id.to_owned(),
            name: name.to_owned(),
            player_type: PlayerType::Trainer,
            player_options: PlayerOptions::default(),
            team: TeamData::default(),
            dex: PlayerDex::default(),
        });
        self
    }

    pub fn add_protagonist_to_side_1(mut self, id: &str, name: &str) -> Self {
        self.options.side_1.players.push(PlayerData {
            id: id.to_owned(),
            name: name.to_owned(),
            player_type: PlayerType::Protagonist,
            player_options: PlayerOptions {
                has_affection: true,
                has_strict_bag: true,
                mons_caught: 151,
                cannot_mega_evolve: false,
                cannot_dynamax: false,
                cannot_terastallize: false,
            },
            team: TeamData::default(),
            dex: PlayerDex::default(),
        });
        self
    }

    pub fn add_player_to_side_2(mut self, id: &str, name: &str) -> Self {
        self.options.side_2.players.push(PlayerData {
            id: id.to_owned(),
            name: name.to_owned(),
            player_type: PlayerType::Trainer,
            player_options: PlayerOptions::default(),
            team: TeamData::default(),
            dex: PlayerDex::default(),
        });
        self
    }

    pub fn add_wild_mon_to_side_2(
        mut self,
        id: &str,
        name: &str,
        options: WildPlayerOptions,
    ) -> Self {
        self.options.side_2.players.push(PlayerData {
            id: id.to_owned(),
            name: name.to_owned(),
            player_type: PlayerType::Wild(options),
            player_options: PlayerOptions::default(),
            team: TeamData::default(),
            dex: PlayerDex::default(),
        });
        self
    }

    pub fn with_team(mut self, player_id: &str, team: TeamData) -> Self {
        self.teams.insert(player_id.to_owned(), team);
        self
    }

    pub fn with_player_dex(mut self, player_id: &str, dex: PlayerDex) -> Self {
        if let Some(player) = self.players_mut().find(|player| player.id == player_id) {
            player.dex = dex;
        }
        self
    }

    pub fn with_adjacency_reach(mut self, adjacency_reach: u8) -> Self {
        self.options.format.rules.insert(Rule::Value {
            name: Id::from("Adjacency Reach"),
            value: adjacency_reach.to_string(),
        });
        self
    }

    pub fn with_obedience_cap(mut self, obedience_cap: u8) -> Self {
        self.options.format.rules.insert(Rule::Value {
            name: Id::from("Obedience Cap"),
            value: obedience_cap.to_string(),
        });
        self
    }

    fn with_boolean_rule(mut self, name: &str, value: bool) -> Self {
        let rule = Rule::Value {
            name: Id::from(name),
            value: String::default(),
        };
        if value {
            self.options.format.rules.insert(rule);
        } else {
            self.options.format.rules.remove(&rule);
        }
        self
    }

    pub fn with_bag_items(self, bag_items: bool) -> Self {
        self.with_boolean_rule("Bag Items", bag_items)
    }

    pub fn with_infinite_bags(mut self, infinite_bags: bool) -> Self {
        self.infinite_bags = infinite_bags;
        self
    }

    pub fn with_mega_evolution(self, mega_evolution: bool) -> Self {
        self.with_boolean_rule("Mega Evolution", mega_evolution)
    }

    pub fn with_primal_reversion(self, primal_reversion: bool) -> Self {
        self.with_boolean_rule("Primal Reversion", primal_reversion)
    }

    pub fn with_dynamax(self, dynamax: bool) -> Self {
        self.with_boolean_rule("Dynamax", dynamax)
    }

    pub fn with_terastallization(self, terastallization: bool) -> Self {
        self.with_boolean_rule("Terastallization", terastallization)
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

    pub fn with_time_of_day(mut self, time: TimeOfDay) -> Self {
        self.options.field.time = time;
        self
    }
}
