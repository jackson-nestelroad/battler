use anyhow::Result;
use async_trait::async_trait;

use crate::{
    core::{
        id::Id,
        uri::Uri,
    },
    router::context::RealmContext,
};

#[async_trait]
pub trait PubSubPolicies<S>: Send + Sync {
    async fn validate_subscription(
        &self,
        context: &RealmContext<'_, '_, S>,
        session: Id,
        topic: &Uri,
    ) -> Result<()>;

    async fn validate_publication(
        &self,
        context: &RealmContext<'_, '_, S>,
        session: Id,
        topic: &Uri,
    ) -> Result<()>;
}

#[derive(Default)]
pub struct EmptyPubSubPolicies {}

#[async_trait]
impl<S> PubSubPolicies<S> for EmptyPubSubPolicies {
    async fn validate_subscription(
        &self,
        _: &RealmContext<'_, '_, S>,
        _: Id,
        _: &Uri,
    ) -> Result<()> {
        Ok(())
    }

    async fn validate_publication(
        &self,
        _: &RealmContext<'_, '_, S>,
        _: Id,
        _: &Uri,
    ) -> Result<()> {
        Ok(())
    }
}
