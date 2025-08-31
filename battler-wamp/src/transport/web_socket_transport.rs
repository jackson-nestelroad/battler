use core::str;
use std::{
    pin::Pin,
    task,
};

use anyhow::{
    Error,
    Result,
};
use futures_util::{
    Sink,
    SinkExt,
    Stream,
    StreamExt,
};
use tokio::net::TcpStream;
use tokio_tungstenite::{
    MaybeTlsStream,
    WebSocketStream,
    tungstenite::Message,
};

use crate::{
    serializer::serializer::SerializerType,
    transport::transport::{
        Transport,
        TransportData,
        TransportFactory,
    },
};

/// A transport implemented for a TCP stream using the WebSocket protocol.
#[derive(Debug)]
pub struct WebSocketTransport {
    stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
    binary: bool,
}

impl Transport for WebSocketTransport {}

impl Stream for WebSocketTransport {
    type Item = Result<TransportData>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Option<Self::Item>> {
        match futures_util::ready!(self.stream.poll_next_unpin(cx)) {
            Some(Ok(message)) => {
                if message.is_ping() {
                    task::Poll::Ready(Some(Ok(TransportData::Ping(message.into_data().to_vec()))))
                } else if message.is_text() || message.is_binary() {
                    if message.is_text() && self.binary {
                        task::Poll::Ready(Some(Err(Error::msg("expected binary"))))
                    } else if message.is_binary() && !self.binary {
                        task::Poll::Ready(Some(Err(Error::msg("expected text"))))
                    } else {
                        task::Poll::Ready(Some(Ok(TransportData::Message(
                            message.into_data().to_vec(),
                        ))))
                    }
                } else if message.is_close() {
                    task::Poll::Ready(None)
                } else {
                    task::Poll::Ready(Some(Err(Error::msg("unexpected websocket message"))))
                }
            }
            Some(Err(err)) => task::Poll::Ready(Some(Err(err.into()))),
            None => task::Poll::Ready(None),
        }
    }
}

impl Sink<TransportData> for WebSocketTransport {
    type Error = Error;

    fn poll_ready(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<std::result::Result<(), Self::Error>> {
        self.stream.poll_ready_unpin(cx).map_err(Error::new)
    }

    fn start_send(
        mut self: Pin<&mut Self>,
        item: TransportData,
    ) -> std::result::Result<(), Self::Error> {
        let message = match item {
            TransportData::Ping(data) => Message::Pong(data.into()),
            TransportData::Message(data) => {
                if self.binary {
                    Message::Binary(data.into())
                } else {
                    Message::Text(str::from_utf8(&data)?.to_owned().into())
                }
            }
        };
        self.stream.start_send_unpin(message).map_err(Error::new)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<std::result::Result<(), Self::Error>> {
        self.stream.poll_flush_unpin(cx).map_err(Error::new)
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<std::result::Result<(), Self::Error>> {
        self.stream.poll_close_unpin(cx).map_err(Error::new)
    }
}

/// A factory for [`WebSocketTransport`].
#[derive(Default)]
pub struct WebSocketTransportFactory {}

impl TransportFactory<WebSocketStream<MaybeTlsStream<TcpStream>>> for WebSocketTransportFactory {
    fn new_transport(
        &self,
        stream: WebSocketStream<MaybeTlsStream<TcpStream>>,
        serializer_type: SerializerType,
    ) -> Box<dyn Transport> {
        let binary = serializer_type == SerializerType::MessagePack;
        Box::new(WebSocketTransport { stream, binary })
    }
}
