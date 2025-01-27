use std::{
    fmt::Debug,
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
use tokio::sync::mpsc;

use crate::{
    message::message::Message,
    serializer::serializer::Serializer,
    transport::transport::{
        Transport,
        TransportData,
    },
};

/// A WAMP message returned out from a [`MessageStream`].
#[derive(Debug)]
pub enum StreamMessage {
    Ping(Vec<u8>),
    Message(Message),
}

/// A stream that produces a sequence of WAMP [`Message`]s asynchronously.
pub trait MessageStream:
    Stream<Item = Result<StreamMessage>> + Sink<StreamMessage, Error = Error> + Send + Unpin + Debug
{
    /// The message stream type for logging.
    fn message_stream_type(&self) -> &'static str;
}

/// A stream of messages over some transport, such as a network connection.
///
/// All remote connections use this message stream.
#[derive(Debug)]
pub struct TransportMessageStream {
    transport: Box<dyn Transport>,
    serializer: Box<dyn Serializer>,
}

impl TransportMessageStream {
    /// Creates a message stream with the given transport and serialization.
    pub fn new(transport: Box<dyn Transport>, serializer: Box<dyn Serializer>) -> Self {
        Self {
            transport,
            serializer,
        }
    }
}

impl MessageStream for TransportMessageStream {
    fn message_stream_type(&self) -> &'static str {
        "TransportMessageStream"
    }
}

impl Stream for TransportMessageStream {
    type Item = Result<StreamMessage>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Option<Self::Item>> {
        match futures_util::ready!(self.transport.poll_next_unpin(cx)) {
            Some(Ok(TransportData::Ping(data))) => {
                task::Poll::Ready(Some(Ok(StreamMessage::Ping(data))))
            }
            Some(Ok(TransportData::Message(data))) => {
                let message = self.serializer.deserialize(&data)?;
                task::Poll::Ready(Some(Ok(StreamMessage::Message(message))))
            }
            Some(Err(err)) => task::Poll::Ready(Some(Err(err.into()))),
            None => task::Poll::Ready(None),
        }
    }
}

impl Sink<StreamMessage> for TransportMessageStream {
    type Error = Error;

    fn poll_ready(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<std::result::Result<(), Self::Error>> {
        self.transport.poll_ready_unpin(cx)
    }

    fn start_send(
        mut self: Pin<&mut Self>,
        item: StreamMessage,
    ) -> std::result::Result<(), Self::Error> {
        let data = match item {
            StreamMessage::Ping(data) => TransportData::Ping(data),
            StreamMessage::Message(message) => {
                TransportData::Message(self.serializer.serialize(&message)?)
            }
        };
        self.transport.start_send_unpin(data)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<std::result::Result<(), Self::Error>> {
        self.transport.poll_flush_unpin(cx)
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<std::result::Result<(), Self::Error>> {
        self.transport.poll_close_unpin(cx)
    }
}

/// A direct stream of messages.
///
/// Used for connections running in the same local process, to skip serialization.
pub struct DirectMessageStream {
    stream: Pin<Box<dyn Stream<Item = StreamMessage> + Send + Sync>>,
    sink: Pin<Box<dyn Sink<StreamMessage, Error = mpsc::error::SendError<Message>> + Send + Sync>>,
}

impl DirectMessageStream {
    /// Creates a direct message stream.
    pub fn new(message_tx: mpsc::Sender<Message>, message_rx: mpsc::Receiver<Message>) -> Self {
        let stream = futures_util::stream::unfold(message_rx, move |mut message_rx| async {
            match message_rx.recv().await {
                Some(message) => Some((StreamMessage::Message(message), message_rx)),
                None => None,
            }
        });
        let sink = futures_util::sink::unfold(
            message_tx,
            move |message_tx, message: StreamMessage| async {
                match message {
                    StreamMessage::Message(message) => message_tx.send(message).await?,
                    StreamMessage::Ping(_) => (),
                }
                Ok::<_, mpsc::error::SendError<_>>(message_tx)
            },
        );
        Self {
            stream: Box::pin(stream),
            sink: Box::pin(sink),
        }
    }
}

impl Debug for DirectMessageStream {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message_stream_type())
    }
}

impl MessageStream for DirectMessageStream {
    fn message_stream_type(&self) -> &'static str {
        "DirectMessageStream"
    }
}

impl Stream for DirectMessageStream {
    type Item = Result<StreamMessage>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Option<Self::Item>> {
        match futures_util::ready!(self.stream.poll_next_unpin(cx)) {
            Some(message) => task::Poll::Ready(Some(Ok(message))),
            None => task::Poll::Ready(None),
        }
    }
}

impl Sink<StreamMessage> for DirectMessageStream {
    type Error = Error;

    fn poll_ready(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<std::result::Result<(), Self::Error>> {
        self.sink.poll_ready_unpin(cx).map_err(Error::new)
    }

    fn start_send(
        mut self: Pin<&mut Self>,
        item: StreamMessage,
    ) -> std::result::Result<(), Self::Error> {
        self.sink.start_send_unpin(item).map_err(Error::new)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<std::result::Result<(), Self::Error>> {
        self.sink.poll_flush_unpin(cx).map_err(Error::new)
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<std::result::Result<(), Self::Error>> {
        self.sink.poll_close_unpin(cx).map_err(Error::new)
    }
}
