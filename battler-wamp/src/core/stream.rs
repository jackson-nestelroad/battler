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

use crate::{
    message::message::Message,
    serializer::serializer::Serializer,
    transport::transport::{
        Transport,
        TransportData,
    },
};

pub enum StreamMessage {
    Ping(Vec<u8>),
    Message(Message),
}

pub struct MessageStream {
    transport: Box<dyn Transport>,
    serializer: Box<dyn Serializer>,
}

impl MessageStream {
    pub fn new(transport: Box<dyn Transport>, serializer: Box<dyn Serializer>) -> Self {
        Self {
            transport,
            serializer,
        }
    }
}

impl Stream for MessageStream {
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

impl Sink<StreamMessage> for MessageStream {
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
