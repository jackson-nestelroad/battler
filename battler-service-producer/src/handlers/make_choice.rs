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

impl<'d> battler_service_schema::MakeChoiceProcedure for Handler<'d> {}

impl<'d> battler_wamprat::procedure::TypedPatternMatchedProcedure for Handler<'d> {
    type Pattern = battler_service_schema::MakeChoicePattern;
    type Input = battler_service_schema::MakeChoiceInput;
    type Output = battler_service_schema::MakeChoiceOutput;
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
                PlayerOperation::MakeChoice,
            )
            .await?;
        self.service
            .make_choice(
                Uuid::try_parse(&procedure.0)?,
                &input.0.player,
                &input.0.choice,
            )
            .await?;
        Ok(battler_service_schema::MakeChoiceOutput)
    }

    fn options() -> battler_wamprat::procedure::ProcedureOptions {
        battler_wamprat::procedure::ProcedureOptions {
            disclose_caller: true,
            ..Default::default()
        }
    }
}
