use std::sync::Arc;

use anyhow::{
    Error,
    Result,
};
use battler_client::BattlerClient;
use battler_multiplayer_service::{
    ProposedBattle,
    ProposedBattleOptions,
    ProposedBattleResponse,
    ProposedBattleUpdate,
};
use battler_multiplayer_service_client::BattlerMultiplayerServiceClient;
use battler_service_client::BattlerServiceClient;
use tokio::sync::broadcast;
use uuid::Uuid;

/// A high-level client for a single player managing matchmaking and battle transition.
pub struct BattlerMultiplayerClient<'b> {
    player: String,
    battler_multiplayer_service_client: Arc<Box<dyn BattlerMultiplayerServiceClient>>,
    battler_service_client: Arc<Box<dyn BattlerServiceClient + 'b>>,
}

impl<'b> BattlerMultiplayerClient<'b> {
    /// Creates a new multiplayer client instance for a player.
    pub fn new(
        player: String,
        battler_multiplayer_service_client: Arc<Box<dyn BattlerMultiplayerServiceClient>>,
        battler_service_client: Arc<Box<dyn BattlerServiceClient + 'b>>,
    ) -> Self {
        Self {
            player,
            battler_multiplayer_service_client,
            battler_service_client,
        }
    }

    /// The player's ID associated with this client.
    pub fn player(&self) -> &str {
        &self.player
    }

    /// Proposes a new battle.
    pub async fn propose_battle(&self, options: ProposedBattleOptions) -> Result<ProposedBattle> {
        self.battler_multiplayer_service_client
            .propose_battle(options)
            .await
    }

    /// Responds to a proposed battle invitation.
    pub async fn respond_to_proposal(
        &self,
        proposed_battle: Uuid,
        accept: bool,
    ) -> Result<ProposedBattle> {
        self.battler_multiplayer_service_client
            .respond_to_proposed_battle(
                proposed_battle,
                &self.player,
                ProposedBattleResponse { accept },
            )
            .await
    }

    /// Lists active proposals for this player.
    pub async fn proposed_battles(
        &self,
        count: usize,
        offset: usize,
    ) -> Result<Vec<ProposedBattle>> {
        self.battler_multiplayer_service_client
            .proposed_battles_for_player(&self.player, count, offset)
            .await
    }

    /// Subscribes to matchmaking and invitation updates for this player.
    pub async fn proposed_battle_updates(
        &self,
    ) -> Result<broadcast::Receiver<ProposedBattleUpdate>> {
        self.battler_multiplayer_service_client
            .proposed_battle_updates(&self.player)
            .await
    }

    /// Helper to block and wait for a specific proposed battle to start.
    ///
    /// Returns the battle UUID once accepted by all players.
    pub async fn wait_for_battle_start(
        &self,
        proposed_battle: Uuid,
        rx: &mut broadcast::Receiver<ProposedBattleUpdate>,
    ) -> Result<Uuid> {
        loop {
            let update = rx.recv().await?;
            if update.proposed_battle.uuid == proposed_battle {
                if let Some(battle_uuid) = update.proposed_battle.battle {
                    return Ok(battle_uuid);
                }
                if update.rejection.is_some() || update.deletion_reason.is_some() {
                    return Err(Error::msg(
                        "proposed battle proposal was rejected or cancelled",
                    ));
                }
            }
        }
    }

    /// Instantiates a [`BattlerClient`] to play a started battle.
    pub async fn create_battler_client(&self, battle_uuid: Uuid) -> Result<BattlerClient<'b>> {
        BattlerClient::new(
            battle_uuid,
            self.player.clone(),
            self.battler_service_client.clone(),
        )
        .await
    }

    /// Proposes a battle, waits for matchmaking to accept and start it,
    /// and returns the instantiated [`BattlerClient`].
    pub async fn propose_and_wait_for_battle_start(
        &self,
        options: ProposedBattleOptions,
    ) -> Result<BattlerClient<'b>> {
        let mut rx = self.proposed_battle_updates().await?;
        let proposed = self.propose_battle(options).await?;
        let battle_uuid = self.wait_for_battle_start(proposed.uuid, &mut rx).await?;
        self.create_battler_client(battle_uuid).await
    }
}
