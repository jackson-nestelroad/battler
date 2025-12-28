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

    fn update_log(&mut self) {
        self.logs.append(self.battle.new_log_entries());
    }

    fn continue_battle(&mut self) -> Result<bool> {
        let continued = if self.battle.ready_to_continue()? {
            self.cancel_timers_tx.send(()).ok();
            self.battle.continue_battle()?;
            true
        } else {
            false
        };
        self.update_log();
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
/// reference to the battle, and we must ensure these tasks are finished when the battle is dropped.
/// This object manages such things.
struct LiveBattleManager<'d> {
    live_battle: Arc<tokio::sync::Mutex<LiveBattle<'d>>>,
    task_tx: tokio::sync::Mutex<Option<mpsc::Sender<()>>>,
    task_rx: tokio::sync::Mutex<mpsc::Receiver<()>>,
}

impl<'d> LiveBattleManager<'d> {
    fn new(battle: LiveBattle<'d>) -> Self {
        let (task_tx, task_rx) = mpsc::channel(1);
        Self {
            live_battle: Arc::new(tokio::sync::Mutex::new(battle)),
            task_tx: tokio::sync::Mutex::new(Some(task_tx)),
            task_rx: tokio::sync::Mutex::new(task_rx),
        }
    }

    async fn task_tx(&self) -> Result<mpsc::Sender<()>> {
        self.task_tx.lock().await.clone().ok_or_else(|| {
            Error::msg("battle has been canceled by the service, so task_tx is unavailable")
        })
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
            live_battle.update_log();
        }
        Self::proceed(self.live_battle.clone(), self.task_tx().await?.downgrade()).await;
        Ok(())
    }

    async fn make_choice(&self, player: &str, choice: &str) -> Result<()> {
        self.live_battle.lock().await.make_choice(player, choice)?;
        Self::proceed(self.live_battle.clone(), self.task_tx().await?.downgrade()).await;
        Ok(())
    }

    async fn proceed(
        battle: Arc<tokio::sync::Mutex<LiveBattle<'d>>>,
        task_tx: mpsc::WeakSender<()>,
    ) {
        let task_tx = match task_tx.upgrade() {
            Some(task_tx) => task_tx,
            None => return,
        };
        // SAFETY: self.proceed_tasks is joined during shutdown, before this object is dropped.
        // Additionally, the Drop implementation panics if tasks are remaining. Thus, no tasks
        // extend beyond the lifetime of this object, and the lifetime of 'd.
        let live_battle = unsafe {
            std::mem::transmute::<
                Arc<tokio::sync::Mutex<LiveBattle<'d>>>,
                Arc<tokio::sync::Mutex<LiveBattle<'static>>>,
            >(battle.clone())
        };
        let mut battle = battle.lock().await;

        // Garbage collection.
        while let Some(_) = battle.proceed_tasks.try_join_next() {}

        battle
            .proceed_tasks
            .spawn(LiveBattleManager::proceed_detached(
                Arc::downgrade(&live_battle),
                task_tx.downgrade(),
            ));
    }

    async fn proceed_detached(
        battle: Weak<tokio::sync::Mutex<LiveBattle<'d>>>,
        task_tx: mpsc::WeakSender<()>,
    ) {
        let battle = match battle.upgrade() {
            Some(battle) => battle,
            None => return,
        };
        let task_tx = match task_tx.upgrade() {
            Some(task_tx) => task_tx,
            None => return,
        };
        if let Err(err) = Self::proceed_detached_internal(battle.clone(), task_tx).await {
            battle.lock().await.error = Some(format!("{err:#}"));
        }
    }

    async fn proceed_detached_internal(
        battle: Arc<tokio::sync::Mutex<LiveBattle<'d>>>,
        task_tx: mpsc::Sender<()>,
    ) -> Result<()> {
        let (continued, ended) = {
            let mut battle = battle.lock().await;
            battle.error = None;
            (battle.continue_battle()?, battle.battle.ended())
        };

        if continued && !ended {
            Self::resume_timers(battle, task_tx).await?;
        }

        Ok(())
    }

    async fn join_all_timer_tasks(battle: &tokio::sync::Mutex<LiveBattle<'d>>) {
        // Soft cancellation; we want timer tasks to finish so that their state is updated.
        battle.lock().await.cancel_timers_tx.send(()).ok();

        // Join all timer tasks when finished.
        let mut current_timer_tasks = JoinSet::default();
        std::mem::swap(
            &mut current_timer_tasks,
            &mut battle.lock().await.current_timer_tasks,
        );
        current_timer_tasks.join_all().await;
    }

    // We must manually add the `Send` trait because this async function can be recursive: when a
    // timer finished, the battle proceeds and restarts timers.
    fn resume_timers(
        battle: Arc<tokio::sync::Mutex<LiveBattle<'d>>>,
        task_tx: mpsc::Sender<()>,
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
                            // `battle.timers`.
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
                // SAFETY: All tasks spawned here are joined during shutdown, and before a new
                // JoinSet is created. No task can extend beyond the lifetime of this object and the
                // lifetime of 'd.
                let battle = unsafe {
                    std::mem::transmute::<
                        Arc<tokio::sync::Mutex<LiveBattle<'d>>>,
                        Arc<tokio::sync::Mutex<LiveBattle<'static>>>,
                    >(battle.clone())
                };
                let mut tasks = JoinSet::default();
                for timer_type in timers {
                    tasks.spawn(LiveBattleManager::run_timer(
                        Arc::downgrade(&battle),
                        timer_type,
                        choice_made_rx.resubscribe(),
                        cancel_timers_rx.resubscribe(),
                        task_tx.downgrade(),
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
        battle: Weak<tokio::sync::Mutex<LiveBattle<'d>>>,
        timer_type: TimerType,
        choice_made_rx: broadcast::Receiver<String>,
        cancel_timers_rx: broadcast::Receiver<()>,
        task_tx: mpsc::WeakSender<()>,
    ) {
        let battle = match battle.upgrade() {
            Some(battle) => battle,
            None => return,
        };
        let task_tx = match task_tx.upgrade() {
            Some(task_tx) => task_tx,
            None => return,
        };
        let (state, proceed) = match Self::run_timer_internal(
            battle.clone(),
            &timer_type,
            choice_made_rx,
            cancel_timers_rx,
        )
        .await
        {
            Ok((state, proceed)) => (state, proceed),
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
        if proceed {
            Self::proceed(battle, task_tx.downgrade()).await;
        }
    }

    async fn run_timer_internal(
        battle: Arc<tokio::sync::Mutex<LiveBattle<'d>>>,
        timer_type: &TimerType,
        mut choice_made_rx: broadcast::Receiver<String>,
        mut cancel_timers_rx: broadcast::Receiver<()>,
    ) -> Result<(Option<TimerState>, bool)> {
        // Read the current state of the timer.
        let state = match battle.lock().await.timers.get(&timer_type).cloned() {
            Some(state) => state,
            None => return Ok((None, false)),
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

        let proceed = loop {
            recalculate_remaining(&mut now, &mut remaining);

            // Timer finished.
            if remaining.is_zero() {
                break true;
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
                    break true;
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
                        break false;
                    }
                }
                result = cancel_timers_rx.recv() => {
                    // The timer was canceled, likely because the battle continued.
                    result?;
                    break false;
                }
            }
        };

        recalculate_remaining(&mut now, &mut remaining);

        // Save the timer state, even if the duration is zero.
        //
        // This avoids the scenario where the timer ends immediately after the player makes their
        // choice. We want the timer to end immediately the next time it starts, which requires the
        // state to be saved.
        Ok((
            Some(TimerState {
                total: state.total,
                remaining,
                warnings: state.warnings,
            }),
            proceed,
        ))
    }

    async fn shutdown(&self) {
        let mut battle = self.live_battle.lock().await;
        battle.proceed_tasks.shutdown().await;
        battle.current_timer_tasks.shutdown().await;
    }

    fn cancel(&self) {
        // Drop our Sender, so that the Receiver is only open because of asynchronous tasks.
        self.task_tx.blocking_lock().take();

        {
            let mut battle = self.live_battle.blocking_lock();

            // Abort all tasks.
            battle.proceed_tasks.abort_all();
            battle.current_timer_tasks.abort_all();

            // Then detach. We block on the Receiver below to know when all tasks finish.
            battle.proceed_tasks.detach_all();
            battle.current_timer_tasks.detach_all();
        }

        // Wait for all Senders to be dropped. One Sender is given to each task, so the Receiver
        // closing means all tasks are finished.
        self.task_rx.blocking_lock().blocking_recv();
    }
}

impl Drop for LiveBattleManager<'_> {
    fn drop(&mut self) {
        // SAFETY: Ensure all tasks finish.
        tokio::task::block_in_place(|| {
            self.cancel();
        });
        // SAFETY: Breaking these invariants can lead to undefined behavior, so it is better to just
        // crash now.
        let battle = self
            .live_battle
            .try_lock()
            .expect("battle cannot be locked during drop");
        assert!(
            battle.proceed_tasks.is_empty(),
            "battle has {} proceed tasks during drop",
            battle.proceed_tasks.len()
        );
        assert!(
            battle.current_timer_tasks.is_empty(),
            "battle has {} timer tasks during drop",
            battle.current_timer_tasks.len()
        );

        assert!(
            self.task_rx.get_mut().is_closed(),
            "battle has remaining tasks open during drop"
        );

        let strong_count = Arc::strong_count(&self.live_battle);
        assert_eq!(
            strong_count, 1,
            "live battle has {strong_count} references during drop"
        );
    }
}

/// Service for managing multiple battles on the [`battler`] battle engine.
pub struct BattlerService<'d> {
    data: &'d dyn DataStore,

    // SAFETY: Arc is used for the simplicity of looking up battles internally. When we drop this
    // object, we unwrap the Arc to destroy the object. Thus, these battles cannot live beyond the
    // lifetime of this object and the lifetime of 'd.
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
            // SAFETY: Must call shutdown here to join all tasks that are using the battle, so that
            // the battle does not outlive this object.
            battle.shutdown().await;
            let battle = Arc::try_unwrap(battle)
                .map_err(|_| {
                    Error::msg("battle could not be unwrapped during deletion after shutdown")
                })
                .unwrap();

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

            // Purely for safety, ensure Drop is called here, which checks our invariants.
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

impl Drop for BattlerService<'_> {
    fn drop(&mut self) {
        tokio::task::block_in_place(move || {
            let mut battles = BTreeMap::default();
            std::mem::swap(&mut battles, self.battles.get_mut());
            for (_, battle) in battles {
                // SAFETY: Must synchronously cancel and wait for tasks to finish, so that the
                // battle does not outlive this object.
                battle.cancel();
                Arc::try_unwrap(battle)
                    .map_err(|_| {
                        Error::msg("battle could not be unwrapped during drop after cancel")
                    })
                    .unwrap();
            }
        });
    }
}
