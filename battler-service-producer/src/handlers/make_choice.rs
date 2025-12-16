use std::sync::Arc;

use anyhow::Result;
use battler_service::BattlerService;
use uuid::Uuid;

use crate::common::auth::authorize_player;

pub(crate) struct Handler<'d> {
    pub service: Arc<BattlerService<'d>>,
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
        authorize_player(&invocation.peer_info, &input.0.player)?;
        self.service
            .make_choice(
                Uuid::try_parse(&procedure.0)?,
                &input.0.player,
                &input.0.choice,
            )
            .await?;
        Ok(battler_service_schema::MakeChoiceOutput)
    }
}
