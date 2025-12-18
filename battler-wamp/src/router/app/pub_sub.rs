use anyhow::Result;
use async_trait::async_trait;
use battler_wamp_uri::{
    Uri,
    WildcardUri,
};

use crate::router::SessionHandle;

/// Router-level policies for incoming pub/sub requests.
#[async_trait]
pub trait PubSubPolicies<S>: Send + Sync {
    /// Validates that a subscription is allowed.
    #[allow(unused_variables)]
    async fn validate_subscription(
        &self,
        session: &SessionHandle,
        topic: &WildcardUri,
    ) -> Result<()> {
        Ok(())
    }

    /// Validates that a publication is allowed.
    #[allow(unused_variables)]
    async fn validate_publication(&self, session: &SessionHandle, topic: &Uri) -> Result<()> {
        Ok(())
    }
}

/// Default implementation of [`PubSubPolicies`] with empty policies.
#[derive(Debug, Default)]
pub struct EmptyPubSubPolicies;

#[async_trait]
impl<S> PubSubPolicies<S> for EmptyPubSubPolicies {}
