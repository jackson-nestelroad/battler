use std::sync::Arc;

use anyhow::{
    Context,
    Result,
};
use battler_service::Battle;
use battler_service_client::BattlerServiceClient;
use futures_util::lock::Mutex;
use tokio::{
    sync::{
        broadcast,
        watch,
    },
    task::JoinHandle,
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

#[derive(Debug)]
pub enum BattleClientEvent {
    Request(Option<battler::Request>),
    End,
    Error(String),
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
        let state = BattleState::default();
        let state = alter_battle_state(state, &log)?;

        let request = service.request(battle.uuid, &player).await?;
        let (battle_event_tx, battle_event_rx) =
            watch::channel(BattleClientEvent::Request(request));

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
    ) {
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

        // Check if the battle ended.
        let battle = self
            .service
            .battle(self.battle)
            .await
            .context("battle does not exist")?;
        if battle.state == battler_service::BattleState::Finished {
            self.battle_event_tx.send(BattleClientEvent::End)?;
            return Ok(());
        }

        // Check for a new request when we have caught up.
        if self.caught_up().await? {
            let request = self.service.request(self.battle, &self.player).await?;
            self.battle_event_tx
                .send(BattleClientEvent::Request(request))?;
        }

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
    task_handle: Mutex<Option<JoinHandle<()>>>,
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

        let task_handle = {
            // SAFETY: We only transmute the lifetime, from 'b to 'static. The Drop implementation
            // of this wrapper type ensures we cancel and wait for this task to finish. Thus, this
            // client is captured in the asynchronous task only for the life of this object.
            let client = unsafe {
                std::mem::transmute::<
                    Arc<BattlerClientInternal<'b>>,
                    Arc<BattlerClientInternal<'static>>,
                >(client.clone())
            };
            tokio::spawn(client.watch_battle(log_entry_rx, cancel_rx))
        };

        Ok(Self {
            client,
            task_handle: Mutex::new(Some(task_handle)),
        })
    }

    /// Cancels the client and stops following the battle.
    pub async fn cancel(&self) {
        self.client.cancel_tx.send(()).ok();
        if let Some(task_handle) = self.task_handle.lock().await.take() {
            task_handle.abort();
            task_handle.await.ok();
        }
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

impl<'b> Drop for BattlerClient<'b> {
    fn drop(&mut self) {
        // Ensure we stop using the client.
        tokio::task::block_in_place(move || {
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(self.cancel());
        });
    }
}
