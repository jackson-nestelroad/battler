use anyhow::Result;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream,
    WebSocketStream,
};

use crate::{
    peer::{
        connector::web_socket_connector::WebSocketConnectorFactory,
        peer::{
            Peer,
            PeerConfig,
        },
    },
    transport::web_socket_transport::WebSocketTransportFactory,
};

pub type WebSocketPeer = Peer<WebSocketStream<MaybeTlsStream<TcpStream>>>;

pub fn new_web_socket_peer(config: PeerConfig) -> Result<WebSocketPeer> {
    Peer::new(
        config,
        Box::new(WebSocketConnectorFactory::default()),
        Box::new(WebSocketTransportFactory::default()),
    )
}