use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use battler::{
    CoreBattleOptions,
    PlayerBattleData,
    Request,
    TeamData,
};
use battler_service::{
    Battle,
    BattlePreview,
    BattlerService,
    LogEntry,
    PlayerValidation,
};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::{
    DirectBattlerServiceClient,
    SimpleWampBattlerServiceClient,
};

/// Client wrapper for [`battler_service::BattlerService`].
#[async_trait]
pub trait BattlerServiceClient {
    /// The status of an existing battle.
    async fn battle(&self, battle: Uuid) -> Result<Battle>;
    /// Creates a new battle.
    async fn create(&self, options: CoreBattleOptions) -> Result<Battle>;
    /// Updates a player's team for a battle.
    async fn update_team(&self, battle: Uuid, player: &str, team: TeamData) -> Result<()>;
    /// Validates a player in a battle.
    async fn validate_player(&self, battle: Uuid, player: &str) -> Result<PlayerValidation>;
    /// Starts a battle.
    async fn start(&self, battle: Uuid) -> Result<()>;
    /// Returns the player data for a player in a battle.
    async fn player_data(&self, battle: Uuid, player: &str) -> Result<PlayerBattleData>;
    /// Returns the current request for a player in a battle.
    async fn request(&self, battle: Uuid, player: &str) -> Result<Option<Request>>;
    /// Sets a player's choice in a battle.
    async fn make_choice(&self, battle: Uuid, player: &str, choice: &str) -> Result<()>;
    /// Reads the full battle log for the side.
    async fn full_log(&self, battle: Uuid, side: Option<usize>) -> Result<Vec<String>>;
    /// Subscribes to battle log updates.
    async fn subscribe(
        &self,
        battle: Uuid,
        side: Option<usize>,
    ) -> Result<broadcast::Receiver<LogEntry>>;
    /// Deletes a battle.
    async fn delete(&self, battle: Uuid) -> Result<()>;
    /// Lists battles.
    async fn battles(&self, count: usize, offset: usize) -> Result<Vec<BattlePreview>>;
    /// Lists battles for a player.
    async fn battles_for_player(
        &self,
        player: &str,
        count: usize,
        offset: usize,
    ) -> Result<Vec<BattlePreview>>;
}

/// Creates a new client around a WAMP service consumer.
pub fn battler_service_client_over_simple_wamp_consumer<S>(
    consumer: Arc<battler_service_schema::BattlerServiceConsumer<S>>,
) -> Box<dyn BattlerServiceClient + 'static>
where
    S: Send + 'static,
{
    Box::new(SimpleWampBattlerServiceClient::new(consumer))
}

/// Creates a new client around a service object.
pub fn battler_service_client_over_direct_service<'d>(
    service: Arc<BattlerService<'d>>,
) -> Box<dyn BattlerServiceClient + 'd> {
    Box::new(DirectBattlerServiceClient::new(service))
}
