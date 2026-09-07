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

/// Authorizes reading player data if the WAMP peer is the player or on the same side in the battle.
pub fn authorize_player_or_side(
    peer_info: &battler_wamp::core::peer_info::PeerInfo,
    battle: &Battle,
    player: &str,
) -> Result<()> {
    let id = &peer_info.identity.id;
    if id == player {
        return Ok(());
    }
    let peer_side = battle
        .sides
        .iter()
        .position(|side| side.players.iter().any(|p| &p.id == id));
    let target_side = battle
        .sides
        .iter()
        .position(|side| side.players.iter().any(|p| &p.id == player));
    match (peer_side, target_side) {
        (Some(p), Some(t)) if p == t => Ok(()),
        _ => Err(Error::msg(format!("{id} cannot act as {player}"))),
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

    /// Authorizes reading player data.
    async fn authorize_player_data_access(
        &self,
        peer_info: &battler_wamp::core::peer_info::PeerInfo,
        battle: &Battle,
        player: &str,
    ) -> Result<()> {
        authorize_player_or_side(peer_info, battle, player)
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

#[cfg(test)]
mod test {
    use battler_service::{
        Battle,
        BattleMetadata,
        BattleState,
        BattleStatus,
        Player,
        PlayerState,
        Side,
    };
    use battler_wamp::{
        auth::identity::Identity,
        core::peer_info::{
            ConnectionType,
            PeerInfo,
        },
    };
    use uuid::Uuid;

    use super::*;

    fn mock_peer(id: &str) -> PeerInfo {
        PeerInfo {
            connection_type: ConnectionType::Direct,
            identity: Identity {
                id: id.to_owned(),
                role: "user".to_owned(),
            },
        }
    }

    fn mock_multi_battle() -> Battle {
        Battle {
            uuid: Uuid::new_v4(),
            state: BattleState::Active,
            status: BattleStatus { turn: 1 },
            sides: vec![
                Side {
                    name: "Side 1".to_owned(),
                    players: vec![
                        Player {
                            id: "player-1".to_owned(),
                            name: "P1".to_owned(),
                            state: PlayerState::Ready,
                        },
                        Player {
                            id: "player-3".to_owned(),
                            name: "P3".to_owned(),
                            state: PlayerState::Ready,
                        },
                    ],
                },
                Side {
                    name: "Side 2".to_owned(),
                    players: vec![
                        Player {
                            id: "player-2".to_owned(),
                            name: "P2".to_owned(),
                            state: PlayerState::Ready,
                        },
                        Player {
                            id: "player-4".to_owned(),
                            name: "P4".to_owned(),
                            state: PlayerState::Ready,
                        },
                    ],
                },
            ],
            drop_reason: None,
            metadata: BattleMetadata {
                creator: "creator".to_owned(),
                battle_type: battler::battle::BattleType::Multi,
                rules: vec![],
                timers: Default::default(),
                special: None,
            },
        }
    }

    #[test]
    fn allows_self_access() {
        let battle = mock_multi_battle();
        let peer = mock_peer("player-1");
        assert!(authorize_player_or_side(&peer, &battle, "player-1").is_ok());
    }

    #[test]
    fn allows_ally_access_on_same_side() {
        let battle = mock_multi_battle();
        let peer = mock_peer("player-1");
        assert!(authorize_player_or_side(&peer, &battle, "player-3").is_ok());

        let peer3 = mock_peer("player-3");
        assert!(authorize_player_or_side(&peer3, &battle, "player-1").is_ok());
    }

    #[test]
    fn denies_foe_access_on_opposing_side() {
        let battle = mock_multi_battle();
        let peer = mock_peer("player-1");
        let err = authorize_player_or_side(&peer, &battle, "player-2").unwrap_err();
        assert_eq!(err.to_string(), "player-1 cannot act as player-2");

        let err4 = authorize_player_or_side(&peer, &battle, "player-4").unwrap_err();
        assert_eq!(err4.to_string(), "player-1 cannot act as player-4");
    }

    #[test]
    fn denies_spectator_or_unknown_player_access() {
        let battle = mock_multi_battle();
        let peer = mock_peer("spectator-1");
        let err = authorize_player_or_side(&peer, &battle, "player-1").unwrap_err();
        assert_eq!(err.to_string(), "spectator-1 cannot act as player-1");

        let peer_p1 = mock_peer("player-1");
        let err_unknown =
            authorize_player_or_side(&peer_p1, &battle, "unknown-player").unwrap_err();
        assert_eq!(
            err_unknown.to_string(),
            "player-1 cannot act as unknown-player"
        );
    }
}
