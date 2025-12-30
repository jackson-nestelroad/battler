use std::sync::Arc;

use anyhow::{
    Context,
    Error,
    Result,
};
use battler_service::Battle;
use battler_service_client::BattlerServiceClient;
use futures_util::lock::Mutex;
use thiserror::Error;
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
    log::Log,
    state::{
        BattlePhase,
        BattleState,
        alter_battle_state,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
enum Role {
    Spectator,
    Player { side: usize },
}

impl Role {
    fn side(&self) -> Option<usize> {
        match self {
            Self::Spectator => None,
            Self::Player { side } => Some(*side),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattleClientEvent {
    Request(Option<battler::Request>),
    End,
    Error(String),
}

impl BattleClientEvent {
    /// Checks if the event is a non-empty request that must be responded to.
    pub fn is_request(&self) -> bool {
        match self {
            Self::Request(Some(_)) => true,
            _ => false,
        }
    }
}

fn role_for_player(battle: &Battle, player: &str) -> Role {
    battle
        .sides
        .iter()
        .enumerate()
        .find(|(_, side)| {
            side.players
                .iter()
                .find(|battle_player| battle_player.id == player)
                .is_some()
        })
        .map(|(i, _)| Role::Player { side: i })
        .unwrap_or_else(|| Role::Spectator)
}

/// Error signaling the battle ended.
#[derive(Error, Debug, Default)]
#[error("battle ended")]
pub struct BattleEndedError;

struct BattlerClientInternal<'b> {
    battle: Uuid,
    player: String,
    role: Role,

    log: Mutex<Log>,
    state: Mutex<BattleState>,

    service: Arc<Box<dyn BattlerServiceClient + 'b>>,

    cancel_tx: broadcast::Sender<()>,
    battle_event_tx: watch::Sender<BattleClientEvent>,
    battle_event_rx: watch::Receiver<BattleClientEvent>,
}

impl<'b> BattlerClientInternal<'b> {
    async fn new(
        battle: Uuid,
        player: String,
        service: Arc<Box<dyn BattlerServiceClient + 'b>>,
    ) -> Result<Self> {
        let battle = service
            .battle(battle)
            .await
            .context("battle does not exist")?;
        let role = role_for_player(&battle, &player);

        let (cancel_tx, _) = broadcast::channel(1);

        let log = service.full_log(battle.uuid, role.side()).await?;
        let log = Log::new(log)?;

        // Start with an empty battle state and request.
        //
        // As soon as we start watching for new logs, we will backfill the state and request.
        let state = BattleState::default();

        let (battle_event_tx, battle_event_rx) = watch::channel(BattleClientEvent::Request(None));

        let client = Self {
            battle: battle.uuid,
            player,
            role,
            log: Mutex::new(log),
            state: Mutex::new(state),
            service,
            cancel_tx,
            battle_event_tx,
            battle_event_rx,
        };

        Ok(client)
    }

    async fn watch_battle(
        self: Arc<Self>,
        log_entry_rx: broadcast::Receiver<battler_service::LogEntry>,
        cancel_rx: broadcast::Receiver<()>,
        task_tx: mpsc::WeakSender<()>,
    ) {
        #[allow(unused)]
        let task_tx = match task_tx.upgrade() {
            Some(task_tx) => task_tx,
            None => return,
        };
        log::info!(
            "Starting to watch battle {} for {}",
            self.battle,
            self.player
        );
        if let Err(err) = self.watch_battle_internal(log_entry_rx, cancel_rx).await {
            self.battle_event_tx
                .send(BattleClientEvent::Error(format!("{err:#}")))
                .ok();
        }
    }

    async fn watch_battle_internal(
        &self,
        mut log_entry_rx: broadcast::Receiver<battler_service::LogEntry>,
        mut cancel_rx: broadcast::Receiver<()>,
    ) -> Result<()> {
        // Ensure the log and battle state are caught up.
        self.ensure_caught_up().await?;

        loop {
            if *self.battle_event_rx.borrow() == BattleClientEvent::End {
                break;
            }
            // If we are caught up, propagate a request.
            if self.caught_up().await? && self.last_log_index().await > 0 {
                let request = self.service.request(self.battle, &self.player).await?;
                log::debug!(
                    "Propagating request for {} in battle {}: has_request = {:?}",
                    self.player,
                    self.battle,
                    request.is_some()
                );
                self.battle_event_tx
                    .send(BattleClientEvent::Request(request))?;
            }

            tokio::select! {
                log_entry = log_entry_rx.recv() => {
                    self.process_log_entry(log_entry?).await?;
                }
                _ = cancel_rx.recv() => {
                    break;
                }
            }
        }
        Ok(())
    }

    async fn ensure_caught_up(&self) -> Result<()> {
        let mut log = self.log.lock().await;
        // Ensure the log is filled and update the state accordingly.
        self.backfill_log(&mut log).await?;
        self.update_battle_state(&log).await?;
        Ok(())
    }

    async fn process_log_entry(&self, log_entry: battler_service::LogEntry) -> Result<()> {
        log::trace!(
            "Processing log entry {log_entry:?} for {} in battle {}",
            self.player,
            self.battle
        );
        self.update_battle_state_for_log_entry(log_entry).await?;

        // Check if the battle ended.
        if self.state.lock().await.phase == BattlePhase::Finished {
            log::info!(
                "Client {} in battle {} detected battle finished",
                self.player,
                self.battle
            );
            self.battle_event_tx.send(BattleClientEvent::End)?;
            return Ok(());
        }

        Ok(())
    }

    async fn backfill_log(&self, log: &mut Log) -> Result<()> {
        let full_log = self.service.full_log(self.battle, self.role.side()).await?;
        for (i, entry) in full_log.into_iter().enumerate() {
            log.add(i, entry)?;
        }
        log::info!(
            "Battle log for {} in battle {} was backfilled to {} entries",
            self.player,
            self.battle,
            log.len()
        );
        Ok(())
    }

    async fn update_battle_state_for_log_entry(
        &self,
        log_entry: battler_service::LogEntry,
    ) -> Result<()> {
        let mut log = self.log.lock().await;
        log.add(log_entry.index, log_entry.content)?;

        // If the log is not filled, we must backfill the log.
        if !log.filled() {
            self.backfill_log(&mut *log).await?;
        }

        self.update_battle_state(&log).await
    }

    async fn update_battle_state(&self, log: &Log) -> Result<()> {
        let mut state = self.state.lock().await;
        let mut new_state = BattleState::default();
        std::mem::swap(&mut new_state, &mut state);
        *state = alter_battle_state(new_state, &*log)?;

        Ok(())
    }

    async fn ready_for_battle(&self) -> Result<battler_service::PlayerValidation> {
        self.service
            .validate_player(self.battle, &self.player)
            .await
    }

    async fn update_team(&self, team: battler::TeamData) -> Result<()> {
        self.service
            .update_team(self.battle, &self.player, team)
            .await
    }

    async fn start(&self) -> Result<()> {
        self.service.start(self.battle).await
    }

    async fn make_choice(&self, choice: &str) -> Result<()> {
        self.service
            .make_choice(self.battle, &self.player, choice)
            .await
    }

    async fn player_data(&self) -> Result<battler::PlayerBattleData> {
        self.service.player_data(self.battle, &self.player).await
    }

    fn battle_event_rx(&self) -> watch::Receiver<BattleClientEvent> {
        self.battle_event_rx.clone()
    }

    async fn state(&self) -> BattleState {
        self.state.lock().await.clone()
    }

    async fn last_log_index(&self) -> usize {
        self.state.lock().await.last_log_index
    }

    async fn caught_up(&self) -> Result<bool> {
        let last_log_index = self.last_log_index().await;
        let last_possible_log_index = self
            .service
            .last_log_entry(self.battle, self.role.side())
            .await?
            .map(|(i, _)| i)
            .unwrap_or_default();
        Ok(last_log_index == last_possible_log_index)
    }
}

/// A client for a single player in an individual battle.
pub struct BattlerClient<'b> {
    client: Arc<BattlerClientInternal<'b>>,
    watch_tasks: Mutex<JoinSet<()>>,
    task_tx: Option<mpsc::Sender<()>>,
    task_rx: Mutex<mpsc::Receiver<()>>,
}

impl<'b> BattlerClient<'b> {
    /// Creates a new client for a player in a battle.
    pub async fn new(
        battle: Uuid,
        player: String,
        service: Arc<Box<dyn BattlerServiceClient + 'b>>,
    ) -> Result<Self> {
        let client = Arc::new(BattlerClientInternal::new(battle, player, service).await?);

        let log_entry_rx = client
            .service
            .subscribe(client.battle, client.role.side())
            .await?;
        let cancel_rx = client.cancel_tx.subscribe();

        let (task_tx, task_rx) = mpsc::channel(1);

        let mut watch_tasks = JoinSet::default();
        {
            // SAFETY: We only transmute the lifetime, from 'b to 'static. The Drop implementation
            // of this wrapper type ensures we cancel and wait for this task to finish. Thus, this
            // client is captured in the asynchronous task only for the life of this object.
            let client = unsafe {
                std::mem::transmute::<
                    Arc<BattlerClientInternal<'b>>,
                    Arc<BattlerClientInternal<'static>>,
                >(client.clone())
            };
            watch_tasks.spawn(client.watch_battle(log_entry_rx, cancel_rx, task_tx.downgrade()))
        };

        Ok(Self {
            client,
            watch_tasks: Mutex::new(watch_tasks),
            task_tx: Some(task_tx),
            task_rx: Mutex::new(task_rx),
        })
    }

    /// The battle UUID.
    pub fn battle(&self) -> Uuid {
        self.client.battle
    }

    /// The player ID.
    pub fn player(&self) -> String {
        self.client.player.clone()
    }

    /// Cancels the client and stops following the battle.
    pub async fn cancel(&self) {
        self.client.cancel_tx.send(()).ok();
        self.watch_tasks.lock().await.shutdown().await;
    }

    /// Checks if the player is ready for the battle to start.
    ///
    /// If not, [`Self::update_team`] should be used to prepare the player for battle.
    pub async fn ready_for_battle(&self) -> Result<battler_service::PlayerValidation> {
        self.client.ready_for_battle().await
    }

    /// Updates the player's team.
    pub async fn update_team(&self, team: battler::TeamData) -> Result<()> {
        self.client.update_team(team).await
    }

    /// Starts the battle.
    pub async fn start(&self) -> Result<()> {
        self.client.start().await
    }

    /// Sets the next choice for the player in battle.
    pub async fn make_choice(&self, choice: &str) -> Result<()> {
        self.client.make_choice(choice).await
    }

    /// Reads the player's current battle data.
    pub async fn player_data(&self) -> Result<battler::PlayerBattleData> {
        self.client.player_data().await
    }

    /// Receiver for battle events for the player.
    pub fn battle_event_rx(&self) -> watch::Receiver<BattleClientEvent> {
        self.client.battle_event_rx()
    }

    /// Waits for a new request to be available, failing if an error is encountered or if the battle
    /// ends.
    ///
    /// If the battle ended, [`BattleEndedError`] is returned as the error.
    pub async fn wait_for_request(
        battle_event_rx: &mut watch::Receiver<BattleClientEvent>,
    ) -> Result<battler::Request> {
        loop {
            battle_event_rx.changed().await?;
            // Clone because the reference returned by the receiver is not Send.
            let event = battle_event_rx.borrow_and_update().clone();
            match event {
                BattleClientEvent::Request(Some(request)) => return Ok(request),
                BattleClientEvent::Request(None) => (),
                BattleClientEvent::Error(err) => return Err(Error::msg(err)),
                BattleClientEvent::End => return Err(BattleEndedError.into()),
            }
        }
    }

    /// Waits for the battle to end, failing if an error is encountered.
    pub async fn wait_for_end(
        battle_event_rx: &mut watch::Receiver<BattleClientEvent>,
    ) -> Result<()> {
        battle_event_rx.mark_changed();
        loop {
            battle_event_rx.changed().await?;
            // Clone because the reference returned by the receiver is not Send.
            let event = battle_event_rx.borrow_and_update().clone();
            match event {
                BattleClientEvent::End => return Ok(()),
                BattleClientEvent::Error(err) => return Err(Error::msg(err)),
                _ => (),
            }
        }
    }

    /// The battle state.
    pub async fn state(&self) -> BattleState {
        self.client.state().await
    }

    /// The last log index of the current battle state.
    pub async fn last_log_index(&self) -> usize {
        self.client.last_log_index().await
    }

    /// If the client is caught up with respect to the actual battle state.
    pub async fn caught_up(&self) -> Result<bool> {
        self.client.caught_up().await
    }
}

impl Drop for BattlerClient<'_> {
    fn drop(&mut self) {
        log::trace!(
            "Dropping client {} in battle {}",
            self.player(),
            self.battle()
        );

        // Ensure we stop using the client.
        //
        // Same as `cancel`, but synchronous.
        tokio::task::block_in_place(move || {
            self.client.cancel_tx.send(()).ok();
            self.watch_tasks.get_mut().abort_all();
            self.task_tx.take();

            log::trace!(
                "Blocking on task completion for client {} in battle {}",
                self.player(),
                self.battle()
            );
            self.task_rx.get_mut().blocking_recv();
        });
    }
}
