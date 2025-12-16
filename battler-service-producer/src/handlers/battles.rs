use std::sync::Arc;

use anyhow::Result;
use battler_service::BattlerService;

pub(crate) struct Handler<'d> {
    pub service: Arc<BattlerService<'d>>,
}

impl<'d> battler_service_schema::BattlesProcedure for Handler<'d> {}

impl<'d> battler_wamprat::procedure::TypedProcedure for Handler<'d> {
    type Input = battler_service_schema::BattlesInput;
    type Output = battler_service_schema::BattlesOutput;
    type Error = anyhow::Error;

    async fn invoke(
        &self,
        _: battler_wamprat::procedure::Invocation,
        input: Self::Input,
    ) -> Result<Self::Output, Self::Error> {
        let battles = self
            .service
            .battles(input.0.count as usize, input.0.offset as usize)
            .await;
        Ok(battler_service_schema::BattlesOutput(
            battler_service_schema::BattlesOutputArgs {
                battles: battles
                    .into_iter()
                    .map(|battle| {
                        Ok(battler_service_schema::BattlePreview {
                            battle_json: serde_json::to_string(&battle)?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            },
        ))
    }
}
