use anyhow::Result;
use async_trait::async_trait;
use battler_multiplayer_service::ProposedBattleOptions;
use battler_service_producer::BattleAuthorizer;

#[async_trait]
pub trait MultiplayerBattleAuthorizer: BattleAuthorizer {
    /// Authorizes a new proposed battle to be created.
    async fn authorize_new_proposed_battle(
        &self,
        peer_info: &battler_wamp::core::peer_info::PeerInfo,
        options: &ProposedBattleOptions,
    ) -> Result<()>;
}
