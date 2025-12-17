use anyhow::{
    Error,
    Result,
};
use async_trait::async_trait;
use battler::CoreBattleOptions;
use battler_service::Battle;
use battler_wamp::core::error::BasicError;

/// Authorizes the battle owner based on the WAMP peer.
pub fn authorize_battle_owner(
    peer_info: &battler_wamp::core::peer_info::PeerInfo,
    battle: &Battle,
) -> Result<()> {
    if peer_info.identity.id != battle.metadata.creator {
        return Err(BasicError::NotAllowed(format!(
            "{} does not own the battle",
            peer_info.identity.id
        ))
        .into());
    }
    Ok(())
}

/// Authorizes a player based on the WAMP peer.
pub fn authorize_player(
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
pub fn authorize_side(
    peer_info: &battler_wamp::core::peer_info::PeerInfo,
    battle: &Battle,
    side: Option<usize>,
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
            .ok_or_else(|| Error::msg(format!("{id} is not on given side"))),
        None => Ok(()),
    }
}

/// An operation on a battle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BattleOperation {
    Delete,
    Start,
}

/// An operation on a player in a battle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlayerOperation {
    MakeChoice,
    PlayerData,
    Request,
    UpdateTeam,
    ValidatePlayer,
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

    /// Authorizes a battle operation.
    #[allow(unused_variables)]
    async fn authorize_battle_operation(
        &self,
        peer_info: &battler_wamp::core::peer_info::PeerInfo,
        battle: &Battle,
        operation: BattleOperation,
    ) -> Result<()> {
        authorize_battle_owner(peer_info, battle)
    }

    /// Authorizes a player operation.
    #[allow(unused_variables)]
    async fn authorize_player_operation(
        &self,
        peer_info: &battler_wamp::core::peer_info::PeerInfo,
        player: &str,
        operation: PlayerOperation,
    ) -> Result<()> {
        authorize_player(peer_info, player)
    }

    /// Authorizes log access.
    async fn authorize_log_access(
        &self,
        peer_info: &battler_wamp::core::peer_info::PeerInfo,
        battle: &Battle,
        side: Option<usize>,
    ) -> Result<()> {
        authorize_side(peer_info, battle, side)
    }
}
