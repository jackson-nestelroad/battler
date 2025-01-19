use anyhow::{
    Error,
    Result,
};
use async_trait::async_trait;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream,
    WebSocketStream,
    connect_async,
    tungstenite::{
        ClientRequestBuilder,
        http::header::SEC_WEBSOCKET_PROTOCOL,
    },
};

use crate::{
    peer::{
        connector::connector::{
            Connection,
            Connector,
            ConnectorFactory,
        },
        peer::PeerConfig,
    },
    serializer::serializer::SerializerType,
};

#[derive(Default)]
struct WebSocketConnector {}

#[async_trait]
impl Connector<WebSocketStream<MaybeTlsStream<TcpStream>>> for WebSocketConnector {
    async fn connect(
        &self,
        config: &PeerConfig,
        uri: &str,
    ) -> Result<Connection<WebSocketStream<MaybeTlsStream<TcpStream>>>> {
        let mut request = ClientRequestBuilder::new(uri.try_into()?);
        if !config.agent.is_empty() {
            request = request.with_header("User-Agent", &config.agent);
        }
        for protocol in &config.serializers {
            request = request.with_sub_protocol(protocol.uri().to_string());
        }

        if let Some(web_socket) = &config.web_socket {
            for (key, value) in &web_socket.headers {
                request = request.with_header(key, value);
            }
        }

        let (stream, response) = connect_async(request).await?;
        let serializer = match response.headers().get(SEC_WEBSOCKET_PROTOCOL) {
            Some(protocol) => {
                let protocol = protocol.to_str()?;
                SerializerType::try_from(protocol).map_err(Error::msg)?
            }
            None => return Err(Error::msg("handshake did not produce a sub-protocol")),
        };

        Ok(Connection { stream, serializer })
    }
}

/// A factory for generating [`Connector`]s for WebSocket connections.
#[derive(Default)]
pub struct WebSocketConnectorFactory {}

impl ConnectorFactory<WebSocketStream<MaybeTlsStream<TcpStream>>> for WebSocketConnectorFactory {
    fn new_connector(
        &self,
    ) -> Box<dyn Connector<WebSocketStream<MaybeTlsStream<TcpStream>>> + Send> {
        Box::new(WebSocketConnector::default())
    }
}
