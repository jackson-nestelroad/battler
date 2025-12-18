use anyhow::Result;
use async_trait::async_trait;
use battler_wamp_uri::WildcardUri;

use crate::router::SessionHandle;

/// Router-level policies for incoming RPC requests.
#[async_trait]
pub trait RpcPolicies<S>: Send + Sync {
    /// Validates that a registration is allowed.
    #[allow(unused_variables)]
    async fn validate_registration(
        &self,
        session: &SessionHandle,
        procedure: &WildcardUri,
    ) -> Result<()> {
        Ok(())
    }
}

/// Default implementation of [`RpcPolicies`] with empty policies.
#[derive(Debug, Default)]
pub struct EmptyRpcPolicies;

#[async_trait]
impl<S> RpcPolicies<S> for EmptyRpcPolicies {}
