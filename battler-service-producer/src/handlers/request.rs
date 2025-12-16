use std::sync::Arc;

use anyhow::Result;
use battler_service::BattlerService;
use uuid::Uuid;

use crate::common::auth::authorize_player;

pub(crate) struct Handler<'d> {
    pub service: Arc<BattlerService<'d>>,
}

impl<'d> battler_service_schema::RequestProcedure for Handler<'d> {}

impl<'d> battler_wamprat::procedure::TypedPatternMatchedProcedure for Handler<'d> {
    type Pattern = battler_service_schema::RequestPattern;
    type Input = battler_service_schema::RequestInput;
    type Output = battler_service_schema::RequestOutput;
    type Error = anyhow::Error;

    async fn invoke(
        &self,
        invocation: battler_wamprat::procedure::Invocation,
        input: Self::Input,
        procedure: Self::Pattern,
    ) -> Result<Self::Output, Self::Error> {
        authorize_player(&invocation.peer_info, &input.0.player)?;
        let request = self
            .service
            .request(Uuid::try_parse(&procedure.0)?, &input.0.player)
            .await?;
        Ok(battler_service_schema::RequestOutput(
            battler_service_schema::RequestOutputArgs {
                request_json: request
                    .map(|request| serde_json::to_string(&request))
                    .transpose()?,
            },
        ))
    }
}
