use std::sync::Arc;

use anyhow::Result;
use battler_service::BattlerService;
use uuid::Uuid;

use crate::{
    BattleAuthorizer,
    BattleOperation,
};

pub(crate) struct Handler<'d> {
    pub service: Arc<BattlerService<'d>>,
    pub authorizer: Arc<Box<dyn BattleAuthorizer>>,
}

impl<'d> battler_service_schema::StartProcedure for Handler<'d> {}

impl<'d> battler_wamprat::procedure::TypedPatternMatchedProcedure for Handler<'d> {
    type Pattern = battler_service_schema::StartPattern;
    type Input = battler_service_schema::StartInput;
    type Output = battler_service_schema::StartOutput;
    type Error = anyhow::Error;

    async fn invoke(
        &self,
        invocation: battler_wamprat::procedure::Invocation,
        _: Self::Input,
        procedure: Self::Pattern,
    ) -> Result<Self::Output, Self::Error> {
        let uuid = Uuid::try_parse(&procedure.0)?;
        let battle = self.service.battle(uuid).await?;

        if invocation.peer_info.identity.id != battle.metadata.creator {
            self.authorizer
                .authorize_battle_operation(&invocation.peer_info, &battle, BattleOperation::Start)
                .await?;
        }

        self.service.start(uuid).await?;
        Ok(battler_service_schema::StartOutput)
    }
}
