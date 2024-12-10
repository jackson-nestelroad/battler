use anyhow::Result;
use async_trait::async_trait;

use crate::{
    peer::peer::PeerConfig,
    serializer::serializer::SerializerType,
};

/// A connection to a WAMP router produced by a [`Connector`].
pub struct Connection<S> {
    pub stream: S,
    pub serializer: SerializerType,
}

/// A type for initiating a connection to a router.
#[async_trait]
pub trait Connector<S> {
    async fn connect(&self, config: &PeerConfig, uri: &str) -> Result<Connection<S>>;
}

/// A type for generating a new [`Connector`].
#[async_trait]
pub trait ConnectorFactory<S>: Send + Sync {
    /// Creates a new [`Connector`].
    fn new_connector(&self) -> Box<dyn Connector<S> + Send>;
}
