use std::sync::Arc;

use ahash::HashMap;
use anyhow::{
    Error,
    Result,
};
use battler::{
    CoreBattleEngineOptions,
    CoreBattleOptions,
    DataStore,
    PlayerBattleData,
    PublicCoreBattle,
    Request,
    SideData,
    TeamData,
    ValidationError,
};
use futures_util::lock::Mutex;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::{
    Battle,
    BattleState,
    Player,
    PlayerState,
    PlayerValidation,
    Side,
    log::{
        Log,
        SplitLogs,
    },
};

struct LiveBattle<'d> {
    uuid: Uuid,
    battle: PublicCoreBattle<'d>,
    sides: Vec<Side>,
    logs: SplitLogs,
    error: Option<String>,
}

impl<'d> LiveBattle<'d> {
    fn new(
        options: CoreBattleOptions,
        engine_options: CoreBattleEngineOptions,
        data: &'d dyn DataStore,
    ) -> Result<Self> {
        let uuid = Uuid::new_v4();
        let sides = Vec::from_iter([
            Self::new_side(&options.side_1),
            Self::new_side(&options.side_2),
        ]);
        let logs = SplitLogs::new(sides.len());
        let battle = PublicCoreBattle::new(options, data, engine_options)?;

        LiveBattle {
            uuid,
            battle,
            sides,
            logs,
            error: None,
        }
        .initialize()
    }

    fn new_side(side: &SideData) -> Side {
        Side {
            name: side.name.clone(),
            players: side
                .players
                .iter()
                .map(|player| Player {
                    id: player.id.clone(),
                    name: player.name.clone(),
                    state: PlayerState::Waiting,
                })
                .collect(),
        }
    }

    fn initialize(mut self) -> Result<Self> {
        let players = self.players().map(|s| s.to_owned()).collect::<Vec<_>>();
        for player in players {
            self.update_player_state(&player)?;
        }
        Ok(self)
    }

    fn players(&self) -> impl Iterator<Item = &str> {
        self.sides
            .iter()
            .flat_map(|side| side.players.iter().map(|player| player.id.as_str()))
    }

    fn player_mut(&mut self, id: &str) -> Option<&mut Player> {
        self.sides
            .iter_mut()
            .find_map(|side| side.players.iter_mut().find(|player| player.id == id))
    }

    fn player_mut_or_error(&mut self, id: &str) -> Result<&mut Player> {
        self.player_mut(id)
            .ok_or_else(|| Error::msg("player does not exist"))
    }

    fn battle_state(&self) -> BattleState {
        if !self.battle.started() {
            BattleState::Preparing
        } else if !self.battle.ended() {
            BattleState::Active
        } else {
            BattleState::Finished
        }
    }

    fn battle_status(&self) -> Battle {
        Battle {
            uuid: self.uuid,
            state: self.battle_state(),
            sides: self.sides.clone(),
            error: self.error.clone(),
        }
    }

    fn log_for_side(&self, side: Option<usize>) -> &Log {
        side.and_then(|side| self.logs.side_log(side))
            .unwrap_or(self.logs.public_log())
    }

    fn update_player_state(&mut self, player: &str) -> Result<()> {
        let state = if self.battle.validate_player(&player).is_ok() {
            PlayerState::Ready
        } else {
            PlayerState::Waiting
        };
        self.player_mut_or_error(player)?.state = state;
        Ok(())
    }

    fn proceed(&mut self) {
        self.error = None;
        if let Err(err) = self.proceed_internal() {
            self.error = Some(format!("{err:#}"));
        }
    }

    fn proceed_internal(&mut self) -> Result<()> {
        if self.battle.ready_to_continue()? {
            self.battle.continue_battle()?;
        }
        self.logs.append(self.battle.new_log_entries());
        Ok(())
    }
}

/// Service for managing multiple battles on the [`battler`] battle engine.
pub struct BattlerService<'d> {
    data: &'d dyn DataStore,
    battles: Arc<Mutex<HashMap<Uuid, Arc<Mutex<LiveBattle<'d>>>>>>,
}

impl<'d> BattlerService<'d> {
    /// Creates a new battle service.
    pub fn new(data: &'d dyn DataStore) -> Self {
        Self {
            data,
            battles: Arc::new(Mutex::new(HashMap::default())),
        }
    }

    async fn find_battle(&self, uuid: Uuid) -> Option<Arc<Mutex<LiveBattle<'d>>>> {
        self.battles.lock().await.get(&uuid).cloned()
    }

    async fn find_battle_or_error(&self, uuid: Uuid) -> Result<Arc<Mutex<LiveBattle<'d>>>> {
        self.find_battle(uuid)
            .await
            .ok_or_else(|| Error::msg("battle does not exist"))
    }

    /// Generates the status of an existing battle.
    pub async fn battle(&self, battle: Uuid) -> Result<Battle> {
        let battle = self.find_battle_or_error(battle).await?;
        let battle = battle.lock().await;
        Ok(battle.battle_status())
    }

    /// Creates a new battle.
    pub async fn create_battle(
        &self,
        options: CoreBattleOptions,
        mut engine_options: CoreBattleEngineOptions,
    ) -> Result<Battle> {
        // Do not auto continue, so that we can capture any errors in our own task.
        engine_options.auto_continue = false;

        let battle = LiveBattle::new(options, engine_options, self.data)?;
        let uuid = battle.uuid;
        self.battles
            .lock()
            .await
            .insert(uuid, Arc::new(Mutex::new(battle)));

        self.battle(uuid).await
    }

    /// Updates a player's team for a battle.
    pub async fn update_team(&self, battle: Uuid, player: &str, team: TeamData) -> Result<()> {
        let battle = self.find_battle_or_error(battle).await?;
        let mut battle = battle.lock().await;
        battle.battle.update_team(player, team)?;
        battle.update_player_state(player)?;
        Ok(())
    }

    /// Validates a player in a battle.
    pub async fn validate_player(&self, battle: Uuid, player: &str) -> Result<PlayerValidation> {
        let battle = self.find_battle_or_error(battle).await?;
        let mut battle = battle.lock().await;
        match battle.battle.validate_player(player) {
            Ok(()) => Ok(PlayerValidation::default()),
            Err(err) => match err.downcast::<ValidationError>() {
                Ok(err) => Ok(PlayerValidation {
                    problems: err.problems().map(|s| s.to_owned()).collect(),
                }),
                Err(err) => Err(err),
            },
        }
    }

    /// Starts a battle.
    pub async fn start(&self, battle: Uuid) -> Result<()> {
        let battle = self.find_battle_or_error(battle).await?;
        let mut battle = battle.lock().await;
        battle.battle.start()?;
        battle.proceed();
        Ok(())
    }

    /// Returns the player data for a player in a battle.
    pub async fn player_data(&self, battle: Uuid, player: &str) -> Result<PlayerBattleData> {
        let battle = self.find_battle_or_error(battle).await?;
        let mut battle = battle.lock().await;
        battle.battle.player_data(player).map_err(|err| err.into())
    }

    /// Returns the current request for a player in a battle.
    pub async fn request(&self, battle: Uuid, player: &str) -> Result<Option<Request>> {
        let battle = self.find_battle_or_error(battle).await?;
        let battle = battle.lock().await;
        battle.battle.request_for_player(player)
    }

    /// Sets a player's choice in a battle.
    pub async fn make_choice(&self, battle: Uuid, player: &str, choice: &str) -> Result<()> {
        let battle = self.find_battle_or_error(battle).await?;
        let mut battle = battle.lock().await;
        battle.battle.set_player_choice(player, choice)?;

        battle.proceed();

        Ok(())
    }

    /// Reads the full battle log for the side.
    ///
    /// If `side` is `None`, the public log is used.
    pub async fn full_log(&self, battle: Uuid, side: Option<usize>) -> Result<Vec<String>> {
        let battle = self.find_battle_or_error(battle).await?;
        let battle = battle.lock().await;
        Ok(battle
            .log_for_side(side)
            .entries()
            .map(|s| s.to_owned())
            .collect())
    }

    /// Subscribes to battle log updates for the player.
    ///
    /// If `side` is `None`, the public log is used.
    pub async fn subscribe(
        &self,
        battle: Uuid,
        side: Option<usize>,
    ) -> Result<broadcast::Receiver<String>> {
        let battle = self.find_battle_or_error(battle).await?;
        let battle = battle.lock().await;
        Ok(battle.log_for_side(side).subscribe())
    }
}

#[cfg(test)]
mod battler_service_test {
    use battler::{
        BagData,
        BattleType,
        CoreBattleEngineOptions,
        CoreBattleEngineSpeedSortTieResolution,
        CoreBattleOptions,
        FastHashSet,
        FieldData,
        FormatData,
        FormatOptions,
        Gender,
        LocalDataStore,
        MonData,
        MonPersistentBattleData,
        Nature,
        PlayerData,
        PlayerOptions,
        PlayerType,
        RequestType,
        Rule,
        SideData,
        StatTable,
        TeamData,
        ValidationError,
    };
    use itertools::Itertools;
    use tokio::sync::broadcast;

    use super::BattlerService;
    use crate::{
        BattleState,
        Player,
        PlayerState,
        Side,
    };

    fn mon(
        name: String,
        species: String,
        ability: String,
        moves: Vec<String>,
        level: u8,
    ) -> MonData {
        MonData {
            name,
            species,
            item: None,
            ability,
            moves,
            pp_boosts: Vec::default(),
            nature: Nature::Hardy,
            true_nature: None,
            gender: Gender::Female,
            evs: StatTable::default(),
            ivs: StatTable::default(),
            level,
            experience: 0,
            shiny: false,
            friendship: 255,
            ball: "Poké Ball".to_owned(),
            hidden_power_type: None,
            different_original_trainer: false,
            persistent_battle_data: MonPersistentBattleData::default(),
        }
    }

    fn team(level: u8) -> TeamData {
        TeamData {
            members: Vec::from_iter([
                mon(
                    "Bulbasaur".to_owned(),
                    "Bulbasaur".to_owned(),
                    "Overgrow".to_owned(),
                    Vec::from_iter(["Tackle".to_owned()]),
                    level,
                ),
                mon(
                    "Charmander".to_owned(),
                    "Charmander".to_owned(),
                    "Blaze".to_owned(),
                    Vec::from_iter(["Scratch".to_owned()]),
                    level,
                ),
                mon(
                    "Squirtle".to_owned(),
                    "Squirtle".to_owned(),
                    "Torrent".to_owned(),
                    Vec::from_iter(["Tackle".to_owned()]),
                    level,
                ),
            ]),
            bag: BagData::default(),
        }
    }

    fn core_battle_options(team: TeamData) -> CoreBattleOptions {
        CoreBattleOptions {
            seed: Some(0),
            format: FormatData {
                battle_type: BattleType::Singles,
                rules: FastHashSet::from_iter([Rule::value_name("Item Clause")]),
                options: FormatOptions::default(),
            },
            field: FieldData::default(),
            side_1: SideData {
                name: "Side 1".to_owned(),
                players: Vec::from_iter([PlayerData {
                    id: "player-1".to_owned(),
                    name: "Player 1".to_owned(),
                    player_type: PlayerType::Trainer,
                    player_options: PlayerOptions::default(),
                    team: team.clone(),
                }]),
            },
            side_2: SideData {
                name: "Side 2".to_owned(),
                players: Vec::from_iter([PlayerData {
                    id: "player-2".to_owned(),
                    name: "Player 2".to_owned(),
                    player_type: PlayerType::Trainer,
                    player_options: PlayerOptions::default(),
                    team: team.clone(),
                }]),
            },
        }
    }

    async fn read_all_entries_from_log_rx(log_rx: &mut broadcast::Receiver<String>) -> Vec<String> {
        let mut entries = Vec::new();
        while let Ok(entry) = log_rx.try_recv() {
            entries.push(entry);
        }
        entries
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn creates_battle_and_players_in_waiting_state() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let battler_service = BattlerService::new(&data);
        let battle = battler_service
            .create_battle(
                core_battle_options(TeamData::default()),
                CoreBattleEngineOptions::default(),
            )
            .await
            .unwrap();
        assert_eq!(battle.state, BattleState::Preparing);
        pretty_assertions::assert_eq!(
            battle.sides,
            Vec::from_iter([
                Side {
                    name: "Side 1".to_owned(),
                    players: Vec::from_iter([Player {
                        id: "player-1".to_owned(),
                        name: "Player 1".to_owned(),
                        state: PlayerState::Waiting,
                    }])
                },
                Side {
                    name: "Side 2".to_owned(),
                    players: Vec::from_iter([Player {
                        id: "player-2".to_owned(),
                        name: "Player 2".to_owned(),
                        state: PlayerState::Waiting,
                    }])
                }
            ])
        );

        assert_matches::assert_matches!(battler_service.battle(battle.uuid).await, Ok(read_battle) => {
            pretty_assertions::assert_eq!(read_battle, battle);
        });
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cannot_start_battle_with_empty_teams() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let battler_service = BattlerService::new(&data);
        let battle = battler_service
            .create_battle(
                core_battle_options(TeamData::default()),
                CoreBattleEngineOptions::default(),
            )
            .await
            .unwrap();
        assert_matches::assert_matches!(battler_service.start(battle.uuid).await, Err(err) => {
            assert_matches::assert_matches!(err.downcast::<ValidationError>(), Ok(err) => {
                assert!(err.problems().contains(&"Validation failed for Player 1: Empty team is not allowed."), "{err:?}");
                assert!(err.problems().contains(&"Validation failed for Player 2: Empty team is not allowed."), "{err:?}");
            });
        });
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn player_moves_to_ready_state_with_valid_team() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let battler_service = BattlerService::new(&data);
        let battle = battler_service
            .create_battle(
                core_battle_options(TeamData::default()),
                CoreBattleEngineOptions::default(),
            )
            .await
            .unwrap();
        assert_matches::assert_matches!(
            battler_service
                .update_team(battle.uuid, "player-1", team(5))
                .await,
            Ok(())
        );

        assert_matches::assert_matches!(battler_service.battle(battle.uuid).await, Ok(battle) => {
            assert_eq!(battle.sides[0].players[0].state, PlayerState::Ready);
        });
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn invalid_team_fails_validation_and_resets_state() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let battler_service = BattlerService::new(&data);
        let battle = battler_service
            .create_battle(
                core_battle_options(team(5)),
                CoreBattleEngineOptions::default(),
            )
            .await
            .unwrap();
        assert_eq!(battle.sides[0].players[0].state, PlayerState::Ready);
        assert_matches::assert_matches!(battler_service.validate_player(battle.uuid, "player-1").await, Ok(validation) => {
            assert!(validation.problems.is_empty());
        });

        let mut bad_team = team(5);
        bad_team.members[0].item = Some("Leftovers".to_owned());
        bad_team.members[1].item = Some("Leftovers".to_owned());

        assert_matches::assert_matches!(
            battler_service
                .update_team(battle.uuid, "player-1", bad_team)
                .await,
            Ok(())
        );

        assert_matches::assert_matches!(battler_service.battle(battle.uuid).await, Ok(battle) => {
            assert_eq!(battle.sides[0].players[0].state, PlayerState::Waiting);
        });

        assert_matches::assert_matches!(battler_service.validate_player(battle.uuid, "player-1").await, Ok(validation) => {
            pretty_assertions::assert_eq!(validation.problems, Vec::from_iter(["Item Leftovers appears more than 1 time."]));
        });
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn starts_battle_and_reports_player_and_request_data() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let battler_service = BattlerService::new(&data);
        let battle = battler_service
            .create_battle(
                core_battle_options(team(5)),
                CoreBattleEngineOptions::default(),
            )
            .await
            .unwrap();

        assert_matches::assert_matches!(battler_service.start(battle.uuid).await, Ok(()));

        assert_matches::assert_matches!(
            battler_service.player_data(battle.uuid, "player-1").await,
            Ok(data) => {
                assert_eq!(data.mons.len(), 3);
            }
        );
        assert_matches::assert_matches!(
            battler_service.player_data(battle.uuid, "player-2").await,
            Ok(data) => {
                assert_eq!(data.mons.len(), 3);
            }
        );
        assert_matches::assert_matches!(
            battler_service.player_data(battle.uuid, "player-3").await,
            Err(_)
        );

        assert_matches::assert_matches!(
            battler_service.request(battle.uuid, "player-1").await,
            Ok(Some(request)) => {
                assert_eq!(request.request_type(), RequestType::Turn);
            }
        );
        assert_matches::assert_matches!(
            battler_service.request(battle.uuid, "player-2").await,
            Ok(Some(request)) => {
                assert_eq!(request.request_type(), RequestType::Turn);
            }
        );
        assert_matches::assert_matches!(
            battler_service.request(battle.uuid, "player-3").await,
            Err(_)
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn returns_filtered_logs_by_side() {
        let data = LocalDataStore::new_from_env("DATA_DIR").unwrap();
        let battler_service = BattlerService::new(&data);
        let battle = battler_service
            .create_battle(
                core_battle_options(team(5)),
                CoreBattleEngineOptions {
                    speed_sort_tie_resolution: CoreBattleEngineSpeedSortTieResolution::Keep,
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_matches::assert_matches!(battler_service.start(battle.uuid).await, Ok(()));

        let mut side_1_log_rx = battler_service
            .subscribe(battle.uuid, Some(0))
            .await
            .unwrap();
        let mut side_2_log_rx = battler_service
            .subscribe(battle.uuid, Some(1))
            .await
            .unwrap();
        let mut public_log_rx = battler_service.subscribe(battle.uuid, None).await.unwrap();

        assert_matches::assert_matches!(
            battler_service
                .make_choice(battle.uuid, "player-1", "move 0")
                .await,
            Ok(())
        );
        assert_matches::assert_matches!(
            battler_service
                .make_choice(battle.uuid, "player-2", "move 0")
                .await,
            Ok(())
        );

        pretty_assertions::assert_eq!(
            read_all_entries_from_log_rx(&mut side_1_log_rx).await[1..],
            [
                "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Bulbasaur,player-2,1",
                "damage|mon:Bulbasaur,player-2,1|health:79/100",
                "move|mon:Bulbasaur,player-2,1|name:Tackle|target:Bulbasaur,player-1,1",
                "damage|mon:Bulbasaur,player-1,1|health:15/19",
                "residual",
                "turn|turn:2"
            ],
        );
        pretty_assertions::assert_eq!(
            read_all_entries_from_log_rx(&mut side_2_log_rx).await[1..],
            [
                "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Bulbasaur,player-2,1",
                "damage|mon:Bulbasaur,player-2,1|health:15/19",
                "move|mon:Bulbasaur,player-2,1|name:Tackle|target:Bulbasaur,player-1,1",
                "damage|mon:Bulbasaur,player-1,1|health:79/100",
                "residual",
                "turn|turn:2"
            ],
        );
        pretty_assertions::assert_eq!(
            read_all_entries_from_log_rx(&mut public_log_rx).await[1..],
            [
                "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Bulbasaur,player-2,1",
                "damage|mon:Bulbasaur,player-2,1|health:79/100",
                "move|mon:Bulbasaur,player-2,1|name:Tackle|target:Bulbasaur,player-1,1",
                "damage|mon:Bulbasaur,player-1,1|health:79/100",
                "residual",
                "turn|turn:2"
            ],
        );
    }
}
