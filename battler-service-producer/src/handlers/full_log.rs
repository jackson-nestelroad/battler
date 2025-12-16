use std::sync::Arc;

use anyhow::Result;
use battler_service::BattlerService;
use uuid::Uuid;

use crate::common::auth::authorize_side;

pub(crate) struct Handler<'d> {
    pub service: Arc<BattlerService<'d>>,
}

impl<'d> battler_service_schema::FullLogProcedure for Handler<'d> {}

impl<'d> battler_wamprat::procedure::TypedPatternMatchedProcedure for Handler<'d> {
    type Pattern = battler_service_schema::FullLogPattern;
    type Input = battler_service_schema::FullLogInput;
    type Output = battler_service_schema::FullLogOutput;
    type Error = anyhow::Error;

    async fn invoke(
        &self,
        invocation: battler_wamprat::procedure::Invocation,
        input: Self::Input,
        procedure: Self::Pattern,
    ) -> Result<Self::Output, Self::Error> {
        let uuid = Uuid::try_parse(&procedure.0)?;
        let side = input.0.side.map(|side| side as usize);
        let battle = self.service.battle(uuid).await?;

        authorize_side(&invocation.peer_info, side, &battle)?;

        let log = self
            .service
            .full_log(Uuid::try_parse(&procedure.0)?, side)
            .await?;
        Ok(battler_service_schema::FullLogOutput(
            battler_service_schema::FullLogOutputArgs { log },
        ))
    }
}
