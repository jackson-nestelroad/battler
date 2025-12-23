use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use battler::{
    CoreBattleEngineOptions,
    CoreBattleOptions,
    PlayerBattleData,
    Request,
    TeamData,
};
use battler_service::{
    Battle,
    BattlePreview,
    BattleServiceOptions,
    BattlerService,
    LogEntry,
    PlayerValidation,
};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::BattlerServiceClient;

/// Implementation of [`BattlerServiceClient`] that uses the
/// [`battler_service::BattlerService`] directly for managing battles.
pub struct DirectBattlerServiceClient<'d> {
    service: Arc<BattlerService<'d>>,
    engine_options: CoreBattleEngineOptions,
}

impl<'d> DirectBattlerServiceClient<'d> {
    /// Creates a new client around a service object.
    pub fn new(service: Arc<BattlerService<'d>>) -> Self {
        Self {
            service,
            engine_options: CoreBattleEngineOptions::default(),
        }
    }

    /// Mutable reference to the battle engine options.
    pub fn engine_options_mut(&mut self) -> &mut CoreBattleEngineOptions {
        &mut self.engine_options
    }
}

#[async_trait]
impl<'d> BattlerServiceClient for DirectBattlerServiceClient<'d> {
    async fn battle(&self, battle: Uuid) -> Result<Battle> {
        self.service.battle(battle).await
    }

    async fn create(
        &self,
        options: CoreBattleOptions,
        service_options: BattleServiceOptions,
    ) -> Result<Battle> {
        self.service
            .create(options, self.engine_options.clone(), service_options)
            .await
    }

    async fn update_team(&self, battle: Uuid, player: &str, team: TeamData) -> Result<()> {
        self.service.update_team(battle, player, team).await
    }

    async fn validate_player(&self, battle: Uuid, player: &str) -> Result<PlayerValidation> {
        self.service.validate_player(battle, player).await
    }

    async fn start(&self, battle: Uuid) -> Result<()> {
        self.service.start(battle).await
    }

    async fn player_data(&self, battle: Uuid, player: &str) -> Result<PlayerBattleData> {
        self.service.player_data(battle, player).await
    }

    async fn request(&self, battle: Uuid, player: &str) -> Result<Option<Request>> {
        self.service.request(battle, player).await
    }

    async fn make_choice(&self, battle: Uuid, player: &str, choice: &str) -> Result<()> {
        self.service.make_choice(battle, player, choice).await
    }

    async fn full_log(&self, battle: Uuid, side: Option<usize>) -> Result<Vec<String>> {
        self.service.full_log(battle, side).await
    }

    async fn last_log_entry(
        &self,
        battle: Uuid,
        side: Option<usize>,
    ) -> Result<Option<(usize, String)>> {
        self.service.last_log_entry(battle, side).await
    }

    async fn subscribe(
        &self,
        battle: Uuid,
        side: Option<usize>,
    ) -> Result<broadcast::Receiver<LogEntry>> {
        self.service.subscribe(battle, side).await
    }

    async fn delete(&self, battle: Uuid) -> Result<()> {
        self.service.delete(battle).await
    }

    async fn battles(&self, count: usize, offset: usize) -> Result<Vec<BattlePreview>> {
        Ok(self.service.battles(count, offset).await)
    }

    async fn battles_for_player(
        &self,
        player: &str,
        count: usize,
        offset: usize,
    ) -> Result<Vec<BattlePreview>> {
        Ok(self.service.battles_for_player(player, count, offset).await)
    }
}
