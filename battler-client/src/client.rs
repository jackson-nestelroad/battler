use std::sync::Arc;

use anyhow::{
    Context,
    Result,
};
use battler_service_client::BattlerServiceClient;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Role {
    Spectator,
    Player { side: usize },
}

pub struct BattlerClient<'a> {
    battle: Uuid,
    player: String,
    role: Role,

    service: Arc<Box<dyn BattlerServiceClient + 'a>>,

    task_handle: tokio::task::JoinHandle<()>,
}

impl<'a> BattlerClient<'a> {
    pub async fn new(
        battle: Uuid,
        player: String,
        service: Arc<Box<dyn BattlerServiceClient + 'a>>,
    ) -> Result<Arc<Self>> {
        let battle = service
            .battle(battle)
            .await
            .context("battle does not exist")?;
        let role = battle
            .sides
            .iter()
            .enumerate()
            .find(|(_, side)| {
                side.players
                    .iter()
                    .find(|battle_player| battle_player.id == player)
                    .is_some()
            })
            .map(|(i, _)| Role::Player { side: i })
            .unwrap_or_else(|| Role::Spectator);
        todo!()
    }
}
