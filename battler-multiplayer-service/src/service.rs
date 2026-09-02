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
        Instant,
        SystemTime,
    },
};

const MAX_OUTGOING_PROPOSALS: usize = 3;
const MAX_INCOMING_PROPOSALS: usize = 5;
const PROPOSAL_COOLDOWN: Duration = Duration::from_secs(3);
const HOUSEKEEPING_INTERVAL: Duration = Duration::from_secs(2);

use ahash::HashMap;
use anyhow::{
    Error,
    Result,
};
use battler::DataStoreByName;
use battler_service_client::BattlerServiceClient;
use futures_util::lock::Mutex;
use tokio::{
    sync::{
        broadcast,
        mpsc,
    },
    task::JoinSet,
};
use uuid::Uuid;

use crate::{
    AiPlayers,
    DirectBattlerMultiplayerServiceClient,
    MultiplayerError,
    Player,
    PlayerStatus,
    ProposedBattle,
    ProposedBattleOptions,
    ProposedBattleRejection,
    ProposedBattleResponse,
    ProposedBattleUpdate,
    ProposedSpecialBattleOptions,
    Side,
    SpecialBattle,
    ai::{
        AiPlayerModules,
        AiPlayerRegistry,
    },
};

#[derive(Debug, Clone)]
enum ActiveProposedBattleKind {
    Standard(ProposedBattleOptions),
    Special(ProposedSpecialBattleOptions),
}

#[derive(Debug)]
struct ActiveProposedBattle {
    kind: Option<ActiveProposedBattleKind>,
    proposed_battle: ProposedBattle,
}

impl ActiveProposedBattle {
    fn new(uuid: Uuid, options: ProposedBattleOptions) -> Self {
        let timeout = options.timeout.min(Duration::from_mins(5));
        let proposed_battle = ProposedBattle {
            uuid,
            sides: Vec::from_iter([
                Side::from(&options.battle_options.side_1),
                Side::from(&options.battle_options.side_2),
            ]),
            deadline: SystemTime::now() + timeout,
            battle: None,
            battle_type: options.battle_options.format.battle_type,
            rules: options.battle_options.format.rules.clone(),
            timers: options.service_options.timers.clone(),
            special: None,
        };
        Self {
            kind: Some(ActiveProposedBattleKind::Standard(options)),
            proposed_battle,
        }
    }

    fn new_special(uuid: Uuid, options: ProposedSpecialBattleOptions) -> Self {
        let timeout = options.timeout.min(Duration::from_mins(5));
        let (battle_type, rules) = match &options.special_battle {
            SpecialBattle::Chaos(chaos_opts) => {
                let rules =
                    Vec::from_iter(["Species Clause".to_string(), "Item Clause".to_string()]);
                (chaos_opts.mode.battle_type(), rules)
            }
        };
        let proposed_battle = ProposedBattle {
            uuid,
            sides: Vec::from_iter([Side::from(&options.side_1), Side::from(&options.side_2)]),
            deadline: SystemTime::now() + timeout,
            battle: None,
            battle_type,
            rules,
            timers: options.service_options.timers.clone(),
            special: Some(options.special_battle.clone()),
        };
        Self {
            kind: Some(ActiveProposedBattleKind::Special(options)),
            proposed_battle,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum ActiveProposedBattleInflightState {
    #[default]
    Idle,
    CreatingBattle,
    StartingBattle {
        pending_start_check: bool,
    },
}

#[derive(Debug)]
struct ActiveProposedBattleManagerState {
    proposed_battle: ActiveProposedBattle,
    battle: Option<UnderlyingBattle>,
    error: Option<String>,
    watcher_task_started: bool,
    inflight_state: ActiveProposedBattleInflightState,
    join_set: JoinSet<()>,
}

struct ActiveProposedBattleManager {
    uuid: Uuid,
    players: Vec<String>,
    state: Mutex<ActiveProposedBattleManagerState>,

    data: &'static dyn DataStoreByName,
    battler_service_client: Arc<Box<dyn BattlerServiceClient>>,
    battler_multiplayer_service_state: Arc<Mutex<BattlerMultiplayerServiceState>>,
}

impl ActiveProposedBattleManager {
    fn new(
        proposed_battle: ActiveProposedBattle,
        data: &'static dyn DataStoreByName,
        battler_service_client: Arc<Box<dyn BattlerServiceClient>>,
        battler_multiplayer_service_state: Arc<Mutex<BattlerMultiplayerServiceState>>,
    ) -> Self {
        let uuid = proposed_battle.uuid();
        let players = proposed_battle.players();
        Self {
            uuid,
            players,
            state: Mutex::new(ActiveProposedBattleManagerState {
                proposed_battle,
                battle: None,
                error: None,
                watcher_task_started: false,
                inflight_state: ActiveProposedBattleInflightState::Idle,
                join_set: JoinSet::default(),
            }),
            data,
            battler_service_client,
            battler_multiplayer_service_state,
        }
    }

    fn uuid(&self) -> Uuid {
        self.uuid
    }

    async fn proposed_battle(&self) -> ProposedBattle {
        let state = self.state.lock().await;
        state.proposed_battle.proposed_battle()
    }

    fn players(&self) -> &[String] {
        &self.players
    }

    async fn proposed_battle_rejection(&self) -> Option<ProposedBattleRejection> {
        let state = self.state.lock().await;
        state.proposed_battle.proposed_battle_rejection()
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
        } else if SystemTime::now() >= deadline {
            Some("deadline exceeded".to_owned())
        } else if let Some(err) = error {
            Some(format!("internal error: {err:#}"))
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
        log::info!(
            "Publishing update for proposed battle {}: {update:?}",
            self.uuid
        );
        let global_update_tx = {
            let state = self.battler_multiplayer_service_state.lock().await;
            state.global_update_tx.clone()
        };
        global_update_tx.send(update.clone()).await.ok();
        for player in self.players() {
            self.publish_update_to_player(player, update.clone()).await;
        }
    }

    async fn publish_update_to_player(&self, player: &str, update: ProposedBattleUpdate) {
        let update_tx = {
            let mut state = self.battler_multiplayer_service_state.lock().await;
            state.player_state(player).update_tx.clone()
        };
        update_tx.send(update).ok();
    }

    async fn start(self: &Arc<Self>) {
        self.update().await;
    }

    async fn respond(
        self: &Arc<Self>,
        player: &str,
        response: &ProposedBattleResponse,
    ) -> Result<()> {
        let battle_started = {
            let state = self.state.lock().await;
            state.battle.as_ref().is_some_and(|battle| battle.started)
        };
        if battle_started {
            // Accepting a battle that has started does not need to result in a failure.
            if response.accept {
                return Ok(());
            }
            return Err(Error::msg("battle started"));
        }
        {
            let mut state = self.state.lock().await;
            state.proposed_battle.respond(player, response)?;
        }
        self.update().await;
        Ok(())
    }

    async fn update(self: &Arc<Self>) {
        if let Err(err) = self.update_internal().await {
            log::error!("Update for proposed battle {} failed: {err:?}", self.uuid);
            let mut state = self.state.lock().await;
            if !state.battle.as_ref().is_some_and(|b| b.started) {
                state.error = Some(format!("{err:#}"));
            }
        } else {
            let mut state = self.state.lock().await;
            if state.battle.as_ref().is_some_and(|b| b.started) {
                state.error = None;
            }
        }
        self.publish_update().await;
    }

    async fn update_internal(self: &Arc<Self>) -> Result<()> {
        self.create_battle_if_needed().await?;
        if self.needs_to_watch_battle().await {
            self.watch_battle();
        }
        self.start_battle_if_needed().await?;
        Ok(())
    }

    async fn create_battle_if_needed(&self) -> Result<()> {
        let options = {
            let mut state = self.state.lock().await;
            if state.battle.is_none()
                && state.inflight_state == ActiveProposedBattleInflightState::Idle
                && state.proposed_battle.ready_to_create()
            {
                state.inflight_state = ActiveProposedBattleInflightState::CreatingBattle;
                match state.proposed_battle.kind.take() {
                    Some(ActiveProposedBattleKind::Standard(options)) => {
                        Some((options.battle_options, options.service_options))
                    }
                    Some(ActiveProposedBattleKind::Special(options)) => {
                        match &options.special_battle {
                            SpecialBattle::Chaos(chaos_opts) => {
                                let mut battle_opts =
                                    battler_fuzz_test_generator::generate_random_battle(
                                        self.data,
                                        chaos_opts.mode.battle_type(),
                                        chaos_opts.mode.team_size(),
                                        None,
                                    )?;
                                let mut side_1 = options.side_1;
                                let mut side_2 = options.side_2;
                                for (i, player) in side_1.players.iter_mut().enumerate() {
                                    if let Some(generated_player) =
                                        battle_opts.side_1.players.get(i)
                                    {
                                        player.team = generated_player.team.clone();
                                    }
                                }
                                for (i, player) in side_2.players.iter_mut().enumerate() {
                                    if let Some(generated_player) =
                                        battle_opts.side_2.players.get(i)
                                    {
                                        player.team = generated_player.team.clone();
                                    }
                                }
                                battle_opts.side_1 = side_1;
                                battle_opts.side_2 = side_2;
                                let mut service_options = options.service_options;
                                service_options.special =
                                    Some(format!("{}", options.special_battle));
                                Some((battle_opts, service_options))
                            }
                        }
                    }
                    None => None,
                }
            } else {
                None
            }
        };

        if let Some((battle_options, service_options)) = options {
            log::info!("Creating battle for proposed battle {}", self.uuid);
            let result = self
                .battler_service_client
                .create(battle_options, service_options)
                .await;
            let mut state = self.state.lock().await;
            state.inflight_state = ActiveProposedBattleInflightState::Idle;
            let battle = result?;
            log::info!(
                "Created battle {} for proposed battle {}",
                battle.uuid,
                self.uuid
            );
            state.proposed_battle.proposed_battle.battle = Some(battle.uuid);
            state.battle = Some(UnderlyingBattle {
                uuid: battle.uuid,
                started: false,
            });
        }
        Ok(())
    }

    async fn start_battle_if_needed(&self) -> Result<()> {
        loop {
            let uuid = {
                let mut state = self.state.lock().await;
                let Some(battle) = &state.battle else {
                    return Ok(());
                };
                if battle.started {
                    return Ok(());
                }
                let uuid = battle.uuid;

                if matches!(
                    state.inflight_state,
                    ActiveProposedBattleInflightState::StartingBattle { .. }
                ) {
                    state.inflight_state = ActiveProposedBattleInflightState::StartingBattle {
                        pending_start_check: true,
                    };
                    return Ok(());
                }

                state.inflight_state = ActiveProposedBattleInflightState::StartingBattle {
                    pending_start_check: false,
                };
                uuid
            };

            let result = async {
                let underlying_battle = self.battler_service_client.battle(uuid).await?;
                log::info!(
                    "Player states for battle {uuid}: {:?}",
                    underlying_battle.sides
                );
                if underlying_battle
                    .sides
                    .iter()
                    .flat_map(|side| side.players.iter())
                    .all(|player| player.state == battler_service::PlayerState::Ready)
                {
                    // Auto-start the battle.
                    log::info!("Starting battle {} for proposed battle {}", uuid, self.uuid);
                    self.battler_service_client.start(uuid).await?;
                    return Ok(true);
                }
                Ok(false)
            }
            .await;

            let mut state = self.state.lock().await;
            let pending_start_check = matches!(
                state.inflight_state,
                ActiveProposedBattleInflightState::StartingBattle {
                    pending_start_check: true
                }
            );
            state.inflight_state = ActiveProposedBattleInflightState::Idle;
            match result {
                Ok(true) => {
                    if let Some(battle) = &mut state.battle {
                        battle.started = true;
                    }
                    break;
                }
                Ok(false) => {}
                Err(err) if format!("{err:#}").contains("already started") => {
                    if let Some(battle) = &mut state.battle {
                        battle.started = true;
                    }
                    break;
                }
                Err(err) => return Err(err),
            }

            if !pending_start_check {
                break;
            }
        }
        Ok(())
    }

    async fn needs_to_watch_battle(&self) -> bool {
        let state = self.state.lock().await;
        state.battle.as_ref().is_some_and(|battle| !battle.started) && !state.watcher_task_started
    }

    fn watch_battle(self: &Arc<Self>) {
        let active_proposed_battle_manager = Arc::downgrade(self);
        let battler_service_client = self.battler_service_client.clone();
        let active = self.clone();
        tokio::spawn(async move {
            let mut state = active.state.lock().await;
            if state.watcher_task_started {
                return;
            }
            state.watcher_task_started = true;
            state.join_set.spawn(Self::watch_battle_until_started(
                active_proposed_battle_manager,
                battler_service_client,
            ));
        });
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
                log::error!(
                    "Watching battle for proposed battle {} failed: {err:?}",
                    active_proposed_battle_manager.uuid
                );
                active_proposed_battle_manager.state.lock().await.error = Some(format!("{err:#}"));
            }
        }
    }

    async fn watch_battle_until_started_internal(
        active_proposed_battle_manager: Weak<Self>,
        battler_service_client: Arc<Box<dyn BattlerServiceClient>>,
    ) -> Result<()> {
        let (uuid, battle, deadline) = {
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
            (active_proposed_battle_manager.uuid, battle, deadline)
        };

        log::info!("Watching battle {battle} for proposed battle {uuid} until started");

        if let Some(active_proposed_battle_manager) = active_proposed_battle_manager.upgrade() {
            active_proposed_battle_manager.update().await;
        }

        let process_log = async |entry: &str| {
            let entry = entry.strip_prefix("-battlerservice:").unwrap_or(entry);
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
                    let entries = match entry {
                        Ok(entry) => Vec::from_iter([entry.content]),
                        Err(broadcast::error::RecvError::Lagged(skipped)) => {
                            log::warn!("Battle log receiver for proposed battle watcher on {battle} lagged by {skipped} entries");
                            while let Ok(_) = battle_log_rx.try_recv() {}
                            match battler_service_client.full_log(battle, None).await {
                                Ok(full_log) => full_log,
                                Err(_) => continue,
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    };
                    let mut done = false;
                    for entry in entries {
                        if process_log(&entry).await {
                            done = true;
                            break;
                        }
                    }
                    if done {
                        log::info!("Battle {battle} started, done watching");
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

        let battle = {
            let state = self.state.lock().await;
            state.battle.clone()
        };
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
    outgoing_proposals: BTreeSet<Uuid>,
    incoming_proposals: BTreeSet<Uuid>,
    last_proposed_at: Option<Instant>,
    update_tx: broadcast::Sender<ProposedBattleUpdate>,
}

impl PlayerState {
    fn new() -> Self {
        let (update_tx, _) = broadcast::channel(48);
        Self {
            outgoing_proposals: BTreeSet::default(),
            incoming_proposals: BTreeSet::default(),
            last_proposed_at: None,
            update_tx,
        }
    }
}

struct BattlerMultiplayerServiceState {
    proposed_battles: BTreeMap<Uuid, Arc<ActiveProposedBattleManager>>,
    players: HashMap<String, PlayerState>,
    join_set: JoinSet<()>,
    global_update_tx: mpsc::Sender<ProposedBattleUpdate>,
    global_update_rx: Option<mpsc::Receiver<ProposedBattleUpdate>>,
}

impl BattlerMultiplayerServiceState {
    fn new() -> Self {
        let (global_update_tx, global_update_rx) = mpsc::channel(4096);
        Self {
            proposed_battles: BTreeMap::default(),
            players: HashMap::default(),
            join_set: JoinSet::default(),
            global_update_tx,
            global_update_rx: Some(global_update_rx),
        }
    }

    fn proposed_battle(&self, uuid: Uuid) -> Result<Arc<ActiveProposedBattleManager>> {
        self.proposed_battles
            .get(&uuid)
            .ok_or_else(|| MultiplayerError::ProposedBattleNotFound.into())
            .cloned()
    }

    fn player_state(&mut self, player: &str) -> &mut PlayerState {
        self.players
            .entry(player.to_owned())
            .or_insert_with(PlayerState::new)
    }

    fn delete_proposed_battle(&mut self, uuid: Uuid) -> Option<Arc<ActiveProposedBattleManager>> {
        let proposed_battle = self.proposed_battles.remove(&uuid)?;
        for player in proposed_battle.players() {
            if let Some(player_state) = self.players.get_mut(player) {
                player_state.outgoing_proposals.remove(&uuid);
                player_state.incoming_proposals.remove(&uuid);
            }
        }
        Some(proposed_battle)
    }
}

/// Service for managing multiplayer battles on the [`battler`] battle engine.
pub struct BattlerMultiplayerService<'d> {
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
        let state = Arc::new(Mutex::new(BattlerMultiplayerServiceState::new()));
        let ai_player_registry = Mutex::new(AiPlayerRegistry::default());

        {
            let mut state_lock = state.lock().await;
            state_lock
                .join_set
                .spawn(BattlerMultiplayerService::clean_up_completed_tasks(
                    Arc::downgrade(&state),
                ));
            state_lock.join_set.spawn(
                BattlerMultiplayerService::proposed_battle_housekeeping_loop(Arc::downgrade(
                    &state,
                )),
            );
        }

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
        log::info!("Creating AI players: {ai_players:?}");
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
            .ok_or_else(|| MultiplayerError::ProposedBattleNotFound.into())
            .cloned()
    }

    /// Proposes a battle.
    pub async fn propose_battle(
        self: Arc<Self>,
        options: ProposedBattleOptions,
    ) -> Result<ProposedBattle> {
        self.create_proposed_battle(options).await
    }

    /// Proposes a special battle.
    pub async fn propose_special_battle(
        self: Arc<Self>,
        options: ProposedSpecialBattleOptions,
    ) -> Result<ProposedBattle> {
        self.create_proposed_special_battle(options).await
    }

    async fn delete_proposed_battle(
        state: Arc<Mutex<BattlerMultiplayerServiceState>>,
        uuid: Uuid,
        deletion_reason: String,
    ) {
        log::info!("Deleting proposed battle {uuid}: {deletion_reason}");
        let proposed_battle = {
            let mut state = state.lock().await;
            state.delete_proposed_battle(uuid)
        };

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
        if let Err(err) = &result {
            Self::delete_proposed_battle(
                self.state.clone(),
                uuid,
                format!("creation failed: {err:#}"),
            )
            .await;
        }
        log::info!("Created proposed battle {uuid}");
        result
    }

    async fn create_proposed_special_battle(
        self: Arc<Self>,
        options: ProposedSpecialBattleOptions,
    ) -> Result<ProposedBattle> {
        let uuid = Uuid::new_v4();
        let result = self
            .clone()
            .create_proposed_special_battle_internal(uuid, options)
            .await;
        if let Err(err) = &result {
            Self::delete_proposed_battle(
                self.state.clone(),
                uuid,
                format!("creation failed: {err:#}"),
            )
            .await;
        }
        log::info!("Created proposed special battle {uuid}");
        result
    }

    async fn register_and_start_proposed_battle(
        self: Arc<Self>,
        uuid: Uuid,
        creator: String,
        active_proposed_battle: ActiveProposedBattle,
    ) -> Result<ProposedBattle> {
        let players = active_proposed_battle.players();

        if !players.contains(&creator) {
            return Err(Error::msg("you must participate in the battle"));
        }

        let mut unique_players = BTreeSet::new();
        for player in &players {
            if !unique_players.insert(player) {
                return Err(Error::msg("duplicate players are not allowed"));
            }
        }

        // Validate quotas and cooldown, and insert into proposed_battles.
        let active_proposed_battle_manager = {
            let mut state = self.state.lock().await;

            // 1. Check creator cooldown and outgoing quota.
            let creator_state = state.player_state(&creator);
            if let Some(last_proposed_at) = creator_state.last_proposed_at {
                if last_proposed_at.elapsed() < PROPOSAL_COOLDOWN {
                    return Err(Error::msg("you are proposing battles too quickly"));
                }
            }
            if creator_state.outgoing_proposals.len() >= MAX_OUTGOING_PROPOSALS {
                return Err(Error::msg("you have too many active proposed battles"));
            }

            // 2. Check recipient incoming quota.
            for player in players.iter().filter(|p| **p != creator) {
                let recipient_state = state.player_state(player);
                if recipient_state.incoming_proposals.len() >= MAX_INCOMING_PROPOSALS {
                    return Err(Error::msg(format!(
                        "opponent {player} has too many pending incoming challenges"
                    )));
                }
            }

            // 3. Record validation approval and timestamp.
            state.player_state(&creator).last_proposed_at = Some(Instant::now());

            // 4. Create manager and register proposal.
            // SAFETY: `data` is valid for `'d` on `BattlerMultiplayerService` and will outlive
            // active proposed battles managed by this service instance.
            let data_static = unsafe {
                std::mem::transmute::<&'d dyn DataStoreByName, &'static dyn DataStoreByName>(
                    self.data,
                )
            };
            let active_proposed_battle_manager = Arc::new(ActiveProposedBattleManager::new(
                active_proposed_battle,
                data_static,
                self.battler_service_client.clone(),
                self.state.clone(),
            ));

            state
                .proposed_battles
                .insert(uuid, active_proposed_battle_manager.clone());

            for player in &players {
                let p_state = state.player_state(player);
                if player == &creator {
                    p_state.outgoing_proposals.insert(uuid);
                } else {
                    p_state.incoming_proposals.insert(uuid);
                }
            }

            active_proposed_battle_manager
        };

        active_proposed_battle_manager.start().await;

        // Creator auto-accepts.
        active_proposed_battle_manager
            .respond(&creator, &ProposedBattleResponse { accept: true })
            .await?;

        Ok(active_proposed_battle_manager.proposed_battle().await)
    }

    async fn create_proposed_battle_internal(
        self: Arc<Self>,
        uuid: Uuid,
        options: ProposedBattleOptions,
    ) -> Result<ProposedBattle> {
        // Validate battle options early.
        options
            .battle_options
            .validate()
            .map_err(|err| Error::msg(format!("invalid battle options: {err:#}")))?;

        let creator = options.service_options.creator.clone();

        // Ensure that human players other than the creator do not have pre-specified teams in the
        // proposal.
        {
            let ai_registry = self.ai_player_registry.lock().await;
            for side in &[
                &options.battle_options.side_1,
                &options.battle_options.side_2,
            ] {
                for player in &side.players {
                    if player.id != creator
                        && !ai_registry.is_ai_player(&player.id)
                        && !player.team.members.is_empty()
                    {
                        return Err(Error::msg(format!(
                            "cannot pre-specify team for player {}",
                            player.id
                        )));
                    }
                }
            }
        }
        let active_proposed_battle = ActiveProposedBattle::new(uuid, options);
        self.register_and_start_proposed_battle(uuid, creator, active_proposed_battle)
            .await
    }

    async fn create_proposed_special_battle_internal(
        self: Arc<Self>,
        uuid: Uuid,
        options: ProposedSpecialBattleOptions,
    ) -> Result<ProposedBattle> {
        let creator = options.service_options.creator.clone();
        let active_proposed_battle = ActiveProposedBattle::new_special(uuid, options);
        self.register_and_start_proposed_battle(uuid, creator, active_proposed_battle)
            .await
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

    async fn proposed_battle_housekeeping_loop(
        battler_multiplayer_service_state: Weak<Mutex<BattlerMultiplayerServiceState>>,
    ) {
        while let Some(state_mutex) = battler_multiplayer_service_state.upgrade() {
            let managers = {
                let state = state_mutex.lock().await;
                state.proposed_battles.values().cloned().collect::<Vec<_>>()
            };

            for manager in managers {
                if let Some(deletion_reason) = manager.deletion_reason().await {
                    Self::delete_proposed_battle(
                        state_mutex.clone(),
                        manager.uuid(),
                        deletion_reason,
                    )
                    .await;
                } else if manager.needs_to_watch_battle().await {
                    manager.watch_battle();
                }
            }

            tokio::time::sleep(HOUSEKEEPING_INTERVAL).await;
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
        let managers = {
            let mut state = self.state.lock().await;
            let player_state = state.player_state(player);
            let mut uuids = player_state
                .outgoing_proposals
                .iter()
                .chain(player_state.incoming_proposals.iter())
                .cloned()
                .collect::<Vec<_>>();
            uuids.sort();
            uuids
                .into_iter()
                .skip(offset)
                .take(count)
                .filter_map(|uuid| state.proposed_battles.get(&uuid).cloned())
                .collect::<Vec<_>>()
        };

        let mut proposed_battles = Vec::with_capacity(managers.len());
        for manager in managers {
            proposed_battles.push(manager.proposed_battle().await);
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
        let proposed_battle = {
            let state = self.state.lock().await;
            state.proposed_battle(proposed_battle)?
        };
        log::info!(
            "Received response to proposed battle {} for {player}: {response:?}",
            proposed_battle.uuid()
        );
        proposed_battle.respond(player, response).await?;
        Ok(proposed_battle.proposed_battle().await)
    }

    /// Subscribes to all proposed battle updates for the player.
    pub async fn proposed_battle_updates(
        &self,
        player: &str,
    ) -> Result<broadcast::Receiver<ProposedBattleUpdate>> {
        let mut state = self.state.lock().await;
        Ok(state.player_state(player).update_tx.subscribe())
    }

    /// Takes the global update receiver.
    pub async fn take_global_update_rx(&self) -> Option<mpsc::Receiver<ProposedBattleUpdate>> {
        let mut state = self.state.lock().await;
        state.global_update_rx.take()
    }
}
