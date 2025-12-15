use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use battler::{
    CoreBattleEngineOptions,
    CoreBattleOptions,
};
use battler_service::BattlerService;

/// Authorizes a new battle to be created.
#[async_trait]
pub trait Authorizer: Send + Sync {
    async fn authorize(
        &self,
        peer_info: &battler_wamp::core::peer_info::PeerInfo,
        options: &CoreBattleOptions,
    ) -> Result<()>;
}

pub(crate) struct Handler<'d> {
    pub service: Arc<BattlerService<'d>>,
    pub engine_options: CoreBattleEngineOptions,
    pub authorizer: Arc<Box<dyn Authorizer>>,
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
            .authorize(&invocation.peer_info, &options)
            .await?;

        let service_options = serde_json::from_str(&input.0.service_options_json)?;
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
}
