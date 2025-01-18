use anyhow::Result;
use async_trait::async_trait;

use crate::{
    core::{
        id::Id,
        uri::{
            Uri,
            WildcardUri,
        },
    },
    router::context::RealmContext,
};

/// Router-level policies for incoming pub/sub requests.
#[async_trait]
pub trait PubSubPolicies<S>: Send + Sync {
    /// Validates that a subscription is allowed.
    async fn validate_subscription(
        &self,
        context: &RealmContext<'_, S>,
        session: Id,
        topic: &WildcardUri,
    ) -> Result<()>;

    /// Validates that a publication is allowed.
    async fn validate_publication(
        &self,
        context: &RealmContext<'_, S>,
        session: Id,
        topic: &Uri,
    ) -> Result<()>;
}

/// Default implementation of [`PubSubPolicies`] with empty policies.
#[derive(Debug, Default)]
pub struct EmptyPubSubPolicies {}

#[async_trait]
impl<S> PubSubPolicies<S> for EmptyPubSubPolicies {
    async fn validate_subscription(
        &self,
        _: &RealmContext<'_, S>,
        _: Id,
        _: &WildcardUri,
    ) -> Result<()> {
        Ok(())
    }

    async fn validate_publication(&self, _: &RealmContext<'_, S>, _: Id, _: &Uri) -> Result<()> {
        Ok(())
    }
}
