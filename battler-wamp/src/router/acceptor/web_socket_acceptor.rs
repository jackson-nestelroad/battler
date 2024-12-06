use ahash::HashSet;
use anyhow::{
    Error,
    Result,
};
use async_trait::async_trait;
use tokio::net::TcpStream;
use tokio_tungstenite::{
    tungstenite::{
        handshake::server::{
            Callback,
            ErrorResponse,
            Request,
            Response,
        },
        http::{
            header::SEC_WEBSOCKET_PROTOCOL,
            HeaderValue,
            StatusCode,
        },
    },
    MaybeTlsStream,
    WebSocketStream,
};

use crate::{
    router::{
        acceptor::acceptor::{
            Acceptance,
            Acceptor,
            AcceptorFactory,
        },
        context::RouterContext,
        router::RouterConfig,
    },
    serializer::serializer::SerializerType,
};

struct WebSocketWampNegotiator {
    supported_protocols: HashSet<String>,
    selected_protocol: Option<String>,
}

impl WebSocketWampNegotiator {
    fn new(config: &RouterConfig) -> Self {
        let supported_protocols = config
            .serializers
            .iter()
            .map(|serializer| serializer.uri().into())
            .collect();
        Self {
            supported_protocols,
            selected_protocol: None,
        }
    }

    fn reject_response<S>(message: S) -> ErrorResponse
    where
        S: Into<String>,
    {
        let mut response = ErrorResponse::new(Some(message.into()));
        *response.status_mut() = StatusCode::BAD_REQUEST;
        response
    }

    fn callback(&mut self) -> impl Callback + use<'_> {
        |request: &Request, mut response: Response| -> Result<Response, ErrorResponse> {
            let selected_protocol = request
                .headers()
                .get(SEC_WEBSOCKET_PROTOCOL)
                .map(|protocols| match protocols.to_str() {
                    Ok(protocols) => protocols
                        .split(',')
                        .find(|protocol| self.supported_protocols.contains(protocol.trim())),
                    Err(_) => None,
                })
                .flatten();
            let selected_protocol = match selected_protocol {
                Some(protocol) => protocol,
                None => return Err(Self::reject_response("no supported protocol")),
            };
            self.selected_protocol = Some(selected_protocol.to_owned());
            let header = match HeaderValue::from_str(selected_protocol) {
                Ok(header) => header,
                Err(_) => return Err(Self::reject_response("failed to create response header")),
            };
            response
                .headers_mut()
                .insert(SEC_WEBSOCKET_PROTOCOL, header);
            Ok(response)
        }
    }
}

#[derive(Default)]
struct WebSocketAcceptor {}

#[async_trait]
impl Acceptor<WebSocketStream<MaybeTlsStream<TcpStream>>> for WebSocketAcceptor {
    async fn accept(
        &self,
        context: &RouterContext<WebSocketStream<MaybeTlsStream<TcpStream>>>,
        stream: MaybeTlsStream<TcpStream>,
    ) -> Result<Acceptance<WebSocketStream<MaybeTlsStream<TcpStream>>>> {
        let mut negotiator = WebSocketWampNegotiator::new(&context.router().config);
        let stream = tokio_tungstenite::accept_hdr_async(stream, negotiator.callback()).await?;
        let protocol = match negotiator.selected_protocol {
            Some(protocol) => protocol,
            None => return Err(Error::msg("expected protocol after negotiation")),
        };
        let serializer = SerializerType::try_from(protocol.as_str()).map_err(Error::msg)?;
        Ok(Acceptance { stream, serializer })
    }
}

/// A factory for an [`Acceptor`] for WebSocket connections.
#[derive(Default)]
pub struct WebSocketAcceptorFactory {}

impl AcceptorFactory<WebSocketStream<MaybeTlsStream<TcpStream>>> for WebSocketAcceptorFactory {
    fn new_acceptor(&self) -> Box<dyn Acceptor<WebSocketStream<MaybeTlsStream<TcpStream>>> + Send> {
        Box::new(WebSocketAcceptor::default())
    }
}
