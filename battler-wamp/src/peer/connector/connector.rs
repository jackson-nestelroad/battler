use anyhow::Result;
use async_trait::async_trait;

use crate::{
    peer::peer::PeerConfig,
    serializer::serializer::SerializerType,
};

pub struct Connection<S> {
    pub stream: S,
    pub serializer: SerializerType,
}

#[async_trait]
pub trait Connector<S> {
    async fn connect(&self, config: &PeerConfig, uri: &str) -> Result<Connection<S>>;
}

#[async_trait]
pub trait ConnectorFactory<S>: Send {
    fn new_connector(&self) -> Box<dyn Connector<S> + Send>;
}
