use std::sync::Arc;

use anyhow::Result;
use battler::CoreBattleOptions;
use battler_service::BattlePreview;
use battler_service_client::BattlerServiceClient;
use uuid::Uuid;

use crate::BattlerClient;

/// Wrapper around a [`BattlerServiceClient`] for a single player to manage battles they are
/// participating in.
pub struct BattlerClientManager {
    player: String,
    service: Arc<Box<dyn BattlerServiceClient + Send + Sync>>,
}

impl BattlerClientManager {
    /// Creates a new manager.
    pub fn new(player: String, service: Arc<Box<dyn BattlerServiceClient + Send + Sync>>) -> Self {
        Self { player, service }
    }

    /// Creates a new battle.
    pub async fn create(&self, options: CoreBattleOptions) -> Result<Uuid> {
        let battle = self.service.create(options).await?;
        Ok(battle.uuid)
    }

    /// Joins the battle, creating a new client for it.
    pub async fn join(&self, battle: Uuid) -> Result<Arc<BattlerClient>> {
        BattlerClient::new(battle, self.player.clone(), self.service.clone()).await
    }

    /// Lists battles for the player.
    pub async fn battles(&self, count: usize, offset: usize) -> Result<Vec<BattlePreview>> {
        self.service
            .battles_for_player(&self.player, count, offset)
            .await
    }
}
