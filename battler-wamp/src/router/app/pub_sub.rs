use anyhow::Result;
use async_trait::async_trait;

use crate::{
    core::{
        id::Id,
        uri::Uri,
    },
    router::context::RouterContext,
};

#[async_trait]
pub trait PubSubPolicies<S>: Send + Sync {
    async fn validate_subscription(
        &self,
        context: &RouterContext<S>,
        session: Id,
        realm: &Uri,
        topic: &Uri,
    ) -> Result<()>;
}

#[derive(Default)]
pub struct EmptyPubSubPolicies {}

#[async_trait]
impl<S> PubSubPolicies<S> for EmptyPubSubPolicies {
    async fn validate_subscription(
        &self,
        _: &RouterContext<S>,
        _: Id,
        _: &Uri,
        _: &Uri,
    ) -> Result<()> {
        Ok(())
    }
}
