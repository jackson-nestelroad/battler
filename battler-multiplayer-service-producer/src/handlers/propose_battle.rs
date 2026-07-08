use std::sync::Arc;

use anyhow::Result;
use battler_multiplayer_service::{
    BattlerMultiplayerService,
    ProposedBattleOptions,
};

use crate::MultiplayerBattleAuthorizer;

pub(crate) struct Handler {
    pub service: Arc<BattlerMultiplayerService<'static>>,
    pub authorizer: Arc<Box<dyn MultiplayerBattleAuthorizer>>,
}

impl battler_multiplayer_service_schema::ProposeBattleProcedure for Handler {}

impl battler_wamprat::procedure::TypedProcedure for Handler {
    type Input = battler_multiplayer_service_schema::ProposeBattleInput;
    type Output = battler_multiplayer_service_schema::ProposedBattleOutput;
    type Error = anyhow::Error;

    async fn invoke(
        &self,
        invocation: battler_wamprat::procedure::Invocation,
        input: Self::Input,
    ) -> Result<Self::Output, Self::Error> {
        let options: ProposedBattleOptions =
            serde_json::from_str(&input.0.proposed_battle_options_json)?;
        self.authorizer
            .authorize_new_proposed_battle(&invocation.peer_info, &options)
            .await?;
        let proposed = self.service.clone().propose_battle(options).await?;
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
