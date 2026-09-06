use std::{
    collections::{
        BTreeMap,
        BTreeSet,
    },
    pin::Pin,
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

use ahash::{
    HashMap,
    HashSet,
};
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
    RequestType,
    SideData,
    TeamData,
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
        watch,
    },
    task::JoinSet,
};
use uuid::Uuid;

use crate::{
    Battle,
    BattleError,
    BattleMetadata,
    BattlePreview,
    BattleState,
    BattleStatus,
    DropReason,
    GlobalLogEntry,
    Player,
    PlayerPreview,
    PlayerState,
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
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct BattleServiceOptions {
    /// Player who created the battle.
    #[serde(default)]
    pub creator: String,

    /// Battle timers.
    #[serde(default)]
    pub timers: Timers,

    /// Log absolute deadlines for timers.
    #[serde(default)]
    pub log_timer_deadlines: bool,

    /// Special battle format type.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "typescript", ts(optional))]
    pub special: Option<String>,

    /// Disable team validation prior to the battle starting.
    #[serde(default)]
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "typescript", ts(optional))]
    pub no_team_validation: Option<bool>,
}

/// Options for detecting and dropping stuck or abandoned battles.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WatchdogOptions {
    /// Maximum time allowed for an individual proceed / turn step before timing out.
    pub proceed_step_timeout: Duration,
    /// Maximum allowed inactivity before dropping an unprogressing battle.
    pub max_inactivity_duration: Duration,
    /// Absolute wall-clock maximum lifetime of any battle.
    pub max_battle_duration: Duration,
}

impl Default for WatchdogOptions {
    fn default() -> Self {
        Self {
            proceed_step_timeout: Duration::from_secs(5),
            max_inactivity_duration: Duration::from_secs(600),
            max_battle_duration: Duration::from_secs(3600),
        }
    }
}

#[derive(Debug, Clone, Copy)]
enum TimerLogType {
    Warning,
    Done,
    Inactive,
    Clear,
}

/// An ongoing battle, managed by [`BattlerService`].
///
/// Operations on this object are intended to be **atomic**. In other words, the battle mutex is
/// locked for each operation here.
///
/// For non-atomic operations, the [`LiveBattleManager`] may make mutations to state in this object.
struct LiveBattle<'d> {
    uuid: Uuid,
    state: BattleState,
    battle: PublicCoreBattle<'d>,
    creator: String,
    sides: Vec<Side>,
    drop_reason: Option<DropReason>,
    logs: SplitLogs,
    special: Option<String>,

    timers: BTreeMap<TimerType, TimerState>,
    timers_config: Timers,

    choice_made_tx: broadcast::Sender<String>,
    cancel_timers_tx: broadcast::Sender<()>,
    global_log_tx: mpsc::Sender<GlobalLogEntry>,
    pending_global_logs: Vec<GlobalLogEntry>,
    started_at: Option<Instant>,
    last_activity: Instant,
    finished_at: Option<Instant>,
    log_timer_deadlines: bool,
    is_proceeding: bool,
    proceed_requested: bool,
    canceled: bool,
}

impl<'d> LiveBattle<'d> {
    fn new(
        options: CoreBattleOptions,
        mut engine_options: CoreBattleEngineOptions,
        service_options: BattleServiceOptions,
        data: &'d dyn DataStore,
        global_log_tx: mpsc::Sender<GlobalLogEntry>,
    ) -> Result<Self> {
        if service_options.no_team_validation.unwrap_or(false) {
            engine_options.validate_teams = false;
        }
        let uuid = Uuid::new_v4();
        let sides = Vec::from_iter([
            Self::new_side(&options.side_1),
            Self::new_side(&options.side_2),
        ]);
        let battle = PublicCoreBattle::new(options, data, engine_options)?;
        let logs = SplitLogs::new(uuid, sides.len());

        let (choice_made_tx, _) = broadcast::channel(48);
        let (cancel_timers_tx, _) = broadcast::channel(48);

        let players = sides
            .iter()
            .flat_map(|side| side.players.iter().map(|player| player.id.clone()))
            .collect::<Vec<_>>();
        let timers = service_options.timers.clone().to_state(&players);
        LiveBattle {
            uuid,
            state: BattleState::Preparing,
            battle,
            creator: service_options.creator,
            sides,
            drop_reason: None,
            logs,
            special: service_options.special,
            timers,
            timers_config: service_options.timers,
            choice_made_tx,
            cancel_timers_tx,
            global_log_tx,
            pending_global_logs: Vec::new(),
            started_at: None,
            last_activity: Instant::now(),
            finished_at: None,
            log_timer_deadlines: service_options.log_timer_deadlines,
            is_proceeding: false,
            proceed_requested: false,
            canceled: false,
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
        self.state
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
            drop_reason: self.drop_reason.clone(),
            metadata: BattleMetadata {
                creator: self.creator.clone(),
                battle_type: self.battle.battle_type(),
                rules: self.battle.rules(),
                timers: self.timers_config.clone(),
                special: self.special.clone(),
            },
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
            battle_type: self.battle.battle_type(),
            state: self.battle_state(),
            turn: self.battle.turn(),
            special: self.special.clone(),
            drop_reason: self.drop_reason.clone(),
        }
    }

    fn log_for_side(&self, side: Option<usize>) -> &Log {
        side.and_then(|side| self.logs.side_log(side))
            .unwrap_or(self.logs.public_log())
    }

    fn update_team(&mut self, player: &str, team: TeamData) -> Result<()> {
        self.last_activity = Instant::now();
        self.battle.update_team(player, team)?;
        self.update_player_state(player)
    }

    fn update_player_state(&mut self, player: &str) -> Result<()> {
        let state = if self.battle.validate_player(player).is_ok() {
            PlayerState::Ready
        } else {
            PlayerState::Waiting
        };
        self.player_mut_or_error(player)?.state = state;
        Ok(())
    }

    fn make_choice(&mut self, player: &str, choice: &str) -> Result<()> {
        self.last_activity = Instant::now();
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

    fn update_log(&mut self) -> Result<()> {
        let new_entries: Vec<String> = self
            .battle
            .new_log_entries()
            .map(|s| s.to_owned())
            .collect();
        let has_new_entries = !new_entries.is_empty();
        self.logs.append(new_entries, &mut self.pending_global_logs);
        let players = self
            .sides
            .iter()
            .flat_map(|side| side.players.iter().map(|player| &player.id));
        let mut has_request = false;
        for player in players {
            if self.battle.request_for_player(player)?.is_some() {
                has_request = true;
                break;
            }
        }
        if has_request && has_new_entries {
            self.inject_log_entries(["request"]);
        }
        Ok(())
    }

    fn continue_battle(&mut self) -> Result<bool> {
        let continued = if self.battle.ready_to_continue()? {
            self.cancel_timers_tx.send(()).ok();
            self.battle.continue_battle()?;
            self.last_activity = Instant::now();
            true
        } else {
            false
        };
        self.update_log()?;
        if self.battle.ended() {
            self.state = BattleState::Finished;
            if self.finished_at.is_none() {
                self.finished_at = Some(Instant::now());
            }
        }
        Ok(continued)
    }

    fn injected_log_entry(entry: String) -> String {
        format!("-battlerservice:{entry}")
    }

    fn timer_log(
        timer_type: &TimerType,
        remaining: Duration,
        timer_log_type: Option<TimerLogType>,
        log_timer_deadlines: bool,
    ) -> String {
        let log_type = match timer_log_type {
            Some(TimerLogType::Warning) => "|warning",
            Some(TimerLogType::Done) => "|done",
            Some(TimerLogType::Inactive) => "|inactive",
            Some(TimerLogType::Clear) => "|clear",
            None => "",
        };
        let timer_type = match timer_type {
            TimerType::Battle => "battle".to_owned(),
            TimerType::Player(player) => format!("player:{player}"),
            TimerType::Action(player) => format!("action:{player}"),
            TimerType::TeamPreview => "teampreview".to_owned(),
        };
        if log_timer_deadlines {
            let deadline = SystemTime::now() + remaining;
            let deadline_secs = deadline
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs();
            format!(
                "timer|{timer_type}{log_type}|remainingsecs:{}|deadline:{deadline_secs}",
                remaining.as_secs()
            )
        } else {
            format!(
                "timer|{timer_type}{log_type}|remainingsecs:{}",
                remaining.as_secs()
            )
        }
    }

    fn inject_log_entries<I, S>(&mut self, entries: I)
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut string_entries = Vec::new();
        for entry in entries {
            string_entries.push(Self::injected_log_entry(entry.into()));
        }
        self.logs
            .append(string_entries, &mut self.pending_global_logs);
    }

    fn handle_timer_finished(&mut self, timer_type: &TimerType) -> Result<()> {
        log::info!("Timer {timer_type:?} finished for battle {}", self.uuid);

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

        let log_timer_deadlines = self.log_timer_deadlines;
        self.inject_log_entries([Self::timer_log(
            timer_type,
            Duration::ZERO,
            Some(TimerLogType::Done),
            log_timer_deadlines,
        )]);

        match timer_type {
            TimerType::Battle => self.battle.auto_end(),
            TimerType::Player(player) => self.battle.set_player_choice(player, "forfeit"),
            TimerType::Action(player) => self.battle.set_player_choice(player, "randomall"),
            TimerType::TeamPreview => {
                let players_to_randomize: Vec<String> = self
                    .battle
                    .active_requests()
                    .filter(|(_, request)| request.request_type() == RequestType::TeamPreview)
                    .map(|(player, _)| player)
                    .collect();
                for player in players_to_randomize {
                    self.battle.set_player_choice(&player, "randomall").ok();
                }
                Ok(())
            }
        }
    }

    pub fn take_pending_logs(&mut self) -> Vec<GlobalLogEntry> {
        std::mem::take(&mut self.pending_global_logs)
    }

    fn is_timer_active(&self, timer_type: &TimerType) -> Result<bool> {
        let is_team_preview = self
            .battle
            .active_requests()
            .any(|(_, request)| request.request_type() == RequestType::TeamPreview);
        let active = match timer_type {
            TimerType::Battle => !is_team_preview,
            TimerType::Player(player) => match self.battle.request_for_player(player)? {
                Some(request) => request.request_type() != RequestType::TeamPreview,
                None => false,
            },
            TimerType::Action(player) => match self.battle.request_for_player(player)? {
                Some(request) => request.request_type() != RequestType::TeamPreview,
                None => false,
            },
            TimerType::TeamPreview => is_team_preview,
        };
        Ok(active)
    }

    fn log_type_for_inactive(timer_type: &TimerType) -> TimerLogType {
        match timer_type {
            TimerType::TeamPreview => TimerLogType::Clear,
            _ => TimerLogType::Inactive,
        }
    }

    fn active_timer_types(&self) -> Result<BTreeSet<TimerType>> {
        let mut timers = BTreeSet::default();
        for timer_type in self.timers.keys() {
            if self.is_timer_active(timer_type)? {
                timers.insert(timer_type.clone());
            }
        }
        Ok(timers)
    }
}

#[derive(Default)]
struct LiveBattleTasks {
    proceed_tasks: JoinSet<()>,
    current_timer_tasks: JoinSet<()>,
}

impl LiveBattleTasks {
    fn abort_all(&mut self) {
        self.proceed_tasks.abort_all();
        self.current_timer_tasks.abort_all();
    }
}

/// A wrapper around a [`LiveBattle`] for non-atomic operations.
///
/// Some tasks are spawned in the background, such as tasks for battle timers. Such tasks must have
/// reference to the battle, and we must ensure these tasks are finished when the battle is dropped.
/// This object manages such things.
struct LiveBattleManager<'d> {
    uuid: Uuid,
    side_players: Vec<HashSet<String>>,
    live_battle: Arc<Mutex<LiveBattle<'d>>>,
    tasks: Arc<Mutex<LiveBattleTasks>>,
    preview_tx: watch::Sender<BattlePreview>,
    preview_rx: watch::Receiver<BattlePreview>,
    task_tx: Mutex<Option<mpsc::Sender<()>>>,
    _task_rx: Mutex<mpsc::Receiver<()>>,
}

impl<'d> LiveBattleManager<'d> {
    async fn inject_and_flush_logs(&self, entries: impl IntoIterator<Item = impl Into<String>>) {
        {
            let mut battle = self.live_battle.lock().await;
            battle.inject_log_entries(entries);
        }
        Self::flush_battle_logs(&self.live_battle).await;
    }

    async fn flush_logs(&self) {
        Self::flush_battle_logs(&self.live_battle).await;
    }

    async fn flush_battle_logs(battle: &Arc<Mutex<LiveBattle<'d>>>) {
        let (tx, logs) = {
            let mut battle = battle.lock().await;
            (battle.global_log_tx.clone(), battle.take_pending_logs())
        };
        for log in logs {
            if tx.send(log).await.is_err() {
                break;
            }
        }
    }

    fn new(battle: LiveBattle<'d>) -> Self {
        let uuid = battle.uuid;
        let initial_preview = battle.battle_preview();
        let (preview_tx, preview_rx) = watch::channel(initial_preview);
        let side_players = battle
            .sides
            .iter()
            .map(|side| {
                side.players
                    .iter()
                    .map(|p| p.id.clone())
                    .collect::<HashSet<String>>()
            })
            .collect::<Vec<_>>();
        let (task_tx, task_rx) = mpsc::channel(1);
        Self {
            uuid,
            side_players,
            live_battle: Arc::new(Mutex::new(battle)),
            tasks: Arc::new(Mutex::new(LiveBattleTasks::default())),
            preview_tx,
            preview_rx,
            task_tx: Mutex::new(Some(task_tx)),
            _task_rx: Mutex::new(task_rx),
        }
    }

    fn update_preview(&self, live_battle: &LiveBattle) {
        self.preview_tx.send_replace(live_battle.battle_preview());
    }

    fn side_players(&self, side: Option<usize>) -> Option<HashSet<String>> {
        side.and_then(|side| self.side_players.get(side).cloned())
    }

    async fn task_tx(&self) -> Result<mpsc::Sender<()>> {
        self.task_tx.lock().await.clone().ok_or_else(|| {
            Error::msg("battle has been canceled by the service, so task_tx is unavailable")
        })
    }

    fn players(&self) -> Vec<String> {
        self.side_players
            .iter()
            .flat_map(|s| s.iter().cloned())
            .collect()
    }

    async fn battle_state(&self) -> BattleState {
        self.battle_preview().state
    }

    async fn battle(&self) -> Battle {
        self.live_battle.lock().await.battle()
    }

    fn battle_preview(&self) -> BattlePreview {
        self.preview_rx.borrow().clone()
    }

    async fn finished_at(&self) -> Option<Instant> {
        self.live_battle.lock().await.finished_at
    }

    async fn log_for_side<F, R>(&self, side: Option<usize>, f: F) -> R
    where
        F: Fn(&Log) -> R,
    {
        f(self.live_battle.lock().await.log_for_side(side))
    }

    async fn update_team(&self, player: &str, team: TeamData) -> Result<()> {
        {
            let mut battle = self.live_battle.lock().await;
            if battle.state == BattleState::Finished {
                return Err(Error::msg("cannot update team on a finished battle"));
            }
            battle.update_team(player, team)?;

            // Inject a log entry so that clients can refresh player states.
            battle.inject_log_entries([format!("teamupdate|player:{player}")]);
            self.update_preview(&battle);
        }
        self.flush_logs().await;

        Ok(())
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
            if live_battle.state == BattleState::Finished {
                return Err(Error::msg("cannot start a finished battle"));
            }
            live_battle.battle.start()?;
            live_battle.inject_log_entries(["started"]);
            live_battle.update_log()?;
            live_battle.state = BattleState::Active;
            live_battle.started_at = Some(Instant::now());
            live_battle.last_activity = Instant::now();
            self.update_preview(&live_battle);
        }
        self.flush_logs().await;
        Self::proceed(
            self.uuid,
            self.live_battle.clone(),
            self.tasks.clone(),
            self.preview_tx.clone(),
            self.task_tx().await?,
        )
        .await;
        Ok(())
    }

    async fn make_choice(&self, player: &str, choice: &str) -> Result<()> {
        {
            let mut battle = self.live_battle.lock().await;
            if battle.state == BattleState::Finished {
                return Err(Error::msg("the battle is over"));
            }
            battle.make_choice(player, choice)?;
            self.update_preview(&battle);
        }
        Self::proceed(
            self.uuid,
            self.live_battle.clone(),
            self.tasks.clone(),
            self.preview_tx.clone(),
            self.task_tx().await?,
        )
        .await;
        Ok(())
    }

    async fn proceed(
        uuid: Uuid,
        battle: Arc<Mutex<LiveBattle<'d>>>,
        tasks: Arc<Mutex<LiveBattleTasks>>,
        preview_tx: watch::Sender<BattlePreview>,
        task_tx: mpsc::Sender<()>,
    ) {
        let should_spawn = {
            let mut battle = battle.lock().await;
            if battle.canceled || battle.state == BattleState::Finished {
                return;
            }
            if battle.is_proceeding {
                battle.proceed_requested = true;
                false
            } else {
                battle.is_proceeding = true;
                true
            }
        };

        if !should_spawn {
            return;
        }

        let static_battle = unsafe {
            std::mem::transmute::<Arc<Mutex<LiveBattle<'d>>>, Arc<Mutex<LiveBattle<'static>>>>(
                battle.clone(),
            )
        };

        {
            let tasks_arc = tasks.clone();
            let mut tasks = tasks.lock().await;
            while tasks.proceed_tasks.try_join_next().is_some() {}
            tasks.proceed_tasks.spawn(proceed_loop(
                uuid,
                static_battle,
                tasks_arc,
                preview_tx,
                task_tx.downgrade(),
            ));
        }
    }

    async fn drop_battle_internal(
        uuid: Uuid,
        battle: &Arc<Mutex<LiveBattle<'d>>>,
        tasks: &Arc<Mutex<LiveBattleTasks>>,
        preview_tx: &watch::Sender<BattlePreview>,
        reason: DropReason,
        abort_proceed_tasks: bool,
    ) -> bool {
        log::warn!("Dropping live battle {uuid}: {reason}");

        {
            let mut tasks = tasks.lock().await;
            if abort_proceed_tasks {
                tasks.proceed_tasks.abort_all();
            }
            tasks.current_timer_tasks.abort_all();
        }

        {
            let mut battle = battle.lock().await;

            if battle.state == BattleState::Finished {
                return false;
            }

            battle.state = BattleState::Finished;
            battle.drop_reason = Some(reason.clone());
            if battle.finished_at.is_none() {
                battle.finished_at = Some(Instant::now());
            }
            battle.is_proceeding = false;
            battle.proceed_requested = false;
            battle.cancel_timers_tx.send(()).ok();

            battle.inject_log_entries([format!("dropped|reason:{reason}")]);

            preview_tx.send_replace(battle.battle_preview());
        }

        Self::flush_battle_logs(battle).await;
        true
    }

    async fn drop_battle(&self, reason: DropReason) -> bool {
        Self::drop_battle_internal(
            self.uuid,
            &self.live_battle,
            &self.tasks,
            &self.preview_tx,
            reason,
            true,
        )
        .await
    }

    async fn check_stuck_condition(&self, options: &WatchdogOptions) -> Option<DropReason> {
        if self.battle_preview().state != BattleState::Active {
            return None;
        }

        let mut battle = self.live_battle.lock().await;
        if battle.state != BattleState::Active {
            return None;
        }

        // Max wall-clock duration exceeded
        if let Some(started_at) = battle.started_at
            && started_at.elapsed() >= options.max_battle_duration
        {
            return Some(DropReason::ExceededMaxDuration);
        }

        // Inactivity timeout
        if battle.last_activity.elapsed() >= options.max_inactivity_duration {
            return Some(DropReason::InactivityTimeout);
        }

        // Zombie deadlock: battle has not ended, cannot continue, has no active requests, and is
        // not currently proceeding
        let ended = battle.battle.ended();
        let ready_to_continue = battle.battle.ready_to_continue().unwrap_or(false);
        let has_active_requests = battle.battle.active_requests().next().is_some();
        if !ended && !ready_to_continue && !has_active_requests && !battle.is_proceeding {
            return Some(DropReason::ZombieDeadlock);
        }

        None
    }
}

const PROCEED_STEP_TIMEOUT: Duration = Duration::from_secs(5);

async fn proceed_loop(
    uuid: Uuid,
    battle: Arc<Mutex<LiveBattle<'static>>>,
    tasks: Arc<Mutex<LiveBattleTasks>>,
    preview_tx: watch::Sender<BattlePreview>,
    task_tx: mpsc::WeakSender<()>,
) {
    loop {
        let task_tx = match task_tx.upgrade() {
            Some(task_tx) => task_tx,
            None => return,
        };

        let proceed_result = tokio::time::timeout(
            PROCEED_STEP_TIMEOUT,
            LiveBattleManager::proceed_detached_internal(
                uuid,
                battle.clone(),
                tasks.clone(),
                preview_tx.clone(),
                task_tx,
            ),
        )
        .await;

        match proceed_result {
            Ok(Ok(())) => {}
            Ok(Err(err)) => {
                log::error!("Live battle {uuid} proceed failed: {err:#}");
                LiveBattleManager::drop_battle_internal(
                    uuid,
                    &battle,
                    &tasks,
                    &preview_tx,
                    DropReason::EngineError(format!("{err:#}")),
                    false,
                )
                .await;
                break;
            }
            Err(_) => {
                log::error!("Live battle {uuid} step timed out after {PROCEED_STEP_TIMEOUT:?}");
                LiveBattleManager::drop_battle_internal(
                    uuid,
                    &battle,
                    &tasks,
                    &preview_tx,
                    DropReason::ExecutionTimeout,
                    false,
                )
                .await;
                break;
            }
        }

        let continue_loop = {
            let mut battle = battle.lock().await;
            if battle.canceled || battle.state == BattleState::Finished || battle.battle.ended() {
                battle.is_proceeding = false;
                false
            } else {
                let ready = battle.battle.ready_to_continue().unwrap_or(false);
                if battle.proceed_requested || ready {
                    battle.proceed_requested = false;
                    true
                } else {
                    battle.is_proceeding = false;
                    false
                }
            }
        };

        if !continue_loop {
            break;
        }
    }
}

impl<'d> LiveBattleManager<'d> {
    async fn proceed_detached_internal(
        uuid: Uuid,
        battle: Arc<Mutex<LiveBattle<'d>>>,
        tasks: Arc<Mutex<LiveBattleTasks>>,
        preview_tx: watch::Sender<BattlePreview>,
        task_tx: mpsc::Sender<()>,
    ) -> Result<()> {
        log::info!("Live battle {uuid} is proceeding");
        let is_empty = {
            let tasks = tasks.lock().await;
            tasks.current_timer_tasks.is_empty()
        };
        let (continued, ended, active_timers) = {
            let mut battle = battle.lock().await;
            if battle.canceled || battle.state == BattleState::Finished {
                return Ok(());
            }
            let active_timers = battle.active_timer_types()?;
            let continued = battle.continue_battle()?;
            let ended = battle.battle.ended();
            preview_tx.send_replace(battle.battle_preview());
            (continued, ended, active_timers)
        };

        if (continued || is_empty) && !ended {
            {
                let battle = battle.lock().await;
                if battle.canceled || battle.state == BattleState::Finished {
                    return Ok(());
                }
            }
            Self::resume_timers(uuid, battle.clone(), tasks.clone(), preview_tx, task_tx).await?;
        } else if ended {
            Self::join_all_timer_tasks(&battle, &tasks).await;
            let mut battle = battle.lock().await;
            if battle.canceled || battle.drop_reason.is_some() {
                return Ok(());
            }
            let log_timer_deadlines = battle.log_timer_deadlines;
            let timer_logs = active_timers
                .iter()
                .filter_map(|timer_type| {
                    battle.timers.get(timer_type).map(|state| {
                        LiveBattle::timer_log(
                            timer_type,
                            state.remaining,
                            Some(LiveBattle::log_type_for_inactive(timer_type)),
                            log_timer_deadlines,
                        )
                    })
                })
                .collect::<Vec<_>>();
            battle.inject_log_entries(timer_logs);
            battle.inject_log_entries(["done"]);
            preview_tx.send_replace(battle.battle_preview());
        }

        Self::flush_battle_logs(&battle).await;

        Ok(())
    }

    async fn join_all_timer_tasks(battle: &Mutex<LiveBattle<'d>>, tasks: &Mutex<LiveBattleTasks>) {
        let mut current_timer_tasks = {
            {
                let battle = battle.lock().await;
                battle.cancel_timers_tx.send(()).ok();
            }
            let mut tasks = tasks.lock().await;
            std::mem::take(&mut tasks.current_timer_tasks)
        };
        let timeout_result = tokio::time::timeout(Duration::from_secs(2), async {
            while current_timer_tasks.join_next().await.is_some() {}
        })
        .await;
        if timeout_result.is_err() {
            log::warn!("Timer tasks took too long to finish; aborting them");
            current_timer_tasks.abort_all();
            while current_timer_tasks.join_next().await.is_some() {}
        }
    }

    async fn resume_timers(
        uuid: Uuid,
        battle: Arc<Mutex<LiveBattle<'d>>>,
        tasks: Arc<Mutex<LiveBattleTasks>>,
        preview_tx: watch::Sender<BattlePreview>,
        task_tx: mpsc::Sender<()>,
    ) -> Result<()> {
        log::trace!("Starting to join all previous timer tasks for live battle {uuid}");

        Self::join_all_timer_tasks(&battle, &tasks).await;

        log::trace!("Joined all previous timer tasks for live battle {uuid}");

        let (timers, choice_made_tx, cancel_timers_tx) = {
            let mut battle = battle.lock().await;
            if battle.canceled || battle.state == BattleState::Finished {
                return Ok(());
            }

            let is_team_preview = battle
                .battle
                .active_requests()
                .any(|(_, request)| request.request_type() == RequestType::TeamPreview);
            if !is_team_preview && battle.timers.contains_key(&TimerType::TeamPreview) {
                battle.timers.remove(&TimerType::TeamPreview);
            }

            for (timer_type, timer_state) in &mut battle.timers {
                if timer_type.reset_on_resume() {
                    timer_state.remaining = timer_state.total;
                }
            }

            let timers = battle.active_timer_types()?;

            let mut timer_logs = timers
                .iter()
                .filter_map(|timer_type| {
                    battle.timers.get(timer_type).map(|timer_state| {
                        LiveBattle::timer_log(
                            timer_type,
                            timer_state.remaining,
                            None,
                            battle.log_timer_deadlines,
                        )
                    })
                })
                .collect::<Vec<_>>();

            for (timer_type, timer_state) in &battle.timers {
                if !timers.contains(timer_type) {
                    timer_logs.push(LiveBattle::timer_log(
                        timer_type,
                        timer_state.remaining,
                        Some(LiveBattle::log_type_for_inactive(timer_type)),
                        battle.log_timer_deadlines,
                    ));
                }
            }

            battle.inject_log_entries(timer_logs);
            preview_tx.send_replace(battle.battle_preview());

            (
                timers,
                battle.choice_made_tx.subscribe(),
                battle.cancel_timers_tx.subscribe(),
            )
        };

        log::info!("Resuming timers for live battle {uuid}: {timers:?}");

        let static_battle = unsafe {
            std::mem::transmute::<Arc<Mutex<LiveBattle<'d>>>, Arc<Mutex<LiveBattle<'static>>>>(
                battle.clone(),
            )
        };

        let mut new_tasks = JoinSet::default();
        let weak_tasks = Arc::downgrade(&tasks);
        for timer_type in timers {
            log::debug!("Spawning timer task for live battle {uuid}: {timer_type:?}");
            new_tasks.spawn(LiveBattleManager::run_timer(
                uuid,
                Arc::downgrade(&static_battle),
                weak_tasks.clone(),
                preview_tx.clone(),
                timer_type,
                choice_made_tx.resubscribe(),
                cancel_timers_tx.resubscribe(),
                task_tx.downgrade(),
            ));
        }

        {
            let battle = battle.lock().await;
            if battle.canceled || battle.state == BattleState::Finished {
                new_tasks.abort_all();
                return Ok(());
            }
        }

        {
            let mut tasks = tasks.lock().await;
            tasks.current_timer_tasks = new_tasks;
        }

        log::trace!("Spawned all timer tasks for live battle {uuid}");
        log::trace!("Done resuming timers for live battle {uuid}");

        Self::flush_battle_logs(&battle).await;

        Ok(())
    }

    fn run_timer(
        uuid: Uuid,
        battle: Weak<Mutex<LiveBattle<'d>>>,
        tasks: Weak<Mutex<LiveBattleTasks>>,
        preview_tx: watch::Sender<BattlePreview>,
        timer_type: TimerType,
        choice_made_rx: broadcast::Receiver<String>,
        cancel_timers_rx: broadcast::Receiver<()>,
        task_tx: mpsc::WeakSender<()>,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send + 'd>> {
        Box::pin(async move {
            let battle = match battle.upgrade() {
                Some(battle) => battle,
                None => return,
            };
            let tasks = match tasks.upgrade() {
                Some(tasks) => tasks,
                None => return,
            };
            let task_tx = match task_tx.upgrade() {
                Some(task_tx) => task_tx,
                None => return,
            };
            log::debug!("Running timer {timer_type:?} for live battle {uuid}");
            let (mut state, proceed) = match Self::run_timer_internal(
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

            if finished && !timer_type.reset_on_resume() {
                state = None;
            }

            {
                let mut battle = battle.lock().await;

                match state {
                    Some(state) => battle.timers.insert(timer_type.clone(), state),
                    None => battle.timers.remove(&timer_type),
                };

                if finished && let Err(err) = battle.handle_timer_finished(&timer_type) {
                    log::error!(
                        "Failed to handle finished timer {timer_type:?} for battle {uuid}: {err:?}"
                    );
                }
            }
            Self::flush_battle_logs(&battle).await;

            if proceed {
                let static_battle = unsafe {
                    std::mem::transmute::<Arc<Mutex<LiveBattle<'d>>>, Arc<Mutex<LiveBattle<'static>>>>(
                        battle,
                    )
                };
                tokio::spawn(LiveBattleManager::<'static>::proceed(
                    uuid,
                    static_battle,
                    tasks,
                    preview_tx,
                    task_tx,
                ));
            }
        })
    }

    async fn run_timer_internal(
        battle_arc: Arc<Mutex<LiveBattle<'d>>>,
        timer_type: &TimerType,
        mut choice_made_rx: broadcast::Receiver<String>,
        mut cancel_timers_rx: broadcast::Receiver<()>,
    ) -> Result<(Option<TimerState>, bool)> {
        // Read the current state of the timer and check if it is active.
        let (state, active) = {
            let battle = battle_arc.lock().await;
            let state = match battle.timers.get(timer_type).cloned() {
                Some(state) => state,
                None => return Ok((None, false)),
            };
            let active = battle.is_timer_active(timer_type)?;
            (state, active)
        };

        if !active {
            {
                let mut battle = battle_arc.lock().await;
                let log_timer_deadlines = battle.log_timer_deadlines;
                battle.inject_log_entries([LiveBattle::timer_log(
                    timer_type,
                    state.remaining,
                    Some(LiveBattle::log_type_for_inactive(timer_type)),
                    log_timer_deadlines,
                )]);
            }
            Self::flush_battle_logs(&battle_arc).await;
            return Ok((Some(state), false));
        }

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
                .find(|time| remaining >= **time)
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
                    {
                        let mut battle = battle_arc.lock().await;
                        let log_timer_deadlines = battle.log_timer_deadlines;
                        battle.inject_log_entries([LiveBattle::timer_log(
                            timer_type,
                            next_warning.unwrap_or_default(),
                            Some(TimerLogType::Warning),
                            log_timer_deadlines,
                        )]);
                    }
                    Self::flush_battle_logs(&battle_arc).await;
                }
                choice_made = choice_made_rx.recv() => {
                    // A choice was made for the player this timer corresponds to.
                    match choice_made {
                        Ok(player) => {
                            if timer_type.player().is_some_and(|timer_player| timer_player == player) {
                                break false;
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            break false;
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            // If lag occurs, we might have missed the choice. Check the battle state directly.
                            while choice_made_rx.try_recv().is_ok() {}
                            if let Some(player) = timer_type.player() {
                                let mut battle = battle_arc.lock().await;
                                if battle.battle.ready_to_continue().unwrap_or(false)
                                    || !battle.battle.active_requests().any(|(p, _)| p == player)
                                {
                                    break false;
                                }
                            }
                        }
                    }
                }
                result = cancel_timers_rx.recv() => {
                    // The timer was canceled, likely because the battle continued.
                    match result {
                        Ok(()) | Err(broadcast::error::RecvError::Closed) | Err(broadcast::error::RecvError::Lagged(_)) => {
                            break false;
                        }
                    }
                }
            }
        };

        recalculate_remaining(&mut now, &mut remaining);

        if !proceed {
            let mut battle = battle_arc.lock().await;
            let log_timer_deadlines = battle.log_timer_deadlines;
            battle.inject_log_entries([LiveBattle::timer_log(
                timer_type,
                remaining,
                Some(LiveBattle::log_type_for_inactive(timer_type)),
                log_timer_deadlines,
            )]);
        }

        Self::flush_battle_logs(&battle_arc).await;

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
        log::trace!("Shutting down live battle {}", self.uuid);
        {
            let mut battle = self.live_battle.lock().await;
            battle.canceled = true;
        }
        let (mut proceed_tasks, mut current_timer_tasks) = {
            let mut tasks = self.tasks.lock().await;
            tasks.abort_all();
            (
                std::mem::take(&mut tasks.proceed_tasks),
                std::mem::take(&mut tasks.current_timer_tasks),
            )
        };
        let _ = tokio::time::timeout(Duration::from_secs(2), async {
            while proceed_tasks.join_next().await.is_some() {}
            while current_timer_tasks.join_next().await.is_some() {}
        })
        .await;
    }

    fn cancel(&self) {
        log::trace!("Canceling live battle {}", self.uuid);

        if let Some(mut task_tx) = self.task_tx.try_lock() {
            task_tx.take();
        }

        if let Some(mut tasks) = self.tasks.try_lock() {
            tasks.abort_all();
            tasks.proceed_tasks.detach_all();
            tasks.current_timer_tasks.detach_all();
        }

        if let Some(mut battle) = self.live_battle.try_lock() {
            battle.canceled = true;
        }
    }
}

impl Drop for LiveBattleManager<'_> {
    fn drop(&mut self) {
        log::trace!("Dropping live battle {}", self.uuid);

        // Ensure all tasks are canceled.
        self.cancel();
    }
}

#[derive(Default)]
struct BattlerServiceState<'d> {
    battles: BTreeMap<Uuid, Arc<LiveBattleManager<'d>>>,
    battles_by_player: HashMap<String, BTreeSet<Uuid>>,
}

/// Service for managing multiple battles on the [`battler`] battle engine.
pub struct BattlerService<'d> {
    data: &'d dyn DataStore,

    state: Mutex<BattlerServiceState<'d>>,

    global_log_tx: mpsc::Sender<GlobalLogEntry>,
    global_log_rx: Option<mpsc::Receiver<GlobalLogEntry>>,
}

impl<'d> BattlerService<'d> {
    /// Creates a new battle service.
    pub fn new(data: &'d dyn DataStore) -> Self {
        let (global_log_tx, global_log_rx) = mpsc::channel(10000);
        Self {
            data,
            state: Mutex::new(BattlerServiceState::default()),
            global_log_tx,
            global_log_rx: Some(global_log_rx),
        }
    }

    /// Takes the global log receiver.
    ///
    /// All log entries for all battles will be sent over this channel for consumption.
    ///
    /// This method can only be called once. Subsequent calls will return [`None`].
    pub fn take_global_log_rx(&mut self) -> Option<mpsc::Receiver<GlobalLogEntry>> {
        self.global_log_rx.take()
    }

    async fn find_battle(&self, uuid: Uuid) -> Option<Arc<LiveBattleManager<'d>>> {
        self.state.lock().await.battles.get(&uuid).cloned()
    }

    async fn find_battle_or_error(
        &self,
        uuid: Uuid,
    ) -> Result<Arc<LiveBattleManager<'d>>, BattleError> {
        self.find_battle(uuid).await.ok_or(BattleError::NotFound)
    }

    /// Generates the status of an existing battle.
    pub async fn battle(&self, battle: Uuid) -> Result<Battle> {
        let battle = self.find_battle_or_error(battle).await?;
        Ok(battle.battle().await)
    }

    /// Gets the player IDs for a given side in a battle without locking the live battle.
    pub async fn side_players(&self, uuid: Uuid, side: Option<usize>) -> Option<HashSet<String>> {
        let battle = self.find_battle(uuid).await?;
        battle.side_players(side)
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
        {
            let mut state = self.state.lock().await;
            state.battles.insert(uuid, Arc::new(battle));

            for player in players {
                state
                    .battles_by_player
                    .entry(player)
                    .or_default()
                    .insert(uuid);
            }
        }

        log::info!("Created battle {uuid}");

        self.battle(uuid).await
    }

    /// Updates a player's team for a battle.
    pub async fn update_team(&self, battle: Uuid, player: &str, team: TeamData) -> Result<()> {
        log::info!("Updating team for {player} in battle {battle}");
        let battle = self.find_battle_or_error(battle).await?;
        battle.update_team(player, team).await
    }

    /// Starts a battle.
    pub async fn start(&self, battle: Uuid) -> Result<()> {
        log::info!("Starting battle {battle}");
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
        log::info!("Received choice for {player} in battle {battle}: {choice}");
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

    fn unwrap_battle(battle: Arc<LiveBattleManager<'d>>, context: &str) -> LiveBattleManager<'d> {
        match Arc::try_unwrap(battle) {
            Ok(battle) => battle,
            Err(_) => {
                panic!("battle could not be unwrapped during {context} (leaked references exist)")
            }
        }
    }

    /// Checks all active battles and drops any that are stuck according to the watchdog options.
    ///
    /// Returns the list of UUIDs of battles that were dropped along with the reason.
    pub async fn drop_stuck_battles(&self, options: &WatchdogOptions) -> Vec<(Uuid, DropReason)> {
        let battles = {
            let state = self.state.lock().await;
            state
                .battles
                .iter()
                .map(|(uuid, battle)| (*uuid, battle.clone()))
                .collect::<Vec<_>>()
        };

        let mut dropped = Vec::new();
        for (uuid, manager) in battles {
            if let Some(reason) = manager.check_stuck_condition(options).await
                && manager.drop_battle(reason.clone()).await
            {
                dropped.push((uuid, reason));
            }
        }

        dropped
    }

    /// Manually drops a battle with a reason.
    pub async fn drop_battle(&self, battle: Uuid, reason: DropReason) -> Result<()> {
        let manager = self.find_battle_or_error(battle).await?;
        manager.drop_battle(reason).await;
        Ok(())
    }

    /// Deletes a battle.
    pub async fn delete(&self, battle: Uuid) -> Result<()> {
        self.delete_internal(battle, false).await
    }

    /// Forces deletion of a battle even if it is ongoing.
    pub async fn delete_force(&self, battle: Uuid) -> Result<()> {
        self.delete_internal(battle, true).await
    }

    async fn delete_internal(&self, battle: Uuid, force: bool) -> Result<()> {
        let manager = match self.find_battle_or_error(battle).await {
            Ok(manager) => manager,
            Err(_) => return Ok(()),
        };
        if !force {
            let state = manager.battle_state().await;
            if state == BattleState::Active {
                return Err(Error::msg("cannot delete an ongoing battle"));
            }
        }
        log::info!("Deleting battle {battle}");
        let players = manager.players();

        let manager = {
            let mut state = self.state.lock().await;
            let manager = state.battles.remove(&battle);
            for player in players {
                if let std::collections::hash_map::Entry::Occupied(mut entry) =
                    state.battles_by_player.entry(player)
                {
                    entry.get_mut().remove(&battle);
                    if entry.get().is_empty() {
                        entry.remove_entry();
                    }
                }
            }
            manager
        };

        if let Some(manager) = manager {
            if force {
                manager.cancel();
            }
            manager.inject_and_flush_logs(["deleted"]).await;
            manager.shutdown().await;
        }

        Ok(())
    }

    /// Returns active battles up to the specified count.
    pub async fn battles(&self, count: usize, offset: usize) -> Vec<BattlePreview> {
        let count = count.min(100);
        let cloned_battles = {
            let state = self.state.lock().await;
            state
                .battles
                .values()
                .skip(offset)
                .take(count)
                .cloned()
                .collect::<Vec<_>>()
        };
        cloned_battles
            .into_iter()
            .map(|battle| battle.battle_preview())
            .collect()
    }

    /// Looks up battles for a player.
    pub async fn battles_for_player(
        &self,
        player: &str,
        count: usize,
        offset: usize,
    ) -> Vec<BattlePreview> {
        let count = count.min(100);
        let cloned_battles = {
            let state = self.state.lock().await;
            match state.battles_by_player.get(player) {
                Some(uuids) => uuids
                    .iter()
                    .skip(offset)
                    .take(count)
                    .filter_map(|uuid| state.battles.get(uuid).cloned())
                    .collect::<Vec<_>>(),
                None => return Vec::default(),
            }
        };
        cloned_battles
            .into_iter()
            .map(|battle| battle.battle_preview())
            .collect()
    }

    /// Cleans up finished battles that are older than the specified duration.
    pub async fn clean_up_finished_battles(&self, max_age: Duration) -> Result<Vec<Uuid>> {
        let uuids_and_battles = {
            let state = self.state.lock().await;
            state
                .battles
                .iter()
                .filter(|(_, battle)| battle.battle_preview().state == BattleState::Finished)
                .map(|(uuid, battle)| (*uuid, battle.clone()))
                .collect::<Vec<_>>()
        };
        let mut to_delete = Vec::new();
        for (uuid, battle) in uuids_and_battles {
            if let Some(finished_at) = battle.finished_at().await
                && finished_at.elapsed() >= max_age
            {
                to_delete.push(uuid);
            }
        }

        let mut deleted = Vec::new();
        for uuid in to_delete {
            if let Err(err) = self.delete(uuid).await {
                log::error!("Failed to auto-delete expired finished battle {uuid}: {err:?}");
            } else {
                deleted.push(uuid);
            }
        }
        Ok(deleted)
    }
}

impl Drop for BattlerService<'_> {
    fn drop(&mut self) {
        log::trace!("Dropping battler service");

        tokio::task::block_in_place(move || {
            let mut battles = BTreeMap::default();
            std::mem::swap(&mut battles, &mut self.state.get_mut().battles);
            for (_, battle) in battles {
                // SAFETY: Must synchronously cancel and wait for tasks to finish, so that the
                // battle does not outlive this object.
                battle.cancel();
                Self::unwrap_battle(battle, "drop after cancel");
            }
        });
    }
}
