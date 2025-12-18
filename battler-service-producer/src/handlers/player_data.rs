use std::sync::Arc;

use anyhow::Result;
use battler_service::BattlerService;
use uuid::Uuid;

use crate::{
    BattleAuthorizer,
    PlayerOperation,
};

pub(crate) struct Handler<'d> {
    pub service: Arc<BattlerService<'d>>,
    pub authorizer: Arc<Box<dyn BattleAuthorizer>>,
}

impl<'d> battler_service_schema::PlayerDataProcedure for Handler<'d> {}

impl<'d> battler_wamprat::procedure::TypedPatternMatchedProcedure for Handler<'d> {
    type Pattern = battler_service_schema::PlayerDataPattern;
    type Input = battler_service_schema::PlayerDataInput;
    type Output = battler_service_schema::PlayerDataOutput;
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
        let player_data = self
            .service
            .player_data(Uuid::try_parse(&procedure.0)?, &input.0.player)
            .await?;
        Ok(battler_service_schema::PlayerDataOutput(
            battler_service_schema::PlayerDataOutputArgs {
                player_data_json: serde_json::to_string(&player_data)?,
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
