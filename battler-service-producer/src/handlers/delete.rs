use std::sync::Arc;

use anyhow::{
    Error,
    Result,
};
use async_trait::async_trait;
use battler_service::{
    Battle,
    BattlerService,
};
use uuid::Uuid;

/// Authorizes a battle to be delete.
#[async_trait]
pub trait Authorizer: Send + Sync {
    async fn authorize(
        &self,
        peer_info: &battler_wamp::core::peer_info::PeerInfo,
        battle: &Battle,
    ) -> Result<()>;
}

pub(crate) struct Handler<'d> {
    pub service: Arc<BattlerService<'d>>,
    pub authorizer: Box<dyn Authorizer>,
}

impl<'d> battler_service_schema::DeleteProcedure for Handler<'d> {}

impl<'d> battler_wamprat::procedure::TypedPatternMatchedProcedure for Handler<'d> {
    type Pattern = battler_service_schema::DeletePattern;
    type Input = battler_service_schema::DeleteInput;
    type Output = battler_service_schema::DeleteOutput;
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
                .authorize(&invocation.peer_info, &battle)
                .await?;
        } else {
            return Err(Error::msg("you cannot delete the battle"));
        }

        self.service.delete(uuid).await?;
        Ok(battler_service_schema::DeleteOutput)
    }
}
