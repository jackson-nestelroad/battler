use anyhow::Result;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream,
    WebSocketStream,
};

use crate::{
    peer::{
        WebSocketConnectorFactory,
        peer::{
            Peer,
            PeerConfig,
        },
    },
    transport::web_socket_transport::WebSocketTransportFactory,
};

/// A WAMP peer over a WebSocket stream.
pub type WebSocketPeer = Peer<WebSocketStream<MaybeTlsStream<TcpStream>>>;

/// Creates a new [`WebSocketPeer`].
pub fn new_web_socket_peer(config: PeerConfig) -> Result<WebSocketPeer> {
    Peer::new(
        config,
        Box::new(WebSocketConnectorFactory::default()),
        Box::new(WebSocketTransportFactory::default()),
    )
}
