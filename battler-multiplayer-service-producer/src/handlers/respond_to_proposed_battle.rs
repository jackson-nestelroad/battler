use std::sync::Arc;

use anyhow::Result;
use battler_multiplayer_service::{
    BattlerMultiplayerService,
    ProposedBattleResponse,
};
use battler_service_producer::PlayerOperation;
use uuid::Uuid;

use crate::MultiplayerBattleAuthorizer;

pub(crate) struct Handler {
    pub service: Arc<BattlerMultiplayerService<'static>>,
    pub authorizer: Arc<Box<dyn MultiplayerBattleAuthorizer>>,
}

impl battler_multiplayer_service_schema::RespondToProposedBattleProcedure for Handler {}

impl battler_wamprat::procedure::TypedPatternMatchedProcedure for Handler {
    type Pattern = battler_multiplayer_service_schema::RespondToProposedBattlePattern;
    type Input = battler_multiplayer_service_schema::RespondToProposedBattleInput;
    type Output = battler_multiplayer_service_schema::ProposedBattleOutput;
    type Error = anyhow::Error;

    async fn invoke(
        &self,
        invocation: battler_wamprat::procedure::Invocation,
        input: Self::Input,
        procedure: Self::Pattern,
    ) -> Result<Self::Output, Self::Error> {
        self.authorizer
            .authorize_player_operation(
                &invocation.peer_info,
                &input.0.player,
                PlayerOperation::PlayerData,
            )
            .await?;
        let response: ProposedBattleResponse =
            serde_json::from_str(&input.0.proposed_battle_response_json)?;
        let proposed = self
            .service
            .respond_to_proposed_battle(Uuid::try_parse(&procedure.0)?, &input.0.player, &response)
            .await?;
        Ok(battler_multiplayer_service_schema::ProposedBattleOutput(
            battler_multiplayer_service_schema::ProposedBattle {
                proposed_battle_json: serde_json::to_string(&proposed)?,
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
