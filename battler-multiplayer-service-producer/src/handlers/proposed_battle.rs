use std::sync::Arc;

use anyhow::Result;
use battler_multiplayer_service::BattlerMultiplayerService;
use uuid::Uuid;

pub(crate) struct Handler {
    pub service: Arc<BattlerMultiplayerService<'static>>,
}

impl battler_multiplayer_service_schema::ProposedBattleProcedure for Handler {}

impl battler_wamprat::procedure::TypedPatternMatchedProcedure for Handler {
    type Pattern = battler_multiplayer_service_schema::ProposedBattlePattern;
    type Input = battler_multiplayer_service_schema::ProposedBattleInput;
    type Output = battler_multiplayer_service_schema::ProposedBattleOutput;
    type Error = anyhow::Error;

    async fn invoke(
        &self,
        _: battler_wamprat::procedure::Invocation,
        _: Self::Input,
        procedure: Self::Pattern,
    ) -> Result<Self::Output, Self::Error> {
        let proposed = self
            .service
            .proposed_battle(Uuid::try_parse(&procedure.0)?)
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
