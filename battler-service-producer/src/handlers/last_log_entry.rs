use std::sync::Arc;

use anyhow::Result;
use battler_service::BattlerService;
use uuid::Uuid;

use crate::common::auth::authorize_side;

pub(crate) struct Handler<'d> {
    pub service: Arc<BattlerService<'d>>,
}

impl<'d> battler_service_schema::LastLogEntryProcedure for Handler<'d> {}

impl<'d> battler_wamprat::procedure::TypedPatternMatchedProcedure for Handler<'d> {
    type Pattern = battler_service_schema::LastLogEntryPattern;
    type Input = battler_service_schema::LastLogEntryInput;
    type Output = battler_service_schema::LastLogEntryOutput;
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

        let last_log_entry = self
            .service
            .last_log_entry(Uuid::try_parse(&procedure.0)?, side)
            .await?;
        Ok(battler_service_schema::LastLogEntryOutput(
            last_log_entry.map(|entry| battler_service_schema::LogEntry {
                index: entry.0 as battler_wamp_values::Integer,
                content: entry.1,
            }),
        ))
    }
}
