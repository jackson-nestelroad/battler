use std::{
    marker::PhantomData,
    sync::{
        Arc,
        Weak,
    },
};

use ahash::HashSet;
use anyhow::{
    Error,
    Result,
};
use battler::DataStoreByName;
use battler_ai::{
    BattlerAi,
    gemini::Gemini,
    run_battler_ai_client,
};
use battler_client::BattlerClient;
use battler_service_client::BattlerServiceClient;
use tokio::{
    sync::{
        Mutex,
        broadcast,
        mpsc,
    },
    task::{
        JoinError,
        JoinHandle,
        JoinSet,
    },
};
use uuid::Uuid;

use crate::{
    AiPlayerOptions,
    AiPlayerType,
    BattlerMultiplayerServiceClient,
    ProposedBattle,
    ProposedBattleResponse,
    ProposedBattleUpdate,
};

/// A handle to an asynchronously-running [`AiPlayer`].
#[derive(Debug)]
pub struct AiPlayerHandle<'d> {
    join_handle: Option<JoinHandle<()>>,
    cancel_tx: broadcast::Sender<()>,
    error_rx: broadcast::Receiver<String>,

    task_rx: mpsc::Receiver<()>,

    phantom: PhantomData<&'d ()>,
}

impl<'d> AiPlayerHandle<'d> {
    /// Cancels the task.
    pub fn cancel(&self) -> Result<()> {
        self.cancel_tx.send(()).map(|_| ()).map_err(Error::new)
    }

    /// Joins the task.
    #[allow(unused)]
    pub async fn join(mut self) -> Result<(), JoinError> {
        // SAFETY: `join_handle` is set
        self.join_handle.take().unwrap().await
    }

    /// The error receiver channel.
    pub fn error_rx(&self) -> broadcast::Receiver<String> {
        self.error_rx.resubscribe()
    }
}

impl Drop for AiPlayerHandle<'_> {
    fn drop(&mut self) {
        self.cancel().ok();
        tokio::task::block_in_place(move || {
            // Abort the task.
            if let Some(join_handle) = self.join_handle.take() {
                join_handle.abort();
            }

            // Wait for all Senders to be blocked. The task in the above JoinSet holds the only
            // Sender.
            self.task_rx.blocking_recv();
        });
    }
}

/// Modules for creating an [`AiPlayer`].
#[derive(Clone)]
pub struct AiPlayerModules<'d> {
    pub data: &'d dyn DataStoreByName,
    pub battler_service_client: Arc<Box<dyn BattlerServiceClient + 'd>>,
    pub battler_multiplayer_service_client: Arc<Box<dyn BattlerMultiplayerServiceClient + 'd>>,
}

#[derive(Default)]
pub struct AiPlayerState {
    watched_battles: HashSet<Uuid>,
}

/// An AI-controlled player.
///
/// Watches proposed battles and battles. All proposed battles are immediately accepted. The
/// underlying [`BattlerAi`] is used to make decisions in active battles.
pub struct AiPlayer<'d> {
    options: AiPlayerOptions,
    data: &'d dyn DataStoreByName,
    battler_service_client: Arc<Box<dyn BattlerServiceClient + 'd>>,
    battler_multiplayer_service_client: Arc<Box<dyn BattlerMultiplayerServiceClient + 'd>>,

    battle_tasks: Mutex<JoinSet<()>>,
    state: Arc<Mutex<AiPlayerState>>,

    error_tx: broadcast::Sender<String>,

    task_tx: Option<mpsc::Sender<()>>,
    task_rx: mpsc::Receiver<()>,
}

impl<'d> AiPlayer<'d> {
    /// Creates a new player.
    pub fn new(options: AiPlayerOptions, modules: AiPlayerModules<'d>) -> Self {
        let (error_tx, _) = broadcast::channel(16);
        let (task_tx, task_rx) = mpsc::channel(1);
        Self {
            options,
            data: modules.data,
            battler_service_client: modules.battler_service_client,
            battler_multiplayer_service_client: modules.battler_multiplayer_service_client,
            battle_tasks: Mutex::new(JoinSet::default()),
            state: Arc::new(Mutex::new(AiPlayerState::default())),
            error_tx,
            task_tx: Some(task_tx),
            task_rx,
        }
    }

    /// Starts the AI player asynchronously.
    ///
    /// The asynchronous task takes ownership of this object. Callers can control the player with
    /// the returned handle.
    pub async fn start(self) -> Result<AiPlayerHandle<'d>> {
        let (cancel_tx, cancel_rx) = broadcast::channel(16);
        let error_rx = self.error_tx.subscribe();
        let (task_tx, task_rx) = mpsc::channel(1);

        // SAFETY: AiPlayerHandle takes the lifetime of 'd, so that it naturally cannot exceed the
        // lifetime of this object. The Drop implementation of AiPlayerHandle blocks on this task
        // finishing. Since this task takes ownership of this object, this object cannot outlive the
        // lifetime of the corresponding handle and the lifetime of 'd.
        let ai_player = unsafe { std::mem::transmute::<Self, AiPlayer<'static>>(self) };
        let join_handle = tokio::spawn(ai_player.run(cancel_rx, task_tx));

        Ok(AiPlayerHandle {
            join_handle: Some(join_handle),
            cancel_tx,
            error_rx,
            task_rx,
            phantom: PhantomData,
        })
    }

    async fn run(
        mut self,
        mut cancel_rx: broadcast::Receiver<()>,
        #[allow(unused)] task_tx: mpsc::Sender<()>,
    ) {
        self.run_internal(&mut cancel_rx).await;
        self.battle_tasks.get_mut().shutdown().await;
    }

    async fn run_internal(&mut self, cancel_rx: &mut broadcast::Receiver<()>) {
        self.handle_proposed_battles(cancel_rx).await
    }

    async fn handle_proposed_battles(&mut self, cancel_rx: &mut broadcast::Receiver<()>) {
        let (proposed_battle_task_cancel_tx, _) = broadcast::channel(1);
        // SAFETY: The future is awaited after cancellation.
        //
        // If the task is aborted and everything is dropped, the cancel channel created above will
        // be dropped, which should trigger all tasks to exit as soon as possible.
        let future = unsafe {
            async_scoped::TokioScope::scope_and_collect(|scope| {
                for player in &self.options.players {
                    scope.spawn_cancellable(
                        self.handle_proposed_battles_for_player(
                            player.clone(),
                            proposed_battle_task_cancel_tx.subscribe(),
                        ),
                        || (),
                    );
                }
            })
        };
        // Wait for the global cancellation before notifying that each task must be canceled.
        //
        // We must use separate channels because resubscribing to a channel causes previous messages
        // to be lost. This way, a cancellation sent before the scoped tasks are spawned is not
        // lost.
        cancel_rx.recv().await.ok();
        proposed_battle_task_cancel_tx.send(()).ok();
        future.await;
    }

    async fn handle_proposed_battles_for_player(
        &self,
        player: String,
        mut cancel_rx: broadcast::Receiver<()>,
    ) {
        while let Err(err) = self
            .handle_proposed_battles_for_player_internal(player.clone(), &mut cancel_rx)
            .await
        {
            self.error_tx
                .send(format!(
                    "Error handling proposed battles for {player}: {err}"
                ))
                .ok();
        }
    }

    async fn handle_proposed_battles_for_player_internal(
        &self,
        player: String,
        cancel_rx: &mut broadcast::Receiver<()>,
    ) -> Result<()> {
        let proposed_battle_update_rx = self
            .battler_multiplayer_service_client
            .proposed_battle_updates(&player)
            .await?;
        self.handle_existing_battles(&player).await?;
        self.handle_existing_proposed_battles(&player).await?;
        self.watch_proposed_battle_updates(&player, proposed_battle_update_rx, cancel_rx)
            .await
    }

    async fn handle_existing_battles(&self, player: &str) -> Result<()> {
        const COUNT: usize = 100;
        let mut offset = 0;
        loop {
            let battles = self
                .battler_service_client
                .battles_for_player(player, COUNT, offset)
                .await?;
            if battles.is_empty() {
                break;
            }
            offset += battles.len();
            for battle in battles {
                self.handle_battle(player, battle.uuid).await;
            }
        }
        Ok(())
    }

    async fn handle_existing_proposed_battles(&self, player: &str) -> Result<()> {
        const COUNT: usize = 100;
        let mut offset = 0;
        loop {
            let proposed_battles = self
                .battler_multiplayer_service_client
                .proposed_battles_for_player(player, COUNT, offset)
                .await?;
            if proposed_battles.is_empty() {
                break;
            }
            offset += proposed_battles.len();
            for proposed_battle in proposed_battles {
                self.respond_to_proposed_battle(player, &proposed_battle)
                    .await?;
            }
        }
        Ok(())
    }

    async fn respond_to_proposed_battle(
        &self,
        player: &str,
        proposed_battle: &ProposedBattle,
    ) -> Result<()> {
        let player = match proposed_battle
            .sides
            .iter()
            .flat_map(|side| side.players.iter())
            .find(|p| p.id == player)
        {
            Some(player) => player,
            None => return Ok(()),
        };
        if player.status.is_some() {
            return Ok(());
        }

        self.battler_multiplayer_service_client
            .respond_to_proposed_battle(
                proposed_battle.uuid,
                &player.id,
                ProposedBattleResponse { accept: true },
            )
            .await
    }

    async fn watch_proposed_battle_updates(
        &self,
        player: &str,
        mut proposed_battle_update_rx: broadcast::Receiver<ProposedBattleUpdate>,
        cancel_rx: &mut broadcast::Receiver<()>,
    ) -> Result<()> {
        loop {
            tokio::select! {
                update = proposed_battle_update_rx.recv() => {
                    self.handle_proposed_battle_update(player, &update?).await?;
                }
                _ = cancel_rx.recv() => {
                    return Ok(());
                }
            }
        }
    }

    async fn handle_proposed_battle_update(
        &self,
        player: &str,
        update: &ProposedBattleUpdate,
    ) -> Result<()> {
        self.respond_to_proposed_battle(player, &update.proposed_battle)
            .await?;
        if let Some(battle) = update.battle {
            self.handle_battle(player, battle).await;
        }
        Ok(())
    }

    async fn handle_battle(&self, player: &str, battle: Uuid) {
        if !self.state.lock().await.watched_battles.insert(battle) {
            return;
        }
        // SAFETY: The `Drop` implementation of this type ensures that all battle tasks finish,
        // so these objects are not used beyond the lifetime of this object.
        let data = unsafe {
            std::mem::transmute::<&'d dyn DataStoreByName, &'static dyn DataStoreByName>(self.data)
        };
        let battler_service_client = unsafe {
            std::mem::transmute::<
                Arc<Box<dyn BattlerServiceClient + 'd>>,
                Arc<Box<dyn BattlerServiceClient + 'static>>,
            >(self.battler_service_client.clone())
        };
        self.battle_tasks.lock().await.spawn(AiPlayer::watch_battle(
            battle,
            player.to_owned(),
            battler_service_client,
            data,
            self.options.clone(),
            Arc::downgrade(&self.state),
            self.error_tx.clone(),
            // SAFETY: task_tx is None only when dropping this object, which cannot happen in
            // parallel because this method takes requires a mutable borrow.
            self.task_tx.clone().unwrap(),
        ));
    }

    fn create_ai(options: &AiPlayerOptions) -> Box<dyn BattlerAi> {
        match options.ai_type {
            AiPlayerType::Gemini(_) => Box::new(Gemini::default()),
        }
    }

    async fn watch_battle(
        battle: Uuid,
        player: String,
        service: Arc<Box<dyn BattlerServiceClient + 'd>>,
        data: &'d dyn DataStoreByName,
        options: AiPlayerOptions,
        state: Weak<Mutex<AiPlayerState>>,
        error_tx: broadcast::Sender<String>,
        #[allow(unused)] task_tx: mpsc::Sender<()>,
    ) {
        while let Err(err) = Self::watch_battle_internal(
            battle,
            player.clone(),
            service.clone(),
            data,
            Self::create_ai(&options),
        )
        .await
        {
            error_tx
                .send(format!(
                    "Error watching battle {battle} for {player}: {err}"
                ))
                .ok();
        }
        if let Some(state) = state.upgrade() {
            state.lock().await.watched_battles.remove(&battle);
        }
    }

    async fn watch_battle_internal(
        battle: Uuid,
        player: String,
        service: Arc<Box<dyn BattlerServiceClient + 'd>>,
        data: &'d dyn DataStoreByName,
        ai: Box<dyn BattlerAi>,
    ) -> Result<()> {
        let client = BattlerClient::new(battle, player, service).await?;
        run_battler_ai_client(data, client, ai).await
    }
}

impl Drop for AiPlayer<'_> {
    fn drop(&mut self) {
        // SAFETY: Ensure all battle tasks finish, so they do not extend beyond the lifetime of this
        // object and the lifetime of 'd.
        tokio::task::block_in_place(move || {
            // Abort all tasks.
            self.battle_tasks.get_mut().abort_all();

            // Drop our Sender.
            self.task_tx.take();

            // Wait for all Senders to be dropped. One Sender is given to each task, so the Receiver
            // closing means all tasks are finished.
            self.task_rx.blocking_recv();
        });
    }
}
