use std::sync::Arc;

use anyhow::Result;
use battler_service::BattlerService;
use battler_wamp::core::error::WampError;
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
        let battle = self
            .service
            .battle(Uuid::try_parse(&procedure.0)?)
            .await
            .map_err(|err| {
                if let Some(battler_service::BattleError::NotFound) =
                    err.downcast_ref::<battler_service::BattleError>()
                {
                    Self::Error::from(Into::<WampError>::into(
                        battler_service_schema::BattlerServiceError::BattleNotFound,
                    ))
                } else {
                    err
                }
            })?;
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
