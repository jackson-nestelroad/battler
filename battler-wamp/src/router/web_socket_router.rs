use anyhow::Result;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream,
    WebSocketStream,
};

use crate::{
    router::{
        acceptor::web_socket_acceptor::WebSocketAcceptorFactory,
        router::{
            Router,
            RouterConfig,
        },
    },
    transport::web_socket_transport::WebSocketTransportFactory,
};

/// A [`Router`] that accepts incoming WebSocket connections.
pub type WebSocketRouter = Router<WebSocketStream<MaybeTlsStream<TcpStream>>>;

/// Creates a new [`WebSocketRouter`].
pub fn new_web_socket_router(config: RouterConfig) -> Result<WebSocketRouter> {
    Router::new(
        config,
        Box::new(WebSocketAcceptorFactory::default()),
        Box::new(WebSocketTransportFactory::default()),
    )
}
