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

impl<'d> battler_service_schema::UpdateTeamProcedure for Handler<'d> {}

impl<'d> battler_wamprat::procedure::TypedPatternMatchedProcedure for Handler<'d> {
    type Pattern = battler_service_schema::UpdateTeamPattern;
    type Input = battler_service_schema::UpdateTeamInput;
    type Output = battler_service_schema::UpdateTeamOutput;
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
                PlayerOperation::UpdateTeam,
            )
            .await?;
        let team_data = serde_json::from_str(&input.0.team_data_json)?;
        self.service
            .update_team(Uuid::try_parse(&procedure.0)?, &input.0.player, team_data)
            .await?;
        Ok(battler_service_schema::UpdateTeamOutput)
    }

    fn options() -> battler_wamprat::procedure::ProcedureOptions {
        battler_wamprat::procedure::ProcedureOptions {
            disclose_caller: true,
            ..Default::default()
        }
    }
}
