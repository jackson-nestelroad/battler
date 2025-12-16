use std::sync::Arc;

use anyhow::Result;
use battler_service::BattlerService;
use uuid::Uuid;

use crate::common::auth::authorize_player;

pub(crate) struct Handler<'d> {
    pub service: Arc<BattlerService<'d>>,
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
        authorize_player(&invocation.peer_info, &input.0.player)?;
        let team_data = serde_json::from_str(&input.0.team_data_json)?;
        self.service
            .update_team(Uuid::try_parse(&procedure.0)?, &input.0.player, team_data)
            .await?;
        Ok(battler_service_schema::UpdateTeamOutput)
    }
}
