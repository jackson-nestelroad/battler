use std::sync::Arc;

use anyhow::Result;
use battler_service::BattlerService;
use uuid::Uuid;

pub(crate) struct Handler<'d> {
    pub service: Arc<BattlerService<'d>>,
}

impl<'d> battler_service_schema::BattleProcedure for Handler<'d> {}

impl<'d> battler_wamprat::procedure::TypedPatternMatchedProcedure for Handler<'d> {
    type Pattern = battler_service_schema::BattlePattern;
    type Input = battler_service_schema::BattleInput;
    type Output = battler_service_schema::BattleOutput;
    type Error = anyhow::Error;

    async fn invoke(
        &self,
        _: battler_wamprat::procedure::Invocation,
        _: Self::Input,
        procedure: Self::Pattern,
    ) -> Result<Self::Output, Self::Error> {
        let battle = self.service.battle(Uuid::try_parse(&procedure.0)?).await?;
        Ok(battler_service_schema::BattleOutput(
            battler_service_schema::Battle {
                battle_json: serde_json::to_string(&battle)?,
            },
        ))
    }
}
