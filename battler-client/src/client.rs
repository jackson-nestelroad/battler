use std::sync::Arc;

use anyhow::{
    Context,
    Result,
};
use battler_service_client::BattlerServiceClient;
use futures_util::lock::Mutex;
use tokio::sync::{
    broadcast,
    watch,
};
use uuid::Uuid;

use crate::{
    log::Log,
    state::{
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

/// A client for a single player in an individual battle.
pub struct BattlerClient {
    battle: Uuid,
    player: String,
    role: Role,

    log: Mutex<Log>,
    state: Mutex<BattleState>,

    service: Arc<Box<dyn BattlerServiceClient + Send + Sync>>,

    cancel_tx: broadcast::Sender<()>,
    request_tx: watch::Sender<Option<battler::Request>>,
    request_rx: watch::Receiver<Option<battler::Request>>,
    watcher_error_tx: watch::Sender<Option<String>>,
    watcher_error_rx: watch::Receiver<Option<String>>,

    task_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

impl BattlerClient {
    /// Creates a new client for a player in a battle.
    pub async fn new(
        battle: Uuid,
        player: String,
        service: Arc<Box<dyn BattlerServiceClient + Send + Sync>>,
    ) -> Result<Arc<Self>> {
        let battle = service
            .battle(battle)
            .await
            .context("battle does not exist")?;
        let role = battle
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
            .unwrap_or_else(|| Role::Spectator);

        let log_entry_rx = service.subscribe(battle.uuid, role.side()).await?;
        let (cancel_tx, cancel_rx) = broadcast::channel(1);

        let log = service.full_log(battle.uuid, role.side()).await?;
        let log = Log::new(log)?;
        let state = BattleState::default();
        let state = alter_battle_state(state, &log)?;

        let request = service.request(battle.uuid, &player).await?;

        let (request_tx, request_rx) = watch::channel(request);
        let (watcher_error_tx, watcher_error_rx) = watch::channel(None);

        let client = Self {
            battle: battle.uuid,
            player,
            role,
            log: Mutex::new(log),
            state: Mutex::new(state),
            service,
            cancel_tx,
            request_tx,
            request_rx,
            watcher_error_tx,
            watcher_error_rx,
            task_handle: Mutex::new(None),
        };
        let client = Arc::new(client);

        let task_handle = tokio::spawn(client.clone().watch_battle(log_entry_rx, cancel_rx));
        *client.task_handle.lock().await = Some(task_handle);

        Ok(client)
    }

    async fn watch_battle(
        self: Arc<Self>,
        log_entry_rx: broadcast::Receiver<battler_service::LogEntry>,
        cancel_rx: broadcast::Receiver<()>,
    ) {
        if let Err(err) = self.watch_battle_internal(log_entry_rx, cancel_rx).await {
            self.watcher_error_tx.send(Some(format!("{err:#}"))).ok();
        }
    }

    async fn watch_battle_internal(
        &self,
        mut log_entry_rx: broadcast::Receiver<battler_service::LogEntry>,
        mut cancel_rx: broadcast::Receiver<()>,
    ) -> Result<()> {
        loop {
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

    async fn process_log_entry(&self, log_entry: battler_service::LogEntry) -> Result<()> {
        self.update_battle_state(log_entry).await?;

        // Check for a new request.
        let request = self.service.request(self.battle, &self.player).await?;
        self.request_tx.send(request)?;

        Ok(())
    }

    async fn update_battle_state(&self, log_entry: battler_service::LogEntry) -> Result<()> {
        let mut log = self.log.lock().await;
        log.add(log_entry.index, log_entry.content)?;

        // If the log is not filled, we must backfill the log.
        if !log.filled() {
            let full_log = self.service.full_log(self.battle, self.role.side()).await?;
            for (i, entry) in full_log.into_iter().enumerate() {
                log.add(i, entry)?;
            }
        }

        // Update the battle state.
        let mut state = self.state.lock().await;
        let mut new_state = BattleState::default();
        std::mem::swap(&mut new_state, &mut *state);
        *state = alter_battle_state(new_state, &*log)?;

        Ok(())
    }

    /// Cancels the client and stops following the battle.
    pub async fn cancel(&self) -> Result<()> {
        self.cancel_tx.send(())?;
        if let Some(task_handle) = self.task_handle.lock().await.take() {
            task_handle.abort();
            task_handle.await?;
        }
        Ok(())
    }

    /// Checks if the player is ready for the battle to start.
    ///
    /// If not, [`Self::update_team`] should be used to prepare the player for battle.
    pub async fn ready_for_battle(&self) -> Result<battler_service::PlayerValidation> {
        self.service
            .validate_player(self.battle, &self.player)
            .await
    }

    /// Updates the player's team.
    pub async fn update_team(&self, team: battler::TeamData) -> Result<()> {
        self.service
            .update_team(self.battle, &self.player, team)
            .await
    }

    /// Starts the battle.
    pub async fn start(&self) -> Result<()> {
        self.service.start(self.battle).await
    }

    /// Sets the next choice for the player in battle.
    pub async fn make_choice(&self, choice: &str) -> Result<()> {
        self.service
            .make_choice(self.battle, &self.player, choice)
            .await
    }

    /// Receiver for requests for the player.
    pub async fn request_rx(&self) -> watch::Receiver<Option<battler::Request>> {
        self.request_rx.clone()
    }

    /// Receiver for errors that occur in the watcher task.
    pub async fn watcher_error_rx(&self) -> watch::Receiver<Option<String>> {
        self.watcher_error_rx.clone()
    }

    /// The battle state.
    pub async fn state(&self) -> BattleState {
        self.state.lock().await.clone()
    }
}
