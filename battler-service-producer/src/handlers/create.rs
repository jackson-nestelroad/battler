use std::sync::Arc;

use anyhow::Result;
use battler::CoreBattleEngineOptions;
use battler_service::{
    BattleServiceOptions,
    BattlerService,
};

use crate::BattleAuthorizer;

pub(crate) struct Handler<'d> {
    pub service: Arc<BattlerService<'d>>,
    pub engine_options: CoreBattleEngineOptions,
    pub authorizer: Arc<Box<dyn BattleAuthorizer>>,
}

impl<'d> battler_service_schema::CreateProcedure for Handler<'d> {}

impl<'d> battler_wamprat::procedure::TypedProcedure for Handler<'d> {
    type Input = battler_service_schema::CreateInput;
    type Output = battler_service_schema::BattleOutput;
    type Error = anyhow::Error;

    async fn invoke(
        &self,
        invocation: battler_wamprat::procedure::Invocation,
        input: Self::Input,
    ) -> Result<Self::Output, Self::Error> {
        let options = serde_json::from_str(&input.0.options_json)?;

        self.authorizer
            .authorize_new_battle(&invocation.peer_info, &options)
            .await?;

        let mut service_options: BattleServiceOptions =
            serde_json::from_str(&input.0.service_options_json)?;

        if !invocation.peer_info.identity.id.is_empty() {
            service_options.creator = invocation.peer_info.identity.id.clone();
        }

        let battle = self
            .service
            .create(options, self.engine_options.clone(), service_options)
            .await?;
        Ok(battler_service_schema::BattleOutput(
            battler_service_schema::Battle {
                battle_json: serde_json::to_string(&battle)?,
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
