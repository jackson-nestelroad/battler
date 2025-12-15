use anyhow::Result;
use async_trait::async_trait;
use battler_wamp_uri::WildcardUri;

use crate::{
    core::id::Id,
    router::context::RealmContext,
};

/// Router-level policies for incoming RPC requests.
#[async_trait]
pub trait RpcPolicies<S>: Send + Sync {
    /// Validates that a registration is allowed.
    async fn validate_registration(
        &self,
        context: &RealmContext<'_, S>,
        session: Id,
        procedure: &WildcardUri,
    ) -> Result<()>;
}

/// Default implementation of [`RpcPolicies`] with empty policies.
#[derive(Debug, Default)]
pub struct EmptyRpcPolicies {}

#[async_trait]
impl<S> RpcPolicies<S> for EmptyRpcPolicies {
    async fn validate_registration(
        &self,
        _: &RealmContext<'_, S>,
        _: Id,
        _: &WildcardUri,
    ) -> Result<()> {
        Ok(())
    }
}
