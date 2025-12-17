use anyhow::{
    Error,
    Result,
};
use async_trait::async_trait;
use battler::CoreBattleOptions;
use battler_service::Battle;

/// Authorizes a player based on the WAMP peer.
pub(crate) fn authorize_player(
    peer_info: &battler_wamp::core::peer_info::PeerInfo,
    player: &str,
) -> Result<()> {
    let id = &peer_info.identity.id;
    if id != player {
        return Err(Error::msg(format!("{id} cannot act as {player}")));
    }
    Ok(())
}

/// Authorizes access to a side based on the WAMP peer.
pub(crate) fn authorize_side(
    peer_info: &battler_wamp::core::peer_info::PeerInfo,
    side: Option<usize>,
    battle: &Battle,
) -> Result<()> {
    let id = &peer_info.identity.id;
    match side {
        Some(side) => battle
            .sides
            .get(side)
            .ok_or_else(|| Error::msg("side does not exist"))?
            .players
            .iter()
            .find(|player| &player.id == id)
            .map(|_| ())
            .ok_or_else(|| Error::msg(format!("{id} is not on side {side}"))),
        None => Ok(()),
    }
}

/// Authorizer for battle operations.
#[async_trait]
pub trait BattleAuthorizer: Send + Sync {
    /// Authorizes a new battle to be created.
    async fn authorize_new_battle(
        &self,
        peer_info: &battler_wamp::core::peer_info::PeerInfo,
        options: &CoreBattleOptions,
    ) -> Result<()>;

    /// Authorizes a battle management operation.
    async fn authorize_battle_management(
        &self,
        peer_info: &battler_wamp::core::peer_info::PeerInfo,
        battle: &Battle,
    ) -> Result<()>;
}
