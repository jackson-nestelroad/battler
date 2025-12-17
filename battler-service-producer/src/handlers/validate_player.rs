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

impl<'d> battler_service_schema::ValidatePlayerProcedure for Handler<'d> {}

impl<'d> battler_wamprat::procedure::TypedPatternMatchedProcedure for Handler<'d> {
    type Pattern = battler_service_schema::ValidatePlayerPattern;
    type Input = battler_service_schema::ValidatePlayerInput;
    type Output = battler_service_schema::ValidatePlayerOutput;
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
                PlayerOperation::UpdateTeam,
            )
            .await?;
        let validation = self
            .service
            .validate_player(Uuid::try_parse(&procedure.0)?, &input.0.player)
            .await?;
        Ok(battler_service_schema::ValidatePlayerOutput(
            battler_service_schema::ValidatePlayerOutputArgs {
                problems: validation.problems,
            },
        ))
    }
}
