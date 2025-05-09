use std::sync::Arc;

use anyhow::Result;
use battler::CoreBattleOptions;
use battler_service::BattlePreview;
use battler_service_client::BattlerServiceClient;
use uuid::Uuid;

use crate::BattlerClient;

/// Wrapper around a [`BattlerServiceClient`] for a single player to manage battles they are
/// participating in.
pub struct BattleClientManager<'a> {
    player: String,
    service: Arc<Box<dyn BattlerServiceClient + 'a>>,
}

impl<'a> BattleClientManager<'a> {
    /// Creates a new manager.
    pub fn new(player: String, service: Arc<Box<dyn BattlerServiceClient + 'a>>) -> Self {
        Self { player, service }
    }

    /// Creates a new battle.
    pub async fn create(&self, options: CoreBattleOptions) -> Result<Uuid> {
        let battle = self.service.create(options).await?;
        Ok(battle.uuid)
    }

    pub async fn join(&self, battle: Uuid) -> Result<BattlerClient<'a>> {
        let client = BattlerClient::new(battle, self.player.clone(), self.service.clone());
        todo!()
    }

    /// Lists battles for the player.
    pub async fn battles(&self, count: usize, offset: usize) -> Result<Vec<BattlePreview>> {
        self.service
            .battles_for_player(&self.player, count, offset)
            .await
    }
}
