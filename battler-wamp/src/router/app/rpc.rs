use anyhow::Result;
use async_trait::async_trait;

use crate::{
    core::{
        id::Id,
        uri::Uri,
    },
    router::context::RealmContext,
};

/// Router-level policies for incoming RPC requests.
#[async_trait]
pub trait RpcPolicies<S>: Send + Sync {
    /// Validates that a registration is allowed.
    async fn validate_registration(
        &self,
        context: &RealmContext<'_, '_, S>,
        session: Id,
        procedure: &Uri,
    ) -> Result<()>;
}

/// Default implementation of [`RpcPolicies`] with empty policies.
#[derive(Default)]
pub struct EmptyRpcPolicies {}

#[async_trait]
impl<S> RpcPolicies<S> for EmptyRpcPolicies {
    async fn validate_registration(
        &self,
        _: &RealmContext<'_, '_, S>,
        _: Id,
        _: &Uri,
    ) -> Result<()> {
        Ok(())
    }
}
