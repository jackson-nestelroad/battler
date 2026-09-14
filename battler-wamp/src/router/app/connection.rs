use anyhow::Result;
use async_trait::async_trait;

use crate::{
    core::peer_info::PeerInfo,
    router::SessionHandle,
};

/// Router-level policies for incoming connections.
#[async_trait]
pub trait ConnectionPolicies<S>: Send + Sync {
    /// Validates that a connection / session establishment is allowed.
    #[allow(unused_variables)]
    async fn validate_connection(
        &self,
        session: &SessionHandle,
        peer_info: &PeerInfo,
    ) -> Result<()> {
        Ok(())
    }
}

/// Default implementation of [`ConnectionPolicies`] with empty policies.
#[derive(Debug, Default)]
pub struct EmptyConnectionPolicies;

#[async_trait]
impl<S> ConnectionPolicies<S> for EmptyConnectionPolicies {}
