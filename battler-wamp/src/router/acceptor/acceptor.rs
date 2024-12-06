use anyhow::Result;
use async_trait::async_trait;
use tokio::net::TcpStream;
use tokio_tungstenite::MaybeTlsStream;

use crate::{
    router::context::RouterContext,
    serializer::serializer::SerializerType,
};

/// The result of an [`Acceptor`] successfully accepting a WAMP connection.
pub struct Acceptance<S> {
    /// The established stream.
    pub stream: S,

    /// The chosen serializer.
    pub serializer: SerializerType,
}

/// An acceptor, which accepts WAMP connections over a stream.
///
/// Note that the acceptor only *accepts* connections. It does not establish sessions. In other
/// words, the acceptor is only responsible for establishing that both the client and server can
/// talk using WAMP.
#[async_trait]
pub trait Acceptor<S> {
    /// Accepts the incoming TCP connection, erroring out if the connection fails.
    async fn accept(
        &self,
        context: &RouterContext<S>,
        stream: MaybeTlsStream<TcpStream>,
    ) -> Result<Acceptance<S>>;
}

/// A factory for creating a new [`Acceptor`].
#[async_trait]
pub trait AcceptorFactory<S>: Send {
    /// Creates a new [`Acceptor`].
    fn new_acceptor(&self) -> Box<dyn Acceptor<S> + Send>;
}
