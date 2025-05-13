use std::sync::Arc;

use anyhow::{
    Context,
    Result,
};
use battler_service_client::BattlerServiceClient;
use futures_util::lock::Mutex;
use tokio::sync::broadcast;
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

#[derive(Debug, Default, Clone)]
pub struct WatcherState {
    pub error: Option<String>,
}

pub struct BattlerClient {
    battle: Uuid,
    player: String,
    role: Role,

    log: Mutex<Log>,
    state: Mutex<BattleState>,

    service: Arc<Box<dyn BattlerServiceClient + Send + Sync>>,

    cancel_tx: broadcast::Sender<()>,

    task_handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
    watcher_state: Mutex<WatcherState>,
}

impl BattlerClient {
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

        let client = Self {
            battle: battle.uuid,
            player,
            role,
            log: Mutex::new(log),
            state: Mutex::new(state),
            service,
            cancel_tx,
            task_handle: Mutex::new(None),
            watcher_state: Mutex::new(WatcherState::default()),
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
        if let Err(err) = self
            .clone()
            .watch_battle_internal(log_entry_rx, cancel_rx)
            .await
        {
            self.watcher_state.lock().await.error = Some(format!("{err:#}"));
        }
    }

    async fn watch_battle_internal(
        self: Arc<Self>,
        mut log_entry_rx: broadcast::Receiver<battler_service::LogEntry>,
        mut cancel_rx: broadcast::Receiver<()>,
    ) -> Result<()> {
        loop {
            tokio::select! {
                log_entry = log_entry_rx.recv() => {
                    self.clone().process_log_entry(log_entry?).await?;
                }
                _ = cancel_rx.recv() => {
                    break;
                }
            }
        }
        Ok(())
    }

    async fn process_log_entry(
        self: Arc<Self>,
        log_entry: battler_service::LogEntry,
    ) -> Result<()> {
        let mut log = self.log.lock().await;
        log.add(log_entry.index, log_entry.content)?;

        // If the log is not filled, we must backfill the log.
        if !log.filled() {
            let full_log = self.service.full_log(self.battle, self.role.side()).await?;
            for (i, entry) in full_log.into_iter().enumerate() {
                log.add(i, entry)?;
            }
        }

        let mut state = self.state.lock().await;
        let mut new_state = BattleState::default();
        std::mem::swap(&mut new_state, &mut *state);
        *state = alter_battle_state(new_state, &*log)?;

        Ok(())
    }

    pub async fn stop(self: Arc<Self>) -> Result<()> {
        self.cancel_tx.send(())?;
        if let Some(task_handle) = self.task_handle.lock().await.take() {
            task_handle.abort();
            task_handle.await?;
        }
        Ok(())
    }
}
