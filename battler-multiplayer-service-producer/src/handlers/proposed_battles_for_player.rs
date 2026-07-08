use std::sync::Arc;

use anyhow::Result;
use battler_multiplayer_service::BattlerMultiplayerService;
use battler_service_producer::PlayerOperation;

use crate::MultiplayerBattleAuthorizer;

pub(crate) struct Handler {
    pub service: Arc<BattlerMultiplayerService<'static>>,
    pub authorizer: Arc<Box<dyn MultiplayerBattleAuthorizer>>,
}

impl battler_multiplayer_service_schema::ProposedBattlesForPlayerProcedure for Handler {}

impl battler_wamprat::procedure::TypedProcedure for Handler {
    type Input = battler_multiplayer_service_schema::ProposedBattlesForPlayerInput;
    type Output = battler_multiplayer_service_schema::ProposedBattlesOutput;
    type Error = anyhow::Error;

    async fn invoke(
        &self,
        invocation: battler_wamprat::procedure::Invocation,
        input: Self::Input,
    ) -> Result<Self::Output, Self::Error> {
        self.authorizer
            .authorize_player_operation(
                &invocation.peer_info,
                &input.0.player,
                PlayerOperation::PlayerData,
            )
            .await?;
        let proposed_battles = self
            .service
            .proposed_battles_for_player(
                &input.0.player,
                input.0.count as usize,
                input.0.offset as usize,
            )
            .await;
        Ok(battler_multiplayer_service_schema::ProposedBattlesOutput(
            battler_multiplayer_service_schema::ProposedBattlesOutputArgs {
                proposed_battles: proposed_battles
                    .into_iter()
                    .map(|proposed_battle| {
                        Ok(battler_multiplayer_service_schema::ProposedBattle {
                            proposed_battle_json: serde_json::to_string(&proposed_battle)?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            },
        ))
    }

    fn options() -> battler_wamprat::procedure::ProcedureOptions {
        battler_wamprat::procedure::ProcedureOptions {
            disclose_caller: true,
            ..Default::default()
        }
    }
}
