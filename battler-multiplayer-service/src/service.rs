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
use battler::{
    DataStoreByName,
    SideData,
};
use battler_service_client::BattlerServiceClient;
use futures_util::lock::Mutex;
use tokio::{
    sync::broadcast,
    task::JoinSet,
};
use uuid::Uuid;

use crate::{
    AiPlayers,
    DirectBattlerMultiplayerServiceClient,
    Player,
    PlayerStatus,
    ProposedBattle,
    ProposedBattleOptions,
    ProposedBattleRejection,
    ProposedBattleResponse,
    ProposedBattleUpdate,
    Side,
    ai::{
        AiPlayerModules,
        AiPlayerRegistry,
    },
};

#[derive(Debug)]
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
            battle: None,
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

#[derive(Debug)]
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
        ProposedBattleUpdate {
            proposed_battle: self.proposed_battle().await,
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
        let player_state = self
            .battler_multiplayer_service_state
            .lock()
            .await
            .player_state(player)
            .clone();
        player_state.lock().await.update_tx.send(update).ok();
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
        self.create_battle_if_needed().await?;
        self.start_battle_if_needed().await?;
        Ok(())
    }

    async fn create_battle_if_needed(&self) -> Result<()> {
        let (underlying_battle, ready_to_create) = {
            let state = self.state.lock().await;
            (
                state.battle.clone(),
                state.proposed_battle.ready_to_create(),
            )
        };

        if underlying_battle.is_none() && ready_to_create {
            let (battle_options, service_options) = {
                let state = self.state.lock().await;
                (
                    state.proposed_battle.options.battle_options.clone(),
                    state.proposed_battle.options.service_options.clone(),
                )
            };
            let battle = self
                .battler_service_client
                .create(battle_options, service_options)
                .await?;
            {
                let mut state = self.state.lock().await;
                state.proposed_battle.proposed_battle.battle = Some(battle.uuid);
                state.battle = Some(UnderlyingBattle {
                    uuid: battle.uuid,
                    started: false,
                });
            }
        }
        Ok(())
    }

    async fn start_battle_if_needed(&self) -> Result<()> {
        let underlying_battle = self.state.lock().await.battle.clone();

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
                self.state
                    .lock()
                    .await
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

        let battle = self.state.lock().await.battle.clone();
        if let Some(battle) = battle
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

    async fn delete_proposed_battle(
        &mut self,
        uuid: Uuid,
    ) -> Option<Arc<ActiveProposedBattleManager>> {
        let proposed_battle = self.proposed_battles.remove(&uuid)?;
        let players = proposed_battle.players().await;

        for player in players {
            self.player_state(&player)
                .lock()
                .await
                .proposed_battles
                .remove(&uuid);
        }
        Some(proposed_battle)
    }
}

/// Service for managing multiplayer battles on the [`battler`] battle engine.
pub struct BattlerMultiplayerService<'d> {
    #[allow(unused)]
    data: &'d dyn DataStoreByName,
    battler_service_client: Arc<Box<dyn BattlerServiceClient>>,
    state: Arc<Mutex<BattlerMultiplayerServiceState>>,
    ai_player_registry: Mutex<AiPlayerRegistry<'d>>,
}

impl<'d> BattlerMultiplayerService<'d> {
    /// Creates a new battler multiplayer service.
    pub async fn new(
        data: &'d dyn DataStoreByName,
        battler_service_client: Arc<Box<dyn BattlerServiceClient>>,
    ) -> Self {
        let state = Arc::new(Mutex::new(BattlerMultiplayerServiceState::default()));
        let ai_player_registry = Mutex::new(AiPlayerRegistry::default());

        state
            .lock()
            .await
            .join_set
            .spawn(BattlerMultiplayerService::clean_up_completed_tasks(
                Arc::downgrade(&state),
            ));

        Self {
            data,
            battler_service_client,
            state,
            ai_player_registry,
        }
    }

    /// Creates AI players.
    ///
    /// Previously-existing AI players will be dropped.
    pub async fn create_ai_players(self: Arc<Self>, ai_players: AiPlayers) -> Result<()> {
        let modules = AiPlayerModules {
            data: self.data,
            battler_service_client: self.battler_service_client.clone(),
            battler_multiplayer_service_client: Arc::new(Box::new(
                DirectBattlerMultiplayerServiceClient::new(self.clone()),
            )),
        };
        let mut ai_player_registry = self.ai_player_registry.lock().await;
        for (id, options) in ai_players.players {
            ai_player_registry
                .create_ai_player(id, options, modules.clone())
                .await?;
        }
        Ok(())
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
        self.create_proposed_battle(options).await
    }

    async fn delete_proposed_battle(state: Arc<Mutex<BattlerMultiplayerServiceState>>, uuid: Uuid) {
        let proposed_battle = state.lock().await.delete_proposed_battle(uuid).await;
        if let Some(proposed_battle) = proposed_battle {
            proposed_battle.delete().await;
        }
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
            Self::delete_proposed_battle(self.state.clone(), uuid).await;
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

        let players = {
            let mut state = self.state.lock().await;
            state
                .proposed_battles
                .insert(uuid, active_proposed_battle_manager.clone());
            let mut player_states = Vec::default();
            for player in players {
                player_states.push(state.player_state(&player));
            }
            player_states
        };

        for player in players {
            player.lock().await.proposed_battles.insert(uuid);
        }

        active_proposed_battle_manager.start().await;

        // Creator auto-accepts.
        active_proposed_battle_manager
            .respond(&creator, &ProposedBattleResponse { accept: true })
            .await?;

        self.state.lock().await.join_set.spawn(
            BattlerMultiplayerService::proposed_battle_housekeeping(
                Arc::downgrade(&self.state),
                Arc::downgrade(&active_proposed_battle_manager),
            ),
        );

        Ok(active_proposed_battle_manager.proposed_battle().await)
    }

    async fn clean_up_completed_tasks(
        battler_multiplayer_service_state: Weak<Mutex<BattlerMultiplayerServiceState>>,
    ) {
        while let Some(battler_multiplayer_service_state) =
            battler_multiplayer_service_state.upgrade()
        {
            while let Some(_) = battler_multiplayer_service_state
                .lock()
                .await
                .join_set
                .try_join_next()
            {}
            tokio::time::sleep(Duration::from_mins(5)).await;
        }
    }

    async fn proposed_battle_housekeeping(
        battler_multiplayer_service_state: Weak<Mutex<BattlerMultiplayerServiceState>>,
        active_proposed_battle_manager: Weak<ActiveProposedBattleManager>,
    ) {
        while let Some(battler_multiplayer_service_state) =
            battler_multiplayer_service_state.upgrade()
            && let Some(active_proposed_battle_manager) = active_proposed_battle_manager.upgrade()
        {
            if active_proposed_battle_manager
                .deletion_reason()
                .await
                .is_some()
            {
                Self::delete_proposed_battle(
                    battler_multiplayer_service_state.clone(),
                    active_proposed_battle_manager.uuid().await,
                )
                .await;
                break;
            }
            if active_proposed_battle_manager.needs_to_watch_battle().await {
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
    ) -> Result<ProposedBattle> {
        let proposed_battle = self.state.lock().await.proposed_battle(proposed_battle)?;
        proposed_battle.respond(player, response).await?;
        Ok(proposed_battle.proposed_battle().await)
    }

    /// Subscribes to all proposed battle updates for the player.
    pub async fn proposed_battle_updates(
        &self,
        player: &str,
    ) -> Result<broadcast::Receiver<ProposedBattleUpdate>> {
        let player_state = self.state.lock().await.player_state(player);
        Ok(player_state.lock().await.update_tx.subscribe())
    }
}
