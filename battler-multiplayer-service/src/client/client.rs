use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::{
    ProposedBattle,
    ProposedBattleOptions,
    ProposedBattleResponse,
    ProposedBattleUpdate,
};

/// Client wrapper for [`BattlerMultiplayerService`][`crate::BattlerMultiplayerService`].
#[async_trait]
pub trait BattlerMultiplayerServiceClient: Send + Sync {
    /// Proposes a battle.
    async fn propose_battle(&self, options: ProposedBattleOptions) -> Result<ProposedBattle>;
    /// Lists proposed battles for a player.
    async fn proposed_battles_for_player(
        &self,
        player: &str,
        count: usize,
        offset: usize,
    ) -> Result<Vec<ProposedBattle>>;
    /// Responds to a proposed battle.
    async fn respond_to_proposed_battle(
        &self,
        proposed_battle: Uuid,
        player: &str,
        response: ProposedBattleResponse,
    ) -> Result<()>;
    /// Subscribes to all proposed battle updates for the player.
    async fn proposed_battle_updates(
        &self,
        player: &str,
    ) -> Result<broadcast::Receiver<ProposedBattleUpdate>>;
}
