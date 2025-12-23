use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    sync::{
        Arc,
        Weak,
    },
    time::{
        Duration,
        SystemTime,
    },
};

use ahash::HashMap;
use anyhow::{
    Error,
    Result,
};
use battler::SideData;
use battler_service_client::BattlerServiceClient;
use futures_util::lock::Mutex;
use tokio::{
    sync::broadcast,
    task::JoinSet,
};
use uuid::Uuid;

use crate::{
    Player,
    PlayerStatus,
    ProposedBattle,
    ProposedBattleOptions,
    ProposedBattleRejection,
    ProposedBattleResponse,
    ProposedBattleUpdate,
    Side,
};

struct ActiveProposedBattle {
    options: ProposedBattleOptions,
    proposed_battle: ProposedBattle,
}

impl ActiveProposedBattle {
    fn new(uuid: Uuid, options: ProposedBattleOptions) -> Self {
        let timeout = options.timeout.min(Duration::from_mins(5));
        let proposed_battle = ProposedBattle {
            uuid,
            sides: Vec::from_iter([
                Self::new_side(&options.battle_options.side_1),
                Self::new_side(&options.battle_options.side_2),
            ]),
            deadline: SystemTime::now() + timeout,
        };
        Self {
            options,
            proposed_battle,
        }
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
                    status: None,
                })
                .collect(),
        }
    }

    fn uuid(&self) -> Uuid {
        self.proposed_battle.uuid
    }

    fn deadline(&self) -> SystemTime {
        self.proposed_battle.deadline
    }

    fn proposed_battle(&self) -> ProposedBattle {
        self.proposed_battle.clone()
    }

    fn proposed_battle_rejection(&self) -> Option<ProposedBattleRejection> {
        let rejected = self
            .proposed_battle
            .sides
            .iter()
            .flat_map(|side| side.players.iter())
            .filter(|player| {
                player
                    .status
                    .as_ref()
                    .is_some_and(|status| *status == PlayerStatus::Rejected)
            })
            .map(|player| player.id.clone())
            .collect::<Vec<_>>();
        if rejected.is_empty() {
            return None;
        }
        Some(ProposedBattleRejection { players: rejected })
    }

    fn players(&self) -> Vec<String> {
        self.proposed_battle
            .sides
            .iter()
            .flat_map(|side| side.players.iter())
            .map(|player| player.id.clone())
            .collect()
    }

    fn player_mut(&mut self, id: &str) -> Result<&mut Player> {
        self.proposed_battle
            .sides
            .iter_mut()
            .flat_map(|side| side.players.iter_mut())
            .find(|player| player.id == id)
            .ok_or_else(|| Error::msg("player not found"))
    }

    fn respond(&mut self, player: &str, response: &ProposedBattleResponse) -> Result<()> {
        let player = self.player_mut(player)?;
        let new_status = if response.accept {
            Some(PlayerStatus::Accepted)
        } else {
            Some(PlayerStatus::Rejected)
        };
        if new_status == player.status {
            return Err(Error::msg("already responded"));
        }
        player.status = new_status;
        Ok(())
    }

    fn ready_to_create(&self) -> bool {
        self.proposed_battle.sides.iter().all(|side| {
            side.players.iter().all(|player| {
                player
                    .status
                    .as_ref()
                    .is_some_and(|status| *status == PlayerStatus::Accepted)
            })
        })
    }

    fn rejected(&self) -> bool {
        self.proposed_battle.sides.iter().any(|side| {
            side.players.iter().any(|player| {
                player
                    .status
                    .as_ref()
                    .is_some_and(|status| *status == PlayerStatus::Rejected)
            })
        })
    }
}

#[derive(Debug, Clone)]
struct UnderlyingBattle {
    uuid: Uuid,
    started: bool,
}

struct ActiveProposedBattleManagerState {
    proposed_battle: ActiveProposedBattle,
    battle: Option<UnderlyingBattle>,
    error: Option<String>,
    watcher_task_started: bool,
    join_set: JoinSet<()>,
}

struct ActiveProposedBattleManager {
    state: Mutex<ActiveProposedBattleManagerState>,

    battler_service_client: Arc<Box<dyn BattlerServiceClient>>,
    battler_multiplayer_service_state: Arc<Mutex<BattlerMultiplayerServiceState>>,
}

impl ActiveProposedBattleManager {
    fn new(
        proposed_battle: ActiveProposedBattle,
        battler_service_client: Arc<Box<dyn BattlerServiceClient>>,
        battler_multiplayer_service_state: Arc<Mutex<BattlerMultiplayerServiceState>>,
    ) -> Self {
        Self {
            state: Mutex::new(ActiveProposedBattleManagerState {
                proposed_battle,
                battle: None,
                error: None,
                watcher_task_started: false,
                join_set: JoinSet::default(),
            }),
            battler_service_client,
            battler_multiplayer_service_state,
        }
    }

    async fn uuid(&self) -> Uuid {
        self.state.lock().await.proposed_battle.uuid()
    }

    async fn proposed_battle(&self) -> ProposedBattle {
        self.state.lock().await.proposed_battle.proposed_battle()
    }

    async fn players(&self) -> Vec<String> {
        self.state.lock().await.proposed_battle.players()
    }

    async fn proposed_battle_rejection(&self) -> Option<ProposedBattleRejection> {
        self.state
            .lock()
            .await
            .proposed_battle
            .proposed_battle_rejection()
    }

    async fn deletion_reason(&self) -> Option<String> {
        let (deadline, rejected, started, error) = {
            let state = self.state.lock().await;
            (
                state.proposed_battle.deadline(),
                state.proposed_battle.rejected(),
                state.battle.as_ref().is_some_and(|battle| battle.started),
                state.error.clone(),
            )
        };
        if started {
            Some("fulfilled".to_owned())
        } else if rejected {
            Some("rejected".to_owned())
        } else if let Some(err) = error {
            Some(format!("internal error: {err}"))
        } else if SystemTime::now() >= deadline {
            Some("deadline exceeded".to_owned())
        } else {
            None
        }
    }

    async fn proposed_battle_update(&self) -> ProposedBattleUpdate {
        let battle = self
            .state
            .lock()
            .await
            .battle
            .as_ref()
            .map(|battle| battle.uuid.clone());
        ProposedBattleUpdate {
            proposed_battle: self.proposed_battle().await,
            battle,
            rejection: self.proposed_battle_rejection().await,
            deletion_reason: self.deletion_reason().await,
        }
    }

    async fn publish_update(&self) {
        let update = self.proposed_battle_update().await;
        for player in self.players().await {
            self.publish_update_to_player(&player, update.clone()).await;
        }
    }

    async fn publish_update_to_player(&self, player: &str, update: ProposedBattleUpdate) {
        self.battler_multiplayer_service_state
            .lock()
            .await
            .player_state(player)
            .lock()
            .await
            .update_tx
            .send(update)
            .ok();
    }

    async fn start(&self) {
        self.update().await;
    }

    async fn respond(&self, player: &str, response: &ProposedBattleResponse) -> Result<()> {
        if self
            .state
            .lock()
            .await
            .battle
            .as_ref()
            .is_some_and(|battle| battle.started)
        {
            return Err(Error::msg("battle started"));
        }
        self.state
            .lock()
            .await
            .proposed_battle
            .respond(player, response)?;
        self.update().await;
        Ok(())
    }

    async fn update(&self) {
        if let Err(err) = self.update_internal().await {
            self.state.lock().await.error = Some(err.to_string());
        }
        self.publish_update().await;
    }

    async fn update_internal(&self) -> Result<()> {
        let mut state = self.state.lock().await;
        let underlying_battle = state.battle.clone();

        if underlying_battle.is_none() && state.proposed_battle.ready_to_create() {
            let battle = self
                .battler_service_client
                .create(
                    state.proposed_battle.options.battle_options.clone(),
                    state.proposed_battle.options.service_options.clone(),
                )
                .await?;
            state.battle = Some(UnderlyingBattle {
                uuid: battle.uuid,
                started: false,
            })
        }

        if let Some(battle) = &underlying_battle
            && !battle.started
        {
            let battle = self.battler_service_client.battle(battle.uuid).await?;
            if battle
                .sides
                .iter()
                .flat_map(|side| side.players.iter())
                .all(|player| player.state == battler_service::PlayerState::Ready)
            {
                // Auto-start the battle.
                self.battler_service_client.start(battle.uuid).await?;
                state
                    .battle
                    .as_mut()
                    .ok_or_else(|| Error::msg("expected battle"))?
                    .started = true;

                // We rely on the owner to start watching the battle, in order to avoid a circular
                // dependency for the borrow and Send trait checkers (since the watching task can
                // call this method).
            }
        }
        Ok(())
    }

    async fn needs_to_watch_battle(&self) -> bool {
        let state = self.state.lock().await;
        state.battle.as_ref().is_some_and(|battle| !battle.started) && !state.watcher_task_started
    }

    async fn watch_battle(self: Arc<Self>) {
        // Ensure we only start watching the battle once.
        let mut state = self.state.lock().await;
        if state.watcher_task_started {
            return;
        }
        state.watcher_task_started = true;
        state.join_set.spawn(Self::watch_battle_until_started(
            Arc::downgrade(&self),
            self.battler_service_client.clone(),
        ));
    }

    async fn watch_battle_until_started(
        active_proposed_battle_manager: Weak<Self>,
        battler_service_client: Arc<Box<dyn BattlerServiceClient>>,
    ) {
        if let Err(err) = Self::watch_battle_until_started_internal(
            active_proposed_battle_manager.clone(),
            battler_service_client,
        )
        .await
        {
            if let Some(active_proposed_battle_manager) = active_proposed_battle_manager.upgrade() {
                active_proposed_battle_manager.state.lock().await.error = Some(err.to_string());
            }
        }
    }

    async fn watch_battle_until_started_internal(
        active_proposed_battle_manager: Weak<Self>,
        battler_service_client: Arc<Box<dyn BattlerServiceClient>>,
    ) -> Result<()> {
        let (battle, deadline) = {
            let active_proposed_battle_manager =
                active_proposed_battle_manager.upgrade().ok_or_else(|| {
                    Error::msg("active proposed battle already deleted before initializing watcher")
                })?;
            let state = active_proposed_battle_manager.state.lock().await;
            let battle = state
                .battle
                .as_ref()
                .ok_or_else(|| Error::msg("battle not available when initializing watcher"))?
                .uuid;
            let deadline = state.proposed_battle.deadline();
            (battle, deadline)
        };

        let process_log = async |entry: &str| {
            let entry = match entry.strip_prefix("-battlerservice:") {
                Some(entry) => entry,
                None => return false,
            };
            if entry == "started" {
                return true;
            }
            let active_proposed_battle_manager = match active_proposed_battle_manager.upgrade() {
                Some(active_proposed_battle_manager) => active_proposed_battle_manager,
                None => return true,
            };
            active_proposed_battle_manager.update().await;
            false
        };

        // Subscribe to the battle, for any new log.
        let mut battle_log_rx = battler_service_client.subscribe(battle, None).await?;

        // Read all old logs, and retroactively process them.
        //
        // This ensures we do not miss team updates and the battle starting
        let full_log = battler_service_client.full_log(battle, None).await?;
        for entry in full_log {
            if process_log(&entry).await {
                return Ok(());
            }
        }

        let now = SystemTime::now();
        let deadline = deadline.duration_since(now)?;

        // Watch the battle until it is reported as started.
        //
        // This loop ends when the corresponding ActiveProposedBattleManager is deleted (our Weak
        // upgrade fails). As a failsafe, we also stop watching the battle at the deadline.
        loop {
            tokio::select! {
                entry = battle_log_rx.recv() => {
                    if process_log(&entry?.content).await {
                        break;
                    }
                }
                _ = tokio::time::sleep(deadline) => {
                    break;
                }
            }
        }
        Ok(())
    }

    async fn delete(&self) {
        // Publish that we are deleting.
        //
        // NOTE: A previous update may have communicated a deletion reason, but this is not
        // guaranteed (e.g., a timeout), so we must publish an update here even if it ends up in a
        // duplicate.
        self.publish_update().await;

        let state = self.state.lock().await;
        if let Some(battle) = &state.battle
            && !battle.started
        {
            // NOTE: The battle can leak here, but realistically the only error should be that the
            // battle is ongoing, and we wouldn't want to delete the battle in that situation
            // anyway.
            self.battler_service_client.delete(battle.uuid).await.ok();
        }
    }
}

struct PlayerState {
    proposed_battles: BTreeSet<Uuid>,
    update_tx: broadcast::Sender<ProposedBattleUpdate>,
}

impl PlayerState {
    fn new() -> Self {
        let (update_tx, _) = broadcast::channel(16);
        Self {
            proposed_battles: BTreeSet::default(),
            update_tx,
        }
    }
}

#[derive(Default)]
struct BattlerMultiplayerServiceState {
    proposed_battles: BTreeMap<Uuid, Arc<ActiveProposedBattleManager>>,
    players: HashMap<String, Arc<Mutex<PlayerState>>>,
    join_set: JoinSet<()>,
}

impl BattlerMultiplayerServiceState {
    fn proposed_battle(&self, uuid: Uuid) -> Result<Arc<ActiveProposedBattleManager>> {
        self.proposed_battles
            .get(&uuid)
            .ok_or_else(|| Error::msg("proposed battle not found"))
            .cloned()
    }

    fn player_state(&mut self, player: &str) -> Arc<Mutex<PlayerState>> {
        self.players
            .entry(player.to_owned())
            .or_insert(Arc::new(Mutex::new(PlayerState::new())))
            .clone()
    }
}

/// Service for managing multiplayer battles on the [`battler`] battle engine.
pub struct BattlerMultiplayerService {
    battler_service_client: Arc<Box<dyn BattlerServiceClient>>,
    state: Arc<Mutex<BattlerMultiplayerServiceState>>,
}

impl BattlerMultiplayerService {
    /// Creates a new battler multiplayer service.
    pub fn new(battler_service_client: Box<dyn BattlerServiceClient>) -> Self {
        Self {
            battler_service_client: Arc::new(battler_service_client),
            state: Arc::new(Mutex::new(BattlerMultiplayerServiceState::default())),
        }
    }

    async fn active_proposed_battle_manager(
        &self,
        uuid: Uuid,
    ) -> Result<Arc<ActiveProposedBattleManager>> {
        self.state
            .lock()
            .await
            .proposed_battles
            .get(&uuid)
            .ok_or_else(|| Error::msg("proposed battle not found"))
            .cloned()
    }

    /// Proposes a battle.
    pub async fn propose_battle(
        self: Arc<Self>,
        options: ProposedBattleOptions,
    ) -> Result<ProposedBattle> {
        // TODO: Set up AI players in AiPlayerRegistry. They will need a simple client around this
        // service.

        self.create_proposed_battle(options).await
    }

    async fn create_proposed_battle(
        self: Arc<Self>,
        options: ProposedBattleOptions,
    ) -> Result<ProposedBattle> {
        let uuid = Uuid::new_v4();
        let result = self
            .clone()
            .create_proposed_battle_internal(uuid, options)
            .await;
        if result.is_err() {
            self.delete_proposed_battle(uuid).await;
        }
        result
    }

    async fn create_proposed_battle_internal(
        self: Arc<Self>,
        uuid: Uuid,
        options: ProposedBattleOptions,
    ) -> Result<ProposedBattle> {
        let creator = options.service_options.creator.clone();
        let active_proposed_battle = ActiveProposedBattle::new(uuid, options);

        let players = active_proposed_battle.players();

        if !players.contains(&creator) {
            return Err(Error::msg("you must participate in the battle"));
        }

        let active_proposed_battle_manager = ActiveProposedBattleManager::new(
            active_proposed_battle,
            self.battler_service_client.clone(),
            self.state.clone(),
        );
        let active_proposed_battle_manager = Arc::new(active_proposed_battle_manager);

        {
            let mut state = self.state.lock().await;
            state
                .proposed_battles
                .insert(uuid, active_proposed_battle_manager.clone());
            for player in players {
                let player_state = state
                    .players
                    .entry(player)
                    .or_insert(Arc::new(Mutex::new(PlayerState::new())));
                player_state.lock().await.proposed_battles.insert(uuid);
            }
        }

        active_proposed_battle_manager.start().await;

        // Creator auto-accepts.
        active_proposed_battle_manager
            .respond(&creator, &ProposedBattleResponse { accept: true })
            .await?;

        self.state
            .lock()
            .await
            .join_set
            .spawn(Self::proposed_battle_housekeeping(
                Arc::downgrade(&self),
                Arc::downgrade(&active_proposed_battle_manager),
            ));

        Ok(active_proposed_battle_manager.proposed_battle().await)
    }

    async fn delete_proposed_battle(&self, uuid: Uuid) {
        let proposed_battle = self.state.lock().await.proposed_battles.remove(&uuid);
        let proposed_battle = match proposed_battle {
            Some(proposed_battle) => proposed_battle,
            None => return,
        };

        proposed_battle.delete().await;

        let state = self.state.lock().await;
        for player in proposed_battle.players().await {
            if let Some(player) = state.players.get(&player) {
                player.lock().await.proposed_battles.remove(&uuid);
            }
        }
    }

    async fn proposed_battle_housekeeping(
        battler_multiplayer_service: Weak<Self>,
        active_proposed_battle_manager: Weak<ActiveProposedBattleManager>,
    ) {
        while let Some(battler_multiplayer_service) = battler_multiplayer_service.upgrade()
            && let Some(active_proposed_battle_manager) = active_proposed_battle_manager.upgrade()
        {
            if active_proposed_battle_manager
                .deletion_reason()
                .await
                .is_some()
            {
                battler_multiplayer_service
                    .delete_proposed_battle(active_proposed_battle_manager.uuid().await)
                    .await;
                break;
            } else if active_proposed_battle_manager.needs_to_watch_battle().await {
                active_proposed_battle_manager.watch_battle().await;
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    }

    /// Looks up a proposed battle.
    pub async fn proposed_battle(&self, proposed_battle: Uuid) -> Result<ProposedBattle> {
        Ok(self
            .active_proposed_battle_manager(proposed_battle)
            .await?
            .proposed_battle()
            .await)
    }

    /// Lists proposed battles for a player.
    pub async fn proposed_battles_for_player(
        &self,
        player: &str,
        count: usize,
        offset: usize,
    ) -> Vec<ProposedBattle> {
        let count = count.min(100);
        let player_state = self.state.lock().await.player_state(player);
        let uuids = player_state
            .lock()
            .await
            .proposed_battles
            .iter()
            .skip(offset)
            .take(count)
            .cloned()
            .collect::<Vec<_>>();
        let mut proposed_battles = Vec::with_capacity(count);
        for uuid in uuids {
            // Battle could be deleted while we are reading.
            if let Ok(active_proposed_battle_manager) =
                self.active_proposed_battle_manager(uuid).await
            {
                proposed_battles.push(active_proposed_battle_manager.proposed_battle().await);
            }
        }
        proposed_battles
    }

    /// Responds to a proposed battle.
    pub async fn respond_to_proposed_battle(
        &self,
        proposed_battle: Uuid,
        player: &str,
        response: &ProposedBattleResponse,
    ) -> Result<()> {
        let proposed_battle = self.state.lock().await.proposed_battle(proposed_battle)?;
        proposed_battle.respond(player, response).await
    }

    /// Subscribes to updates to a proposed battle.
    pub async fn proposed_battle_updates(
        &self,
        player: &str,
    ) -> Result<broadcast::Receiver<ProposedBattleUpdate>> {
        Ok(self
            .state
            .lock()
            .await
            .player_state(player)
            .lock()
            .await
            .update_tx
            .subscribe())
    }
}

#[cfg(test)]
mod battler_multiplayer_service_test {
    use std::{
        sync::Arc,
        time::{
            Duration,
            Instant,
            SystemTime,
        },
        usize,
    };

    use ahash::{
        HashMap,
        HashSet,
    };
    use anyhow::{
        Error,
        Result,
    };
    use battler::{
        BattleType,
        CoreBattleOptions,
        FieldData,
        FormatData,
        Id,
        MonData,
        PlayerData,
        Rule,
        SideData,
        TeamData,
    };
    use battler_service::{
        BattleServiceOptions,
        BattleState,
        BattlerService,
        PlayerState,
        Timer,
        Timers,
    };
    use battler_service_client::battler_service_client_over_direct_service;
    use battler_test_utils::static_local_data_store;
    use tokio::sync::broadcast;
    use uuid::Uuid;

    use crate::{
        BattlerMultiplayerService,
        Player,
        PlayerStatus,
        ProposedBattleOptions,
        ProposedBattleRejection,
        ProposedBattleResponse,
        ProposedBattleUpdate,
        Side,
    };

    fn battler_service() -> Arc<BattlerService<'static>> {
        Arc::new(BattlerService::new(static_local_data_store()))
    }

    fn battler_multiplayer_service() -> Arc<BattlerMultiplayerService> {
        battler_multiplayer_service_over_battler_service(battler_service())
    }

    fn battler_multiplayer_service_over_battler_service(
        battler_service: Arc<BattlerService<'static>>,
    ) -> Arc<BattlerMultiplayerService> {
        Arc::new(BattlerMultiplayerService::new(
            battler_service_client_over_direct_service(battler_service),
        ))
    }

    fn team_data() -> TeamData {
        TeamData {
            members: Vec::from_iter([MonData {
                name: "Pikachu".to_owned(),
                species: "Pikachu".to_owned(),
                ability: "Static".to_owned(),
                moves: Vec::from_iter(["Headbutt".to_owned()]),
                level: 5,
                ..Default::default()
            }]),
            ..Default::default()
        }
    }

    fn battle_options() -> CoreBattleOptions {
        CoreBattleOptions {
            seed: Some(0),
            format: FormatData {
                battle_type: BattleType::Singles,
                rules: HashSet::from_iter([Rule::Value {
                    name: Id::from("Species Clause"),
                    value: String::default(),
                }]),
            },
            field: FieldData::default(),
            side_1: SideData {
                name: "Side 1".to_owned(),
                players: Vec::from_iter([PlayerData {
                    id: "player-1".to_owned(),
                    name: "Player 1".to_owned(),
                    team: TeamData::default(),
                    ..Default::default()
                }]),
            },
            side_2: SideData {
                name: "Side 2".to_owned(),
                players: Vec::from_iter([PlayerData {
                    id: "player-2".to_owned(),
                    name: "Player 2".to_owned(),
                    team: TeamData::default(),
                    ..Default::default()
                }]),
            },
        }
    }

    fn battle_service_options<S>(creator: S) -> BattleServiceOptions
    where
        S: Into<String>,
    {
        BattleServiceOptions {
            creator: creator.into(),
            timers: Timers {
                battle: Some(Timer {
                    secs: 60,
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        }
    }

    fn proposed_battle_options<S>(creator: S) -> ProposedBattleOptions
    where
        S: Into<String>,
    {
        ProposedBattleOptions {
            battle_options: battle_options(),
            service_options: battle_service_options(creator),
            timeout: Duration::from_secs(30),
            ai_players: HashMap::default(),
        }
    }

    async fn read_all_entries_from_update_rx_stopping_at_deleted_or_timeout(
        update_rx: &mut broadcast::Receiver<ProposedBattleUpdate>,
        timeout: Duration,
    ) -> Vec<ProposedBattleUpdate> {
        let deadline = Instant::now() + timeout;
        let mut updates = Vec::new();
        loop {
            tokio::select! {
                update = update_rx.recv() => {
                    match update {
                        Ok(update) => {
                            let deleted = update.deletion_reason.is_some();
                            updates.push(update);
                            if deleted {
                                break;
                            }
                        },
                        Err(_) => break,
                    }
                }
                _ = tokio::time::sleep_until(deadline.into()) => break,
            }
        }

        // Past deadline, read everything else available.
        while let Ok(update) = update_rx.try_recv() {
            let deleted = update.deletion_reason.is_some();
            updates.push(update);
            if deleted {
                break;
            }
        }

        updates
    }

    async fn wait_until_proposed_battle_deleted(
        service: &BattlerMultiplayerService,
        proposed_battle: Uuid,
        timeout: Duration,
    ) -> Result<()> {
        let deadline = SystemTime::now() + timeout;
        while SystemTime::now() < deadline {
            if let Err(_) = service.proposed_battle(proposed_battle).await {
                return Ok(());
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
        Err(Error::msg("deadline exceeded"))
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cannot_find_invalid_proposed_battle() {
        let service = battler_multiplayer_service();
        assert_matches::assert_matches!(service.proposed_battle(Uuid::new_v4()).await, Err(err) => {
            assert_eq!(err.to_string(), "proposed battle not found");
        });
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn cannot_create_proposed_battle_if_not_participating() {
        let service = battler_multiplayer_service();
        assert_matches::assert_matches!(
            service
                .clone()
                .propose_battle(proposed_battle_options("player-3"))
                .await,
            Err(err) => {
                assert_eq!(err.to_string(), "you must participate in the battle");
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn creates_proposed_battle() {
        let service = battler_multiplayer_service();
        let proposed_battle = service
            .clone()
            .propose_battle(proposed_battle_options("player-1"))
            .await
            .unwrap();
        pretty_assertions::assert_eq!(
            proposed_battle.sides,
            [
                Side {
                    name: "Side 1".to_owned(),
                    players: Vec::from_iter([Player {
                        id: "player-1".to_owned(),
                        name: "Player 1".to_owned(),
                        status: Some(PlayerStatus::Accepted),
                    }])
                },
                Side {
                    name: "Side 2".to_owned(),
                    players: Vec::from_iter([Player {
                        id: "player-2".to_owned(),
                        name: "Player 2".to_owned(),
                        status: None,
                    }])
                }
            ]
        );
        assert_matches::assert_matches!(service.proposed_battle(proposed_battle.uuid).await, Ok(lookup) => {
            pretty_assertions::assert_eq!(lookup, proposed_battle);
        });
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn rejection_deletes_proposed_battle() {
        let service = battler_multiplayer_service();
        let proposed_battle = service
            .clone()
            .propose_battle(proposed_battle_options("player-1"))
            .await
            .unwrap();
        let mut update_rx = service.proposed_battle_updates("player-1").await.unwrap();

        assert_matches::assert_matches!(
            service
                .respond_to_proposed_battle(
                    proposed_battle.uuid,
                    "player-2",
                    &ProposedBattleResponse { accept: false },
                )
                .await,
            Ok(())
        );

        let updates = read_all_entries_from_update_rx_stopping_at_deleted_or_timeout(
            &mut update_rx,
            Duration::from_secs(5),
        )
        .await;
        assert!(updates.len() > 0, "{updates:?}");

        let update = updates.last().unwrap();
        assert_eq!(
            update.proposed_battle.sides[1].players[0].status,
            Some(PlayerStatus::Rejected)
        );
        assert_matches::assert_matches!(update.battle, None);
        assert_matches::assert_matches!(&update.rejection, Some(rejection) => {
            pretty_assertions::assert_eq!(
                rejection,
                &ProposedBattleRejection {
                    players: Vec::from_iter(["player-2".to_owned()]),
                }
            );
        });
        assert_matches::assert_matches!(&update.deletion_reason, Some(reason) => {
            assert_eq!(reason, "rejected");
        });
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn creator_can_reject() {
        let service = battler_multiplayer_service();
        let proposed_battle = service
            .clone()
            .propose_battle(proposed_battle_options("player-1"))
            .await
            .unwrap();
        let mut update_rx = service.proposed_battle_updates("player-1").await.unwrap();

        assert_matches::assert_matches!(
            service
                .respond_to_proposed_battle(
                    proposed_battle.uuid,
                    "player-1",
                    &ProposedBattleResponse { accept: false },
                )
                .await,
            Ok(())
        );

        let updates = read_all_entries_from_update_rx_stopping_at_deleted_or_timeout(
            &mut update_rx,
            Duration::from_secs(5),
        )
        .await;
        assert!(updates.len() > 0, "{updates:?}");

        let update = updates.last().unwrap();
        assert_eq!(
            update.proposed_battle.sides[0].players[0].status,
            Some(PlayerStatus::Rejected)
        );
        assert_matches::assert_matches!(update.battle, None);
        assert_matches::assert_matches!(&update.rejection, Some(rejection) => {
            pretty_assertions::assert_eq!(
                rejection,
                &ProposedBattleRejection {
                    players: Vec::from_iter(["player-1".to_owned()]),
                }
            );
        });
        assert_matches::assert_matches!(&update.deletion_reason, Some(reason) => {
            assert_eq!(reason, "rejected");
        });
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn timeout_deletes_proposed_battle() {
        let service = battler_multiplayer_service();
        let mut options = proposed_battle_options("player-1");
        options.timeout = Duration::from_secs(2);
        assert_matches::assert_matches!(service.clone().propose_battle(options).await, Ok(_));

        let mut update_rx = service.proposed_battle_updates("player-1").await.unwrap();

        let updates = read_all_entries_from_update_rx_stopping_at_deleted_or_timeout(
            &mut update_rx,
            Duration::from_secs(5),
        )
        .await;
        assert!(updates.len() > 0, "{updates:?}");

        let update = updates.last().unwrap();
        assert_matches::assert_matches!(update.battle, None);
        assert_matches::assert_matches!(&update.rejection, None);
        assert_matches::assert_matches!(&update.deletion_reason, Some(reason) => {
            assert_eq!(reason, "deadline exceeded");
        });
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn battle_created_when_accepted() {
        let battler_service = battler_service();
        let service = battler_multiplayer_service_over_battler_service(battler_service.clone());
        let proposed_battle = service
            .clone()
            .propose_battle(proposed_battle_options("player-1"))
            .await
            .unwrap();
        let mut update_rx = service.proposed_battle_updates("player-1").await.unwrap();

        assert_matches::assert_matches!(
            service
                .respond_to_proposed_battle(
                    proposed_battle.uuid,
                    "player-2",
                    &ProposedBattleResponse { accept: true },
                )
                .await,
            Ok(())
        );

        let update = update_rx.recv().await.unwrap();

        assert_eq!(
            update.proposed_battle.sides[1].players[0].status,
            Some(PlayerStatus::Accepted)
        );
        assert_matches::assert_matches!(&update.rejection, None);
        assert_matches::assert_matches!(&update.deletion_reason, None);

        let battle = update.battle.unwrap();
        assert_matches::assert_matches!(battler_service.battle(battle).await, Ok(battle) => {
            assert_eq!(battle.state, BattleState::Preparing);
            assert_eq!(battle.sides[0].players[0].state, PlayerState::Waiting);
            assert_eq!(battle.sides[1].players[0].state, PlayerState::Waiting);
        });
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn proposed_battle_updates_when_team_updates() {
        let battler_service = battler_service();
        let service = battler_multiplayer_service_over_battler_service(battler_service.clone());
        let proposed_battle = service
            .clone()
            .propose_battle(proposed_battle_options("player-1"))
            .await
            .unwrap();
        let mut update_rx = service.proposed_battle_updates("player-1").await.unwrap();

        assert_matches::assert_matches!(
            service
                .respond_to_proposed_battle(
                    proposed_battle.uuid,
                    "player-2",
                    &ProposedBattleResponse { accept: true },
                )
                .await,
            Ok(())
        );

        let battle = update_rx.recv().await.unwrap().battle.unwrap();

        assert_matches::assert_matches!(
            battler_service
                .update_team(battle, "player-1", team_data())
                .await,
            Ok(())
        );

        let update = update_rx.recv().await.unwrap();
        assert_matches::assert_matches!(update.deletion_reason, None);

        // Does not pass Species Clause.
        let mut invalid_team_data = team_data();
        invalid_team_data
            .members
            .push(invalid_team_data.members[0].clone());
        assert_matches::assert_matches!(
            battler_service
                .update_team(battle, "player-2", invalid_team_data)
                .await,
            Ok(())
        );

        let update = update_rx.recv().await.unwrap();
        assert_matches::assert_matches!(update.deletion_reason, None);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn battle_starting_deletes_proposed_battle() {
        let battler_service = battler_service();
        let service = battler_multiplayer_service_over_battler_service(battler_service.clone());
        let proposed_battle = service
            .clone()
            .propose_battle(proposed_battle_options("player-1"))
            .await
            .unwrap();
        let mut update_rx = service.proposed_battle_updates("player-1").await.unwrap();

        assert_matches::assert_matches!(
            service
                .respond_to_proposed_battle(
                    proposed_battle.uuid,
                    "player-2",
                    &ProposedBattleResponse { accept: true },
                )
                .await,
            Ok(())
        );

        let battle = update_rx.recv().await.unwrap().battle.unwrap();

        assert_matches::assert_matches!(
            battler_service
                .update_team(battle, "player-1", team_data())
                .await,
            Ok(())
        );
        assert_matches::assert_matches!(
            battler_service
                .update_team(battle, "player-2", team_data())
                .await,
            Ok(())
        );

        let updates = read_all_entries_from_update_rx_stopping_at_deleted_or_timeout(
            &mut update_rx,
            Duration::from_secs(5),
        )
        .await;
        let update = updates.last().unwrap();

        assert_matches::assert_matches!(&update.deletion_reason, Some(reason) => {
            assert_eq!(reason, "fulfilled");
        });

        assert_matches::assert_matches!(
            wait_until_proposed_battle_deleted(
                &service,
                proposed_battle.uuid,
                Duration::from_secs(5)
            )
            .await,
            Ok(())
        );

        assert_matches::assert_matches!(service.proposed_battle(proposed_battle.uuid).await, Err(err) => {
            assert_eq!(err.to_string(), "proposed battle not found");
        })
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn rejection_deletes_underlying_battle_after_creation() {
        let battler_service = battler_service();
        let service = battler_multiplayer_service_over_battler_service(battler_service.clone());
        let proposed_battle = service
            .clone()
            .propose_battle(proposed_battle_options("player-1"))
            .await
            .unwrap();
        let mut update_rx = service.proposed_battle_updates("player-1").await.unwrap();

        assert_matches::assert_matches!(
            service
                .respond_to_proposed_battle(
                    proposed_battle.uuid,
                    "player-2",
                    &ProposedBattleResponse { accept: true },
                )
                .await,
            Ok(())
        );

        let battle = update_rx.recv().await.unwrap().battle.unwrap();

        assert_matches::assert_matches!(
            service
                .respond_to_proposed_battle(
                    proposed_battle.uuid,
                    "player-2",
                    &ProposedBattleResponse { accept: false },
                )
                .await,
            Ok(())
        );

        assert_matches::assert_matches!(
            wait_until_proposed_battle_deleted(
                &service,
                proposed_battle.uuid,
                Duration::from_secs(5)
            )
            .await,
            Ok(())
        );

        assert_matches::assert_matches!(battler_service.battle(battle).await, Err(err) => {
            assert_eq!(err.to_string(), "battle does not exist");
        });
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn lists_proposed_battles_for_player() {
        let service = battler_multiplayer_service();
        let proposed_battle_1 = service
            .clone()
            .propose_battle(proposed_battle_options("player-1"))
            .await
            .unwrap();
        let proposed_battle_2 = service
            .clone()
            .propose_battle(proposed_battle_options("player-2"))
            .await
            .unwrap();

        pretty_assertions::assert_eq!(
            service
                .proposed_battles_for_player("player-1", usize::MAX, 0)
                .await
                .into_iter()
                .map(|proposed_battle| proposed_battle.uuid)
                .collect::<HashSet<_>>(),
            HashSet::from_iter([proposed_battle_1.uuid, proposed_battle_2.uuid])
        );
        pretty_assertions::assert_eq!(
            service
                .proposed_battles_for_player("player-2", usize::MAX, 0)
                .await
                .into_iter()
                .map(|proposed_battle| proposed_battle.uuid)
                .collect::<HashSet<_>>(),
            HashSet::from_iter([proposed_battle_1.uuid, proposed_battle_2.uuid])
        );
        pretty_assertions::assert_eq!(
            service
                .proposed_battles_for_player("player-3", usize::MAX, 0)
                .await
                .into_iter()
                .map(|proposed_battle| proposed_battle.uuid)
                .collect::<HashSet<_>>(),
            HashSet::default()
        );
    }
}
