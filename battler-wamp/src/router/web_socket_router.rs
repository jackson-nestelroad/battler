use anyhow::Result;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream,
    WebSocketStream,
};

use crate::{
    router::{
        acceptor::web_socket_acceptor::WebSocketAcceptorFactory,
        app::{
            connection::{
                ConnectionPolicies,
                EmptyConnectionPolicies,
            },
            pub_sub::PubSubPolicies,
            rpc::RpcPolicies,
        },
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
pub fn new_web_socket_router(
    config: RouterConfig,
    connection_policies: Box<dyn ConnectionPolicies<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
    pub_sub_policies: Box<dyn PubSubPolicies<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
    rpc_policies: Box<dyn RpcPolicies<WebSocketStream<MaybeTlsStream<TcpStream>>>>,
) -> Result<WebSocketRouter> {
    Router::new(
        config,
        connection_policies,
        pub_sub_policies,
        rpc_policies,
        Box::new(WebSocketAcceptorFactory::default()),
        Box::new(WebSocketTransportFactory::default()),
    )
}
