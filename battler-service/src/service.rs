use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    fmt::Display,
    pin::Pin,
    sync::{
        Arc,
        Weak,
    },
    time::{
        Duration,
        Instant,
    },
};

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
use serde::{
    Deserialize,
    Serialize,
};
use tokio::{
    sync::{
        broadcast,
        mpsc,
    },
    task::JoinSet,
};
use uuid::Uuid;

use crate::{
    Battle,
    BattleMetadata,
    BattlePreview,
    BattleState,
    BattleStatus,
    GlobalLogEntry,
    Player,
    PlayerPreview,
    PlayerState,
    PlayerValidation,
    Side,
    SidePreview,
    Timers,
    log::{
        Log,
        LogEntry,
        SplitLogs,
    },
    timer::{
        TimerState,
        TimerType,
    },
};

/// Options for configuring how [`BattlerService`] manages an individual battle.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct BattleServiceOptions {
    /// Player who created the battle.
    #[serde(default)]
    pub creator: String,

    /// Battle timers.
    #[serde(default)]
    pub timers: Timers,
}

#[derive(Debug, Clone, Copy)]
enum TimerLogType {
    Warning,
    Done,
}

/// An ongoing battle, managed by [`BattlerService`].
///
/// Operations on this object are intended to be **atomic**. In other words, the battle mutex is
/// locked for each operation here.
///
/// For non-atomic operations, the [`LiveBattleManager`] may make mutations to state in this object.
struct LiveBattle<'d> {
    uuid: Uuid,
    battle: PublicCoreBattle<'d>,
    metadata: BattleMetadata,
    sides: Vec<Side>,
    error: Option<String>,
    logs: SplitLogs,

    timers: BTreeMap<TimerType, TimerState>,

    choice_made_tx: broadcast::Sender<String>,
    cancel_timers_tx: broadcast::Sender<()>,

    current_timer_tasks: JoinSet<()>,
    proceed_tasks: JoinSet<()>,
}

impl<'d> LiveBattle<'d> {
    fn new(
        options: CoreBattleOptions,
        engine_options: CoreBattleEngineOptions,
        service_options: BattleServiceOptions,
        data: &'d dyn DataStore,
        global_log_tx: mpsc::UnboundedSender<GlobalLogEntry>,
    ) -> Result<Self> {
        let uuid = Uuid::new_v4();
        let sides = Vec::from_iter([
            Self::new_side(&options.side_1),
            Self::new_side(&options.side_2),
        ]);
        let battle = PublicCoreBattle::new(options, data, engine_options)?;
        let logs = SplitLogs::new(uuid, sides.len(), global_log_tx);

        let (choice_made_tx, _) = broadcast::channel(16);
        let (cancel_timers_tx, _) = broadcast::channel(16);

        let players = sides
            .iter()
            .flat_map(|side| side.players.iter().map(|player| player.id.clone()))
            .collect::<Vec<_>>();
        let timers = service_options.timers.to_state(&players);

        let metadata = BattleMetadata {
            creator: service_options.creator,
        };

        LiveBattle {
            uuid,
            battle,
            metadata,
            sides,
            error: None,
            logs,
            timers,
            choice_made_tx,
            cancel_timers_tx,
            current_timer_tasks: JoinSet::default(),
            proceed_tasks: JoinSet::default(),
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

    fn battle_status(&self) -> BattleStatus {
        BattleStatus {
            turn: self.battle.turn(),
        }
    }

    fn battle(&self) -> Battle {
        Battle {
            uuid: self.uuid,
            state: self.battle_state(),
            status: self.battle_status(),
            sides: self.sides.clone(),
            error: self.error.clone(),
            metadata: self.metadata.clone(),
        }
    }

    fn side_preview(side: &Side) -> SidePreview {
        SidePreview {
            players: side
                .players
                .iter()
                .map(|player| PlayerPreview {
                    id: player.id.clone(),
                    name: player.name.clone(),
                })
                .collect(),
        }
    }

    fn battle_preview(&self) -> BattlePreview {
        BattlePreview {
            uuid: self.uuid,
            sides: self
                .sides
                .iter()
                .map(|side| Self::side_preview(side))
                .collect(),
        }
    }

    fn log_for_side(&self, side: Option<usize>) -> &Log {
        side.and_then(|side| self.logs.side_log(side))
            .unwrap_or(self.logs.public_log())
    }

    fn update_team(&mut self, player: &str, team: TeamData) -> Result<()> {
        self.battle.update_team(player, team)?;
        self.update_player_state(player)
    }

    fn validate_player(&mut self, player: &str) -> Result<PlayerValidation> {
        match self.battle.validate_player(player) {
            Ok(()) => Ok(PlayerValidation::default()),
            Err(err) => match err.downcast::<ValidationError>() {
                Ok(err) => Ok(PlayerValidation {
                    problems: err.problems().map(|s| s.to_owned()).collect(),
                }),
                Err(err) => Err(err),
            },
        }
    }

    fn update_player_state(&mut self, player: &str) -> Result<()> {
        let state = if self
            .validate_player(&player)
            .is_ok_and(|validation| validation.problems.is_empty())
        {
            PlayerState::Ready
        } else {
            PlayerState::Waiting
        };
        self.player_mut_or_error(player)?.state = state;
        Ok(())
    }

    fn make_choice(&mut self, player: &str, choice: &str) -> Result<()> {
        // Ensure the player can make a move.
        if let Some((_, timer_state)) = self.timers.iter().find(|(timer_type, _)| {
            timer_type
                .player()
                .is_some_and(|timer_player| timer_player == player)
        }) && timer_state.remaining.is_zero()
        {
            return Err(Error::msg("you ran out of time"));
        }

        self.battle.set_player_choice(player, choice)?;
        self.choice_made_tx.send(player.to_owned()).ok();
        Ok(())
    }

    fn continue_battle(&mut self) -> Result<bool> {
        let continued = if self.battle.ready_to_continue()? {
            self.cancel_timers_tx.send(()).ok();
            self.battle.continue_battle()?;
            true
        } else {
            false
        };
        self.logs.append(self.battle.new_log_entries());
        Ok(continued)
    }

    fn injected_log_entry<S>(entry: S) -> String
    where
        S: Display,
    {
        format!("-battlerservice:{entry}")
    }

    fn timer_log(
        timer_type: &TimerType,
        remaining: Duration,
        timer_log_type: Option<TimerLogType>,
    ) -> String {
        let timer_type = match timer_type {
            TimerType::Battle => "battle".to_owned(),
            TimerType::Player(player) => format!("player:{player}"),
            TimerType::Action(player) => format!("action:{player}"),
        };
        format!(
            "timer|{timer_type}{}|remainingsecs:{}",
            match timer_log_type {
                Some(TimerLogType::Warning) => "|warning",
                Some(TimerLogType::Done) => "|done",
                None => "",
            },
            remaining.as_secs()
        )
    }

    fn inject_log_entries<I, S>(&mut self, entries: I)
    where
        I: IntoIterator<Item = S>,
        S: Display,
    {
        self.logs.append(
            entries
                .into_iter()
                .map(|entry| Self::injected_log_entry(entry)),
        );
    }

    async fn handle_timer_finished(&mut self, timer_type: &TimerType) -> Result<()> {
        // The timer finished, but the player it corresponds to made an action.
        //
        // Rather than overwrite their action, just forget the fact that the timer finished.
        //
        // For a player timer, the timer will fail immediately on the next request. For an action
        // timer, the timer resets anyway.
        if let Some(player) = timer_type.player()
            && self.battle.request_for_player(player)?.is_none()
        {
            return Ok(());
        }

        self.inject_log_entries([Self::timer_log(
            timer_type,
            Duration::ZERO,
            Some(TimerLogType::Done),
        )]);

        match timer_type {
            TimerType::Battle => self.battle.auto_end(),
            TimerType::Player(player) => self.battle.set_player_choice(player, "forfeit"),
            TimerType::Action(player) => self.battle.set_player_choice(player, "randomall"),
        }
    }
}

/// A wrapper around a [`LiveBattle`] for non-atomic operations.
///
/// Some tasks are spawned in the background, such as tasks for battle timers. Such tasks must have
/// reference to the battle, and we must ensure these tasks are joined when the battle is dropped.
/// This object manages such things.
struct LiveBattleManager<'d> {
    live_battle: Arc<Mutex<LiveBattle<'d>>>,
}

impl<'d> LiveBattleManager<'d> {
    fn new(battle: LiveBattle<'d>) -> Self {
        Self {
            live_battle: Arc::new(Mutex::new(battle)),
        }
    }

    async fn players(&self) -> Vec<String> {
        self.live_battle
            .lock()
            .await
            .players()
            .map(|s| s.to_owned())
            .collect()
    }

    async fn battle_state(&self) -> BattleState {
        self.live_battle.lock().await.battle_state()
    }

    async fn battle(&self) -> Battle {
        self.live_battle.lock().await.battle()
    }

    async fn battle_preview(&self) -> BattlePreview {
        self.live_battle.lock().await.battle_preview()
    }

    async fn log_for_side<F, R>(&self, side: Option<usize>, f: F) -> R
    where
        F: Fn(&Log) -> R,
    {
        f(self.live_battle.lock().await.log_for_side(side))
    }

    async fn update_team(&self, player: &str, team: TeamData) -> Result<()> {
        let mut live_battle = self.live_battle.lock().await;
        live_battle.update_team(player, team)?;

        // Inject a log entry so that clients can refresh player states.
        live_battle.inject_log_entries([format!("teamupdate|player:{player}")]);

        Ok(())
    }

    async fn validate_player(&self, player: &str) -> Result<PlayerValidation> {
        self.live_battle.lock().await.validate_player(player)
    }

    async fn player_data(&self, player: &str) -> Result<PlayerBattleData> {
        self.live_battle.lock().await.battle.player_data(player)
    }

    async fn request_for_player(&self, player: &str) -> Result<Option<Request>> {
        self.live_battle
            .lock()
            .await
            .battle
            .request_for_player(player)
    }

    async fn start(&self) -> Result<()> {
        {
            let mut live_battle = self.live_battle.lock().await;
            live_battle.battle.start()?;
            live_battle.inject_log_entries(["started"]);
        }
        Self::proceed(self.live_battle.clone()).await;
        Ok(())
    }

    async fn make_choice(&self, player: &str, choice: &str) -> Result<()> {
        self.live_battle.lock().await.make_choice(player, choice)?;
        Self::proceed(self.live_battle.clone()).await;
        Ok(())
    }

    async fn proceed(battle: Arc<Mutex<LiveBattle<'d>>>) {
        // SAFETY: self.proceed_tasks is awaited when this object is dropped. Additionally, we
        // ensure this object is dropped when the service is dropped, so no tasks should linger
        // beyond the underlying lifetime.
        let live_battle = unsafe {
            std::mem::transmute::<Arc<Mutex<LiveBattle<'d>>>, Arc<Mutex<LiveBattle<'static>>>>(
                battle.clone(),
            )
        };
        battle
            .lock()
            .await
            .proceed_tasks
            .spawn(LiveBattleManager::proceed_detached(Arc::downgrade(
                &live_battle,
            )));
    }

    async fn proceed_detached(battle: Weak<Mutex<LiveBattle<'d>>>) {
        let battle = match battle.upgrade() {
            Some(battle) => battle,
            None => return,
        };
        if let Err(err) = Self::proceed_detached_internal(battle.clone()).await {
            battle.lock().await.error = Some(format!("{err:#}"));
        }
    }

    async fn proceed_detached_internal(battle: Arc<Mutex<LiveBattle<'d>>>) -> Result<()> {
        let (continued, ended) = {
            let mut battle = battle.lock().await;
            battle.error = None;
            (battle.continue_battle()?, battle.battle.ended())
        };

        if continued && !ended {
            Self::resume_timers(battle).await?;
        }

        Ok(())
    }

    async fn join_all_timer_tasks(battle: &Arc<Mutex<LiveBattle<'d>>>) {
        // Soft cancellation; we want timer tasks to finish so that their state is updated.
        battle.lock().await.cancel_timers_tx.send(()).ok();

        // Join all timer tasks when finished.
        let mut current_timer_tasks = JoinSet::default();
        std::mem::swap(
            &mut current_timer_tasks,
            &mut battle.lock().await.current_timer_tasks,
        );
        while let Some(_) = current_timer_tasks.join_next().await {}
    }

    // We must manually add the `Send` trait because this async function can be recursive: when a
    // timer finished, the battle proceeds and restarts timers.
    fn resume_timers(
        battle: Arc<Mutex<LiveBattle<'d>>>,
    ) -> impl Future<Output = Result<()>> + Send {
        async move {
            // CRITICAL: Ensure all previous timer tasks finished.
            //
            // If we skip this step, a timer task may not have updated the current state of a
            // timer, resulting in the timer not progressing.
            Self::join_all_timer_tasks(&battle).await;

            // Get all timers, after all previous timer tasks finished.
            let timers = {
                let mut battle = battle.lock().await;

                for (timer_type, timer_state) in &mut battle.timers {
                    if timer_type.reset_on_resume() {
                        timer_state.remaining = timer_state.total;
                    }
                }

                // Filter timers that should be active.
                let mut timers = BTreeSet::default();
                for timer_type in battle.timers.keys() {
                    let active = match timer_type {
                        TimerType::Battle => true,
                        TimerType::Player(player) | TimerType::Action(player) => {
                            battle.battle.request_for_player(player)?.is_some()
                        }
                    };
                    if active {
                        timers.insert(timer_type.clone());
                    }
                }

                let timer_logs = timers
                    .iter()
                    .map(|timer_type| {
                        LiveBattle::timer_log(
                            timer_type,
                            // SAFETY: All keys in `timers` are generated from existing values in
                            // `battle.timers``.
                            battle.timers.get(timer_type).unwrap().remaining,
                            None,
                        )
                    })
                    .collect::<Vec<_>>();
                battle.inject_log_entries(timer_logs);

                timers
            };

            // Subscribe to channels before spawning tasks. This ensures that tasks won't miss
            // messages that are sent before they begin polling.
            let (choice_made_rx, cancel_timers_rx) = {
                let battle = battle.lock().await;
                (
                    battle.choice_made_tx.subscribe(),
                    battle.cancel_timers_tx.subscribe(),
                )
            };

            // Spawn new timer tasks.
            let tasks = {
                // SAFETY: All tasks spawned here are joined when the object is dropped, and before
                // a new JoinSet is created.
                let battle = unsafe {
                    std::mem::transmute::<Arc<Mutex<LiveBattle<'d>>>, Arc<Mutex<LiveBattle<'static>>>>(
                        battle.clone(),
                    )
                };
                let mut tasks = JoinSet::default();
                for timer_type in timers {
                    tasks.spawn(LiveBattleManager::run_timer(
                        battle.clone(),
                        timer_type,
                        choice_made_rx.resubscribe(),
                        cancel_timers_rx.resubscribe(),
                    ));
                }
                tasks
            };

            {
                let mut battle = battle.lock().await;
                battle.current_timer_tasks = tasks;
            }
            Ok(())
        }
    }

    async fn run_timer(
        battle: Arc<Mutex<LiveBattle<'d>>>,
        timer_type: TimerType,
        choice_made_rx: broadcast::Receiver<String>,
        cancel_timers_rx: broadcast::Receiver<()>,
    ) {
        let state = match Self::run_timer_internal(
            battle.clone(),
            &timer_type,
            choice_made_rx,
            cancel_timers_rx,
        )
        .await
        {
            Ok(state) => state,
            Err(_) => return,
        };

        let finished = state
            .as_ref()
            .is_some_and(|state| state.remaining.is_zero());

        {
            // Update the timer state and handle the finished timer, all while the battle is locked.
            //
            // Locking the mutex for both of these actions ensures there is no data race between the
            // timer finishing and the player taking an action. The player CANNOT take an action
            // while we are handling a finished timer. If they send their action before, then the
            // finished timer is ignored. If they send their action after, then it fails due to the
            // finished timer.
            let mut battle = battle.lock().await;
            match state {
                Some(state) => battle.timers.insert(timer_type.clone(), state),
                None => battle.timers.remove(&timer_type),
            };

            if finished {
                battle.handle_timer_finished(&timer_type).await.ok();
            }
        }

        // The timer finishing may have allowed the battle to proceed.
        Self::proceed(battle).await;
    }

    async fn run_timer_internal(
        battle: Arc<Mutex<LiveBattle<'d>>>,
        timer_type: &TimerType,
        mut choice_made_rx: broadcast::Receiver<String>,
        mut cancel_timers_rx: broadcast::Receiver<()>,
    ) -> Result<Option<TimerState>> {
        // Read the current state of the timer.
        let state = match battle.lock().await.timers.get(&timer_type).cloned() {
            Some(state) => state,
            None => return Ok(None),
        };

        let mut remaining = state.remaining;
        let mut now = Instant::now();
        let deadline = now + remaining;

        let recalculate_remaining = |now: &mut Instant, remaining: &mut Duration| {
            *now = Instant::now();
            *remaining = if deadline > *now {
                let remaining = deadline - *now;
                // Smallest granularity allowed is 1 second.
                if remaining < Duration::from_secs(1) {
                    Duration::ZERO
                } else {
                    remaining
                }
            } else {
                Duration::ZERO
            };
        };

        loop {
            recalculate_remaining(&mut now, &mut remaining);

            // Timer finished.
            if remaining.is_zero() {
                break;
            }

            // Calculate when the next warning should be.
            let next_warning = state
                .warnings
                .iter()
                .rev()
                .filter(|time| remaining >= **time)
                .next()
                .cloned();
            let next_warning_future: Pin<Box<dyn Future<Output = ()> + Send>> = match next_warning {
                Some(time) => Box::pin(tokio::time::sleep(remaining - time)),
                None => Box::pin(futures_util::future::pending()),
            };

            tokio::select! {
                _ = tokio::time::sleep(remaining) => {
                    // Timer finished.
                    break;
                }
                _ = next_warning_future => {
                    // Issue a warning.
                    let mut battle = battle.lock().await;
                    battle.inject_log_entries([LiveBattle::timer_log(
                        timer_type,
                        next_warning.unwrap_or_default(),
                        Some(TimerLogType::Warning),
                    )]);
                }
                choice_made = choice_made_rx.recv() => {
                    // A choice was made for the player this timer corresponds to.
                    let player = choice_made?;
                    if timer_type.player().is_some_and(|timer_player| timer_player == player) {
                        break;
                    }
                }
                result = cancel_timers_rx.recv() => {
                    // The timer was canceled, likely because the battle continued.
                    result?;
                    break;
                }
            }
        }

        recalculate_remaining(&mut now, &mut remaining);

        // Save the timer state, even if the duration is zero.
        //
        // This avoids the scenario where the timer ends immediately after the player makes their
        // choice. We want the timer to end immediately the next time it starts, which requires the
        // state to be saved.
        Ok(Some(TimerState {
            total: state.total,
            remaining,
            warnings: state.warnings,
        }))
    }

    async fn shutdown(&mut self) {
        let mut battle = self.live_battle.lock().await;
        battle.proceed_tasks.shutdown().await;
        battle.current_timer_tasks.shutdown().await;
    }
}

impl<'d> Drop for LiveBattleManager<'d> {
    fn drop(&mut self) {
        // Ensure we stop using the battle in any active task.
        tokio::task::block_in_place(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                self.shutdown().await;

                // SAFETY: Breaking this invariant can lead to undefined behavior, so it is better
                // to just crash now.
                let strong_count = Arc::strong_count(&self.live_battle);
                assert_eq!(
                    strong_count, 1,
                    "live battle has {strong_count} references, even after being shut down"
                );
            });
        });
    }
}

/// Service for managing multiple battles on the [`battler`] battle engine.
pub struct BattlerService<'d> {
    data: &'d dyn DataStore,

    // SAFETY: Arc is used for the simplicity of looking up battles internally. When we drop this
    // object, we forcibly wait for all references to be dropped.
    battles: Mutex<BTreeMap<Uuid, Arc<LiveBattleManager<'d>>>>,
    battles_by_player: Mutex<HashMap<String, BTreeSet<Uuid>>>,

    global_log_tx: mpsc::UnboundedSender<GlobalLogEntry>,
    global_log_rx: Option<mpsc::UnboundedReceiver<GlobalLogEntry>>,
}

impl<'d> BattlerService<'d> {
    /// Creates a new battle service.
    pub fn new(data: &'d dyn DataStore) -> Self {
        let (global_log_tx, global_log_rx) = mpsc::unbounded_channel();
        Self {
            data,
            battles: Mutex::new(BTreeMap::default()),
            battles_by_player: Mutex::new(HashMap::default()),
            global_log_tx,
            global_log_rx: Some(global_log_rx),
        }
    }

    /// Takes the global log receiver.
    ///
    /// All log entries for all battles will be sent over this channel for consumption.
    ///
    /// This method can only be called once. Subsequent calls will return [`None`],
    pub fn take_global_log_rx(&mut self) -> Option<mpsc::UnboundedReceiver<GlobalLogEntry>> {
        self.global_log_rx.take()
    }

    async fn find_battle(&self, uuid: Uuid) -> Option<Arc<LiveBattleManager<'d>>> {
        self.battles.lock().await.get(&uuid).cloned()
    }

    async fn find_battle_or_error(&self, uuid: Uuid) -> Result<Arc<LiveBattleManager<'d>>> {
        self.find_battle(uuid)
            .await
            .ok_or_else(|| Error::msg("battle does not exist"))
    }

    /// Generates the status of an existing battle.
    pub async fn battle(&self, battle: Uuid) -> Result<Battle> {
        let battle = self.find_battle_or_error(battle).await?;
        Ok(battle.battle().await)
    }

    /// Creates a new battle.
    pub async fn create(
        &self,
        options: CoreBattleOptions,
        mut engine_options: CoreBattleEngineOptions,
        service_options: BattleServiceOptions,
    ) -> Result<Battle> {
        // Do not auto continue, so that we can capture any errors in our own task.
        engine_options.auto_continue = false;

        let battle = LiveBattle::new(
            options,
            engine_options,
            service_options,
            self.data,
            self.global_log_tx.clone(),
        )?;
        let uuid = battle.uuid;
        let players = battle.players().map(|s| s.to_owned()).collect::<Vec<_>>();
        let battle = LiveBattleManager::new(battle);
        self.battles.lock().await.insert(uuid, Arc::new(battle));

        for player in players {
            self.battles_by_player
                .lock()
                .await
                .entry(player)
                .or_default()
                .insert(uuid);
        }

        self.battle(uuid).await
    }

    /// Updates a player's team for a battle.
    pub async fn update_team(&self, battle: Uuid, player: &str, team: TeamData) -> Result<()> {
        let battle = self.find_battle_or_error(battle).await?;
        battle.update_team(player, team).await
    }

    /// Validates a player in a battle.
    pub async fn validate_player(&self, battle: Uuid, player: &str) -> Result<PlayerValidation> {
        let battle = self.find_battle_or_error(battle).await?;
        battle.validate_player(player).await
    }

    /// Starts a battle.
    pub async fn start(&self, battle: Uuid) -> Result<()> {
        let battle = self.find_battle_or_error(battle).await?;
        battle.start().await
    }

    /// Returns the player data for a player in a battle.
    pub async fn player_data(&self, battle: Uuid, player: &str) -> Result<PlayerBattleData> {
        let battle = self.find_battle_or_error(battle).await?;
        battle.player_data(player).await
    }

    /// Returns the current request for a player in a battle.
    pub async fn request(&self, battle: Uuid, player: &str) -> Result<Option<Request>> {
        let battle = self.find_battle_or_error(battle).await?;
        battle.request_for_player(player).await
    }

    /// Sets a player's choice in a battle.
    pub async fn make_choice(&self, battle: Uuid, player: &str, choice: &str) -> Result<()> {
        let battle = self.find_battle_or_error(battle).await?;
        battle.make_choice(player, choice).await
    }

    /// Reads the full battle log for the side.
    ///
    /// If `side` is `None`, the public log is used.
    pub async fn full_log(&self, battle: Uuid, side: Option<usize>) -> Result<Vec<String>> {
        let battle = self.find_battle_or_error(battle).await?;
        Ok(battle
            .log_for_side(side, |log| log.entries().map(|s| s.to_owned()).collect())
            .await)
    }

    /// Reads the last battle log entry for the side.
    ///
    /// If `side` is `None`, the public log is used.
    pub async fn last_log_entry(
        &self,
        battle: Uuid,
        side: Option<usize>,
    ) -> Result<Option<(usize, String)>> {
        let battle = self.find_battle_or_error(battle).await?;
        Ok(battle
            .log_for_side(side, |log| {
                let (i, entry) = log.entries().enumerate().rev().next()?;
                Some((i, entry.to_owned()))
            })
            .await)
    }

    /// Subscribes to battle log updates.
    ///
    /// If `side` is `None`, the public log is used.
    pub async fn subscribe(
        &self,
        battle: Uuid,
        side: Option<usize>,
    ) -> Result<broadcast::Receiver<LogEntry>> {
        let battle = self.find_battle_or_error(battle).await?;
        Ok(battle.log_for_side(side, |log| log.subscribe()).await)
    }

    async fn wait_for_live_battle_manager_to_be_safe_to_drop(
        mut battle: Arc<LiveBattleManager<'d>>,
    ) -> LiveBattleManager<'d> {
        // SAFETY: The LiveBattleManager is held within an Arc in this service object. Multiple
        // tasks may be using the battle still. At this point, we deleted the battle from the
        // internal map, so no NEW references should be taken. Thus, we expect this reference count
        // to eventually get to 1.
        //
        // Waiting for this condition is CRITICAL to object safety. Spawned tasks within the
        // LiveBattleManager take reference to the Arc<Mutex<LiveBattle>> within. The
        // LiveBattleManager's Drop implementation joins all spawned tasks and ensures there are no
        // more references. This function effectively waits for the LiveBattleManager to be safe to
        // drop, which ensures the joining logic is triggered NOW.
        loop {
            match Arc::try_unwrap(battle) {
                Ok(battle) => return battle,
                Err(err) => battle = err,
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }

    /// Deletes a battle.
    pub async fn delete(&self, battle: Uuid) -> Result<()> {
        {
            let battle = match self.find_battle_or_error(battle).await {
                Ok(battle) => battle,
                Err(_) => return Ok(()),
            };
            if battle.battle_state().await == BattleState::Active {
                return Err(Error::msg("cannot delete an ongoing battle"));
            }
        }
        let uuid = battle;
        let battle = self.battles.lock().await.remove(&battle);

        if let Some(battle) = battle {
            let battle = Self::wait_for_live_battle_manager_to_be_safe_to_drop(battle).await;

            let players = battle.players().await;
            for player in players {
                match self.battles_by_player.lock().await.entry(player) {
                    std::collections::hash_map::Entry::Occupied(mut entry) => {
                        entry.get_mut().remove(&uuid);
                        if entry.get().is_empty() {
                            entry.remove_entry();
                        }
                    }
                    _ => (),
                }
            }

            // Purely for safety, ensure Drop is called here, which ensures all spawned tasks are
            // joined.
            drop(battle);
        }

        Ok(())
    }

    /// Lists battles.
    pub async fn battles(&self, count: usize, offset: usize) -> Vec<BattlePreview> {
        let count = count.min(100);
        let battles = self.battles.lock().await;
        let mut previews = Vec::with_capacity(count);
        for (_, battle) in battles.iter().skip(offset).take(count) {
            previews.push(battle.battle_preview().await);
        }
        previews
    }

    /// Looks up battles for a player.
    pub async fn battles_for_player(
        &self,
        player: &str,
        count: usize,
        offset: usize,
    ) -> Vec<BattlePreview> {
        let count = count.min(100);
        let battles = self.battles_by_player.lock().await;
        let battles = match battles.get(player) {
            Some(battles) => battles,
            None => return Vec::default(),
        };
        let uuids = battles
            .iter()
            .skip(offset)
            .take(count)
            .cloned()
            .collect::<Vec<_>>();
        let mut previews = Vec::with_capacity(count);
        let battles = self.battles.lock().await;
        for battle in uuids {
            if let Some(battle) = battles.get(&battle) {
                previews.push(battle.battle_preview().await);
            }
        }
        previews
    }
}

impl<'d> Drop for BattlerService<'d> {
    fn drop(&mut self) {
        // Block on every battle being immediately dropped, so that we do not leak any tasks.
        tokio::task::block_in_place(move || {
            tokio::runtime::Runtime::new().unwrap().block_on(async {
                let mut battles = BTreeMap::default();
                std::mem::swap(&mut battles, &mut *self.battles.lock().await);
                for (_, battle) in battles {
                    Self::wait_for_live_battle_manager_to_be_safe_to_drop(battle).await;
                }
            });
        });
    }
}

#[cfg(test)]
mod battler_service_test {
    use std::{
        collections::BTreeSet,
        time::Duration,
    };

    use ahash::HashSet;
    use anyhow::Error;
    use battler::{
        BagData,
        BattleType,
        CoreBattleEngineOptions,
        CoreBattleEngineSpeedSortTieResolution,
        CoreBattleOptions,
        FieldData,
        FormatData,
        Gender,
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
        battle::PlayerDex,
    };
    use battler_test_utils::static_local_data_store;
    use itertools::Itertools;
    use tokio::{
        sync::broadcast,
        time::Instant,
    };

    use super::BattlerService;
    use crate::{
        BattlePreview,
        BattleServiceOptions,
        BattleState,
        Player,
        PlayerPreview,
        PlayerState,
        Side,
        SidePreview,
        Timer,
        Timers,
        log::LogEntry,
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
            ball: Some("Poké Ball".to_owned()),
            hidden_power_type: None,
            different_original_trainer: false,
            dynamax_level: 0,
            gigantamax_factor: false,
            tera_type: None,
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
                    Vec::from_iter(["Tackle".to_owned(), "Growl".to_owned()]),
                    level,
                ),
                mon(
                    "Charmander".to_owned(),
                    "Charmander".to_owned(),
                    "Blaze".to_owned(),
                    Vec::from_iter(["Scratch".to_owned(), "Growl".to_owned()]),
                    level,
                ),
                mon(
                    "Squirtle".to_owned(),
                    "Squirtle".to_owned(),
                    "Torrent".to_owned(),
                    Vec::from_iter(["Tackle".to_owned(), "Tail Whip".to_owned()]),
                    level,
                ),
            ]),
            bag: BagData::default(),
        }
    }

    fn core_battle_options(battle_type: BattleType, team: TeamData) -> CoreBattleOptions {
        CoreBattleOptions {
            seed: Some(0),
            format: FormatData {
                battle_type: battle_type,
                rules: HashSet::from_iter([Rule::value_name("Item Clause")]),
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
                    dex: PlayerDex::default(),
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
                    dex: PlayerDex::default(),
                }]),
            },
        }
    }

    async fn read_all_entries_from_log_rx_stopping_at(
        log_rx: &mut broadcast::Receiver<LogEntry>,
        stop_at: &str,
    ) -> Vec<String> {
        let mut entries = Vec::new();
        while let Ok(entry) = log_rx.recv().await
            && entry.content != stop_at
        {
            entries.push(entry.content);
        }
        entries
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn creates_battle_and_players_in_waiting_state() {
        let battler_service = BattlerService::new(static_local_data_store());
        let battle = battler_service
            .create(
                core_battle_options(BattleType::Singles, TeamData::default()),
                CoreBattleEngineOptions::default(),
                BattleServiceOptions::default(),
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
        let battler_service = BattlerService::new(static_local_data_store());
        let battle = battler_service
            .create(
                core_battle_options(BattleType::Singles, TeamData::default()),
                CoreBattleEngineOptions::default(),
                BattleServiceOptions::default(),
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
        let battler_service = BattlerService::new(static_local_data_store());
        let battle = battler_service
            .create(
                core_battle_options(BattleType::Singles, TeamData::default()),
                CoreBattleEngineOptions::default(),
                BattleServiceOptions::default(),
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
        let battler_service = BattlerService::new(static_local_data_store());
        let battle = battler_service
            .create(
                core_battle_options(BattleType::Singles, team(5)),
                CoreBattleEngineOptions::default(),
                BattleServiceOptions::default(),
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
        let battler_service = BattlerService::new(static_local_data_store());
        let battle = battler_service
            .create(
                core_battle_options(BattleType::Singles, team(5)),
                CoreBattleEngineOptions::default(),
                BattleServiceOptions::default(),
            )
            .await
            .unwrap();

        assert_matches::assert_matches!(battler_service.start(battle.uuid).await, Ok(()));

        // Wait for battle to start.
        let mut public_log_rx = battler_service.subscribe(battle.uuid, None).await.unwrap();
        assert_matches::assert_matches!(public_log_rx.recv().await, Ok(_));

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
    async fn plays_battle_and_finishes_and_deletes() {
        let battler_service = BattlerService::new(static_local_data_store());
        let battle = battler_service
            .create(
                core_battle_options(BattleType::Singles, team(5)),
                CoreBattleEngineOptions::default(),
                BattleServiceOptions::default(),
            )
            .await
            .unwrap();

        assert_matches::assert_matches!(battler_service.start(battle.uuid).await, Ok(()));

        // Wait for battle to start.
        let mut public_log_rx = battler_service.subscribe(battle.uuid, None).await.unwrap();
        assert_matches::assert_matches!(public_log_rx.recv().await, Ok(_));

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

        read_all_entries_from_log_rx_stopping_at(&mut public_log_rx, "turn|turn:2").await;

        assert_matches::assert_matches!(battler_service.delete(battle.uuid).await, Err(err) => {
            assert_eq!(err.to_string(), "cannot delete an ongoing battle");
        });

        assert_matches::assert_matches!(
            battler_service
                .make_choice(battle.uuid, "player-1", "move 0")
                .await,
            Ok(())
        );
        assert_matches::assert_matches!(
            battler_service
                .make_choice(battle.uuid, "player-2", "forfeit")
                .await,
            Ok(())
        );

        // Wait for battle to end.
        read_all_entries_from_log_rx_stopping_at(&mut public_log_rx, "win|side:0").await;

        assert_matches::assert_matches!(battler_service.battle(battle.uuid).await, Ok(battle) => {
            assert_eq!(battle.state, BattleState::Finished);
        });

        assert_matches::assert_matches!(battler_service.delete(battle.uuid).await, Ok(()));

        pretty_assertions::assert_eq!(battler_service.battles(usize::MAX, 0).await, []);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn returns_filtered_logs_by_side() {
        let battler_service = BattlerService::new(static_local_data_store());
        let battle = battler_service
            .create(
                core_battle_options(BattleType::Singles, team(5)),
                CoreBattleEngineOptions {
                    speed_sort_tie_resolution: CoreBattleEngineSpeedSortTieResolution::Keep,
                    ..Default::default()
                },
                BattleServiceOptions::default(),
            )
            .await
            .unwrap();

        assert_matches::assert_matches!(battler_service.start(battle.uuid).await, Ok(()));

        // Wait for battle to start.
        let mut public_log_rx = battler_service.subscribe(battle.uuid, None).await.unwrap();
        assert_matches::assert_matches!(public_log_rx.recv().await, Ok(_));

        // Read all logs from the battle starting; we only care to verify the first turn.
        while let Ok(_) = public_log_rx.try_recv() {}

        let mut side_1_log_rx = battler_service
            .subscribe(battle.uuid, Some(0))
            .await
            .unwrap();
        let mut side_2_log_rx = battler_service
            .subscribe(battle.uuid, Some(1))
            .await
            .unwrap();

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
            read_all_entries_from_log_rx_stopping_at(&mut side_1_log_rx, "turn|turn:2").await[1..],
            [
                "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Bulbasaur,player-2,1",
                "damage|mon:Bulbasaur,player-2,1|health:79/100",
                "move|mon:Bulbasaur,player-2,1|name:Tackle|target:Bulbasaur,player-1,1",
                "damage|mon:Bulbasaur,player-1,1|health:15/19",
                "residual",
            ],
        );
        pretty_assertions::assert_eq!(
            read_all_entries_from_log_rx_stopping_at(&mut side_2_log_rx, "turn|turn:2").await[1..],
            [
                "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Bulbasaur,player-2,1",
                "damage|mon:Bulbasaur,player-2,1|health:15/19",
                "move|mon:Bulbasaur,player-2,1|name:Tackle|target:Bulbasaur,player-1,1",
                "damage|mon:Bulbasaur,player-1,1|health:79/100",
                "residual",
            ],
        );
        pretty_assertions::assert_eq!(
            read_all_entries_from_log_rx_stopping_at(&mut public_log_rx, "turn|turn:2").await[1..],
            [
                "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Bulbasaur,player-2,1",
                "damage|mon:Bulbasaur,player-2,1|health:79/100",
                "move|mon:Bulbasaur,player-2,1|name:Tackle|target:Bulbasaur,player-1,1",
                "damage|mon:Bulbasaur,player-1,1|health:79/100",
                "residual",
            ],
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn lists_battles_in_uuid_order() {
        let battler_service = BattlerService::new(static_local_data_store());
        let mut battles = Vec::new();
        battles.push(
            battler_service
                .create(
                    core_battle_options(BattleType::Singles, team(5)),
                    CoreBattleEngineOptions::default(),
                    BattleServiceOptions::default(),
                )
                .await
                .unwrap()
                .uuid,
        );
        battles.push(
            battler_service
                .create(
                    core_battle_options(BattleType::Singles, team(5)),
                    CoreBattleEngineOptions::default(),
                    BattleServiceOptions::default(),
                )
                .await
                .unwrap()
                .uuid,
        );
        battles.push(
            battler_service
                .create(
                    core_battle_options(BattleType::Singles, team(5)),
                    CoreBattleEngineOptions::default(),
                    BattleServiceOptions::default(),
                )
                .await
                .unwrap()
                .uuid,
        );

        battles.sort();

        pretty_assertions::assert_eq!(
            battler_service.battles(2, 0).await,
            Vec::from_iter([
                BattlePreview {
                    uuid: battles[0],
                    sides: Vec::from_iter([
                        SidePreview {
                            players: Vec::from_iter([PlayerPreview {
                                id: "player-1".to_owned(),
                                name: "Player 1".to_owned(),
                            }]),
                        },
                        SidePreview {
                            players: Vec::from_iter([PlayerPreview {
                                id: "player-2".to_owned(),
                                name: "Player 2".to_owned(),
                            }]),
                        }
                    ]),
                },
                BattlePreview {
                    uuid: battles[1],
                    sides: Vec::from_iter([
                        SidePreview {
                            players: Vec::from_iter([PlayerPreview {
                                id: "player-1".to_owned(),
                                name: "Player 1".to_owned(),
                            }]),
                        },
                        SidePreview {
                            players: Vec::from_iter([PlayerPreview {
                                id: "player-2".to_owned(),
                                name: "Player 2".to_owned(),
                            }]),
                        }
                    ]),
                }
            ])
        );

        pretty_assertions::assert_eq!(
            battler_service.battles(2, 2).await,
            Vec::from_iter([BattlePreview {
                uuid: battles[2],
                sides: Vec::from_iter([
                    SidePreview {
                        players: Vec::from_iter([PlayerPreview {
                            id: "player-1".to_owned(),
                            name: "Player 1".to_owned(),
                        }]),
                    },
                    SidePreview {
                        players: Vec::from_iter([PlayerPreview {
                            id: "player-2".to_owned(),
                            name: "Player 2".to_owned(),
                        }]),
                    }
                ]),
            }])
        );

        pretty_assertions::assert_eq!(battler_service.battles(2, 3).await, Vec::default());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn lists_battles_for_player_in_uuid_order() {
        let battler_service = BattlerService::new(static_local_data_store());
        let mut battles = Vec::new();
        battles.push(
            battler_service
                .create(
                    core_battle_options(BattleType::Singles, team(5)),
                    CoreBattleEngineOptions::default(),
                    BattleServiceOptions::default(),
                )
                .await
                .unwrap()
                .uuid,
        );
        battles.push(
            battler_service
                .create(
                    core_battle_options(BattleType::Singles, team(5)),
                    CoreBattleEngineOptions::default(),
                    BattleServiceOptions::default(),
                )
                .await
                .unwrap()
                .uuid,
        );
        battles.push(
            battler_service
                .create(
                    core_battle_options(BattleType::Singles, team(5)),
                    CoreBattleEngineOptions::default(),
                    BattleServiceOptions::default(),
                )
                .await
                .unwrap()
                .uuid,
        );

        battles.sort();

        pretty_assertions::assert_eq!(
            battler_service.battles_for_player("player-2", 2, 0).await,
            Vec::from_iter([
                BattlePreview {
                    uuid: battles[0],
                    sides: Vec::from_iter([
                        SidePreview {
                            players: Vec::from_iter([PlayerPreview {
                                id: "player-1".to_owned(),
                                name: "Player 1".to_owned(),
                            }]),
                        },
                        SidePreview {
                            players: Vec::from_iter([PlayerPreview {
                                id: "player-2".to_owned(),
                                name: "Player 2".to_owned(),
                            }]),
                        }
                    ]),
                },
                BattlePreview {
                    uuid: battles[1],
                    sides: Vec::from_iter([
                        SidePreview {
                            players: Vec::from_iter([PlayerPreview {
                                id: "player-1".to_owned(),
                                name: "Player 1".to_owned(),
                            }]),
                        },
                        SidePreview {
                            players: Vec::from_iter([PlayerPreview {
                                id: "player-2".to_owned(),
                                name: "Player 2".to_owned(),
                            }]),
                        }
                    ]),
                }
            ])
        );

        pretty_assertions::assert_eq!(
            battler_service.battles_for_player("player-2", 2, 2).await,
            Vec::from_iter([BattlePreview {
                uuid: battles[2],
                sides: Vec::from_iter([
                    SidePreview {
                        players: Vec::from_iter([PlayerPreview {
                            id: "player-1".to_owned(),
                            name: "Player 1".to_owned(),
                        }]),
                    },
                    SidePreview {
                        players: Vec::from_iter([PlayerPreview {
                            id: "player-2".to_owned(),
                            name: "Player 2".to_owned(),
                        }]),
                    }
                ]),
            }])
        );

        pretty_assertions::assert_eq!(
            battler_service.battles_for_player("player-2", 2, 3).await,
            Vec::default()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn returns_empty_list_for_player_with_no_battles() {
        let battler_service = BattlerService::new(static_local_data_store());
        battler_service
            .create(
                core_battle_options(BattleType::Singles, team(5)),
                CoreBattleEngineOptions::default(),
                BattleServiceOptions::default(),
            )
            .await
            .unwrap();

        pretty_assertions::assert_eq!(
            battler_service.battles_for_player("player-3", 2, 0).await,
            Vec::default()
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn auto_ends_battle_on_battle_timer() {
        let battler_service = BattlerService::new(static_local_data_store());
        let battle = battler_service
            .create(
                core_battle_options(BattleType::Singles, team(5)),
                CoreBattleEngineOptions {
                    speed_sort_tie_resolution: CoreBattleEngineSpeedSortTieResolution::Keep,
                    ..Default::default()
                },
                BattleServiceOptions {
                    timers: Timers {
                        battle: Some(Timer {
                            secs: 5,
                            warnings: BTreeSet::from_iter([4, 2, 1]),
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_matches::assert_matches!(battler_service.start(battle.uuid).await, Ok(()));

        // Wait for the battle to automatically end.
        let deadline = Instant::now() + Duration::from_secs(10);
        assert_matches::assert_matches!(
            tokio::time::timeout_at(
                deadline,
                (async || {
                    while battler_service.battle(battle.uuid).await?.state != BattleState::Finished
                    {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                    Ok::<_, Error>(())
                })(),
            )
            .await,
            Ok(_)
        );

        assert_matches::assert_matches!(battler_service.full_log(battle.uuid, None).await, Ok(log) => {
            pretty_assertions::assert_eq!(
                log[(log.len() - 7)..],
                [
                    "turn|turn:1",
                    "-battlerservice:timer|battle|remainingsecs:5",
                    "-battlerservice:timer|battle|warning|remainingsecs:4",
                    "-battlerservice:timer|battle|warning|remainingsecs:2",
                    "-battlerservice:timer|battle|warning|remainingsecs:1",
                    "-battlerservice:timer|battle|done|remainingsecs:0",
                    "tie",
                ]
            );
        });

        assert_matches::assert_matches!(
            battler_service
                .make_choice(battle.uuid, "player-1", "move 0")
                .await,
            Err(err) => {
                assert!(err.to_string().contains("the battle is over"), "{err:#}");
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn forfeits_on_player_timer() {
        let battler_service = BattlerService::new(static_local_data_store());
        let battle = battler_service
            .create(
                core_battle_options(BattleType::Singles, team(5)),
                CoreBattleEngineOptions {
                    speed_sort_tie_resolution: CoreBattleEngineSpeedSortTieResolution::Keep,
                    log_time: false,
                    ..Default::default()
                },
                BattleServiceOptions {
                    timers: Timers {
                        player: Some(Timer {
                            secs: 5,
                            warnings: BTreeSet::from_iter([1]),
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_matches::assert_matches!(battler_service.start(battle.uuid).await, Ok(()));

        // Wait for battle to start.
        let mut public_log_rx = battler_service.subscribe(battle.uuid, None).await.unwrap();
        assert_matches::assert_matches!(public_log_rx.recv().await, Ok(_));

        // Wait for timers to start.
        read_all_entries_from_log_rx_stopping_at(
            &mut public_log_rx,
            "-battlerservice:timer|player:player-1|remainingsecs:5",
        )
        .await;

        assert_matches::assert_matches!(
            battler_service
                .make_choice(battle.uuid, "player-1", "move 0")
                .await,
            Ok(())
        );

        // Wait for the battle to automatically end.
        let deadline = Instant::now() + Duration::from_secs(10);
        assert_matches::assert_matches!(
            tokio::time::timeout_at(
                deadline,
                (async || {
                    while battler_service.battle(battle.uuid).await?.state != BattleState::Finished
                    {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                    Ok::<_, Error>(())
                })(),
            )
            .await,
            Ok(_)
        );

        assert_matches::assert_matches!(battler_service.full_log(battle.uuid, None).await, Ok(log) => {
            pretty_assertions::assert_eq!(
                log[(log.len() - 9)..],
                [
                    "turn|turn:1",
                    "-battlerservice:timer|player:player-1|remainingsecs:5",
                    "-battlerservice:timer|player:player-2|remainingsecs:5",
                    "-battlerservice:timer|player:player-2|warning|remainingsecs:1",
                    "-battlerservice:timer|player:player-2|done|remainingsecs:0",
                    "continue",
                    "switchout|mon:Bulbasaur,player-2,1",
                    "forfeited|player:player-2",
                    "win|side:0",
                ]
            );
        });

        assert_matches::assert_matches!(
            battler_service
                .make_choice(battle.uuid, "player-1", "move 0")
                .await,
            Err(err) => {
                assert!(err.to_string().contains("the battle is over"), "{err:#}");
            }
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn selects_random_moves_on_action_timer() {
        let battler_service = BattlerService::new(static_local_data_store());
        let mut options = core_battle_options(BattleType::Doubles, team(5));
        options.seed = Some(444444);
        let battle = battler_service
            .create(
                options,
                CoreBattleEngineOptions {
                    speed_sort_tie_resolution: CoreBattleEngineSpeedSortTieResolution::Keep,
                    log_time: false,
                    ..Default::default()
                },
                BattleServiceOptions {
                    timers: Timers {
                        action: Some(Timer {
                            secs: 5,
                            warnings: BTreeSet::default(),
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_matches::assert_matches!(battler_service.start(battle.uuid).await, Ok(()));

        let mut public_log_rx = battler_service.subscribe(battle.uuid, None).await.unwrap();

        // Wait for turn 1.
        read_all_entries_from_log_rx_stopping_at(&mut public_log_rx, "turn|turn:1").await;

        assert_matches::assert_matches!(
            battler_service
                .make_choice(battle.uuid, "player-1", "move 0,1;move 0,1")
                .await,
            Ok(())
        );

        // Wait for the battle to continue.
        read_all_entries_from_log_rx_stopping_at(&mut public_log_rx, "continue").await;

        pretty_assertions::assert_eq!(
            read_all_entries_from_log_rx_stopping_at(&mut public_log_rx, "turn|turn:2").await,
            [
                "move|mon:Charmander,player-1,2|name:Scratch|target:Bulbasaur,player-2,1",
                "damage|mon:Bulbasaur,player-2,1|health:79/100",
                "move|mon:Charmander,player-2,2|name:Growl|spread:Bulbasaur,player-1,1;Charmander,player-1,2",
                "unboost|mon:Bulbasaur,player-1,1|stat:atk|by:1",
                "unboost|mon:Charmander,player-1,2|stat:atk|by:1",
                "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Bulbasaur,player-2,1",
                "damage|mon:Bulbasaur,player-2,1|health:64/100",
                "move|mon:Bulbasaur,player-2,1|name:Tackle|target:Bulbasaur,player-1,1",
                "damage|mon:Bulbasaur,player-1,1|health:79/100",
                "residual",
            ],
        );
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn only_activates_player_timer_if_request_is_active() {
        let battler_service = BattlerService::new(static_local_data_store());
        let mut options = core_battle_options(BattleType::Singles, team(5));
        options.side_1.players[0].team.members[0].level = 100;
        let battle = battler_service
            .create(
                options,
                CoreBattleEngineOptions {
                    speed_sort_tie_resolution: CoreBattleEngineSpeedSortTieResolution::Keep,
                    log_time: false,
                    ..Default::default()
                },
                BattleServiceOptions {
                    timers: Timers {
                        player: Some(Timer {
                            secs: 5,
                            warnings: BTreeSet::default(),
                        }),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        assert_matches::assert_matches!(battler_service.start(battle.uuid).await, Ok(()));

        let mut public_log_rx = battler_service.subscribe(battle.uuid, None).await.unwrap();

        // Wait for turn 1.
        read_all_entries_from_log_rx_stopping_at(&mut public_log_rx, "turn|turn:1").await;

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

        read_all_entries_from_log_rx_stopping_at(&mut public_log_rx, "continue").await;

        // Wait for the battle to automatically end.
        let deadline = Instant::now() + Duration::from_secs(10);
        assert_matches::assert_matches!(
            tokio::time::timeout_at(
                deadline,
                (async || {
                    while battler_service.battle(battle.uuid).await?.state != BattleState::Finished
                    {
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                    Ok::<_, Error>(())
                })(),
            )
            .await,
            Ok(_)
        );

        assert_matches::assert_matches!(battler_service.full_log(battle.uuid, None).await, Ok(log) => {
            pretty_assertions::assert_eq!(
                log[(log.len() - 10)..],
                [
                    "continue",
                    "move|mon:Bulbasaur,player-1,1|name:Tackle|target:Bulbasaur,player-2,1",
                    "damage|mon:Bulbasaur,player-2,1|health:0",
                    "faint|mon:Bulbasaur,player-2,1",
                    "residual",
                    "-battlerservice:timer|player:player-2|remainingsecs:4",
                    "-battlerservice:timer|player:player-2|done|remainingsecs:0",
                    "continue",
                    "forfeited|player:player-2",
                    "win|side:0",
                ]
            );
        });
    }
}
