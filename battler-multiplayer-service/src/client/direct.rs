use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::{
    BattlerMultiplayerService,
    BattlerMultiplayerServiceClient,
    ProposedBattle,
    ProposedBattleOptions,
    ProposedBattleResponse,
    ProposedBattleUpdate,
};

/// Implementation of [`BattlerMultiplayerServiceClient`] that uses the
/// [`BattlerMultiplayerService`] directly for managing proposed battles.
pub struct DirectBattlerMultiplayerServiceClient<'d> {
    service: Arc<BattlerMultiplayerService<'d>>,
}

impl<'d> DirectBattlerMultiplayerServiceClient<'d> {
    /// Creates a new client around a service object.
    pub fn new(service: Arc<BattlerMultiplayerService<'d>>) -> Self {
        Self { service }
    }
}

#[async_trait]
impl<'d> BattlerMultiplayerServiceClient for DirectBattlerMultiplayerServiceClient<'d> {
    async fn propose_battle(&self, options: ProposedBattleOptions) -> Result<ProposedBattle> {
        self.service.clone().propose_battle(options).await
    }

    async fn proposed_battles_for_player(
        &self,
        player: &str,
        count: usize,
        offset: usize,
    ) -> Result<Vec<ProposedBattle>> {
        Ok(self
            .service
            .proposed_battles_for_player(player, count, offset)
            .await)
    }

    async fn respond_to_proposed_battle(
        &self,
        proposed_battle: Uuid,
        player: &str,
        response: ProposedBattleResponse,
    ) -> Result<()> {
        self.service
            .respond_to_proposed_battle(proposed_battle, player, &response)
            .await
    }

    async fn proposed_battle_updates(
        &self,
        player: &str,
    ) -> Result<broadcast::Receiver<ProposedBattleUpdate>> {
        self.service.proposed_battle_updates(player).await
    }
}
