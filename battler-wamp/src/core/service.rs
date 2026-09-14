use std::{
    pin::Pin,
    task,
    time::Duration,
};

use anyhow::{
    Error,
    Result,
};
use futures_util::{
    SinkExt,
    StreamExt,
};
use log::error;
use tokio::{
    sync::{
        broadcast,
        mpsc,
    },
    task::JoinHandle,
};

use crate::{
    core::{
        error::InteractionError,
        peer_info::ConnectionType,
        stream::{
            MessageStream,
            StreamMessage,
        },
    },
    message::{
        common::abort_message_for_error,
        message::Message,
    },
};

struct StreamWrapper {
    inner: Box<dyn MessageStream>,
}

impl futures_util::Stream for StreamWrapper {
    type Item = Result<StreamMessage>;

    fn poll_next(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Option<Self::Item>> {
        Pin::new(&mut *self.inner).poll_next(cx)
    }
}

impl futures_util::Sink<StreamMessage> for StreamWrapper {
    type Error = Error;

    fn poll_ready(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Result<(), Self::Error>> {
        Pin::new(&mut *self.inner).poll_ready(cx)
    }

    fn start_send(mut self: Pin<&mut Self>, item: StreamMessage) -> Result<(), Self::Error> {
        Pin::new(&mut *self.inner).start_send(item)
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Result<(), Self::Error>> {
        Pin::new(&mut *self.inner).poll_flush(cx)
    }

    fn poll_close(
        mut self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Result<(), Self::Error>> {
        Pin::new(&mut *self.inner).poll_close(cx)
    }
}

/// A handle to an asynchronously-running [`Service`].
pub struct ServiceHandle {
    start_handle: JoinHandle<()>,
    cancel_tx: broadcast::Sender<()>,
    message_tx: mpsc::Sender<Message>,
}

impl ServiceHandle {
    /// Joins the task running the service.
    pub async fn join(self) -> Result<()> {
        self.start_handle.await.map_err(Error::new)
    }

    /// Cancels the service.
    ///
    /// Cancellation is the correct way to cleanly exit a service.
    pub fn cancel(&self) -> Result<()> {
        self.cancel_tx.send(()).map(|_| ()).map_err(Error::new)
    }

    /// The message transmission channel.
    pub fn message_tx(&self) -> mpsc::Sender<Message> {
        self.message_tx.clone()
    }
}

/// The core asynchronous service that sends and receives WAMP messages over an underlying
/// transport.
///
/// The goal of this module is to provide a common layer for WAMP messaging. Received messages are
/// passed to a channel for higher layers (such as a single session on a router or a peer) to
/// process.
///
/// This type assumes that errors are handled higher up in the stack. In other words, canceling the
/// operation of this service *will not* inject an ABORT message. If a router wishes to cancel a
/// session, the session object itself should be canceled, and it's expected that the session sends
/// ABORT before canceling the service. The same applies for peers: the peer should inject an ABORT
/// message when canceled before canceling the service.
pub struct Service {
    name: String,
    stream: Box<dyn MessageStream>,
    message_tx: broadcast::Sender<Message>,
    end_tx: broadcast::Sender<()>,
    _end_rx: broadcast::Receiver<()>,
    cancel_tx: broadcast::Sender<()>,
    cancel_rx: broadcast::Receiver<()>,

    user_message_tx: mpsc::Sender<Message>,
    user_message_rx: mpsc::Receiver<Message>,
}

impl Service {
    /// Creates a new service over a message stream.
    pub fn new(name: String, stream: Box<dyn MessageStream>) -> Self {
        let (message_tx, _) = broadcast::channel(4096);
        let (end_tx, end_rx) = broadcast::channel(1);
        let (cancel_tx, cancel_rx) = broadcast::channel(1);
        let (user_message_tx, user_message_rx) = mpsc::channel(4096);
        Self {
            name,
            stream,
            message_tx,
            end_tx,
            _end_rx: end_rx,
            cancel_tx,
            cancel_rx,
            user_message_tx,
            user_message_rx,
        }
    }

    /// The connection type of the underlying stream.
    pub fn connection_type(&self) -> ConnectionType {
        self.stream.connection_type()
    }

    /// The message receiver channel.
    pub fn message_rx(&self) -> broadcast::Receiver<Message> {
        self.message_tx.subscribe()
    }

    /// The end receiver channel.
    pub fn end_rx(&self) -> broadcast::Receiver<()> {
        self.end_tx.subscribe()
    }

    /// Starts the service asynchronously.
    ///
    /// This method takes ownership of the service. All future interactions with the service should
    /// be made through the returned handle.
    pub fn start(self) -> ServiceHandle {
        let cancel_tx = self.cancel_tx.clone();
        let message_tx = self.user_message_tx.clone();
        let start_handle = tokio::spawn(self.run());
        ServiceHandle {
            start_handle,
            cancel_tx,
            message_tx,
        }
    }

    async fn run(self) {
        let wrapper = StreamWrapper { inner: self.stream };
        let (mut stream_sink, mut stream_stream) = wrapper.split();
        let (write_tx, mut write_rx) = mpsc::channel(4096);

        // Spawn the writer task to handle sending asynchronously.
        let name_clone = self.name.clone();
        let writer_handle = tokio::spawn(async move {
            while let Some(msg) = write_rx.recv().await {
                if let Err(err) = stream_sink.send(msg).await {
                    error!("Service {name_clone} writer error: {err:#}");
                    break;
                }
            }
            // Close the sink cleanly on exit.
            tokio::time::timeout(Duration::from_millis(500), stream_sink.close())
                .await
                .ok();
        });

        let mut result = Ok(());
        let mut cancel_rx = self.cancel_rx;
        let mut user_message_rx = self.user_message_rx;
        let message_tx = self.message_tx.clone();

        loop {
            tokio::select! {
                message = stream_stream.next() => {
                    match message {
                        Some(Ok(StreamMessage::Ping(data))) => {
                            // Ping the message back.
                            if write_tx.send(StreamMessage::Ping(data)).await.is_err() {
                                break;
                            }
                        },
                        Some(Ok(StreamMessage::Message(message))) => {
                            // Send the message out for handling.
                            if message_tx.send(message).is_err() {
                                break;
                            }
                        }
                        Some(Err(err)) => {
                            // Failed to parse the message.
                            //
                            // Inject an ABORT message at this layer, since the stream will be abruptly closed, and we have no way of determining what the downstream intent was.
                            //
                            // Ignore the error because the stream may be closed.
                            let abort_msg = abort_message_for_error(&InteractionError::ProtocolViolation("stream abruptly closed".to_owned()).into());
                            let _ = write_tx.send(StreamMessage::Message(abort_msg)).await;
                            result = Err(err);
                            break;
                        }
                        None => {
                            break;
                        }
                    }
                }
                message = user_message_rx.recv() => {
                    match message {
                        Some(message) => {
                            if write_tx.send(StreamMessage::Message(message)).await.is_err() {
                                break;
                            }
                        }
                        None => {
                            result = Err(Error::msg("user message stream closed"));
                            break;
                        }
                    }
                }
                // We expect that cancellation is the correct way to cleanly exit the service.
                _ = cancel_rx.recv() => {
                    break;
                }
                // Timeout is implemented at this layer so that ping messages are considered
                // for keeping the connection alive.
                _ = tokio::time::sleep(Duration::from_secs(300)) => {
                    result = Err(Error::msg("timed out"));
                    break;
                }
            }
        }

        // Clean up: drop write_tx so that the writer task terminates, then wait for the writer task
        // to exit.
        drop(write_tx);
        let _ = writer_handle.await;

        if let Err(err) = result {
            error!("Service {} failed: {err}", self.name);
        }
        let _ = self.end_tx.send(());
    }
}
