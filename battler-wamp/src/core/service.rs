use std::time::Duration;

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
        mpsc::{
            unbounded_channel,
            UnboundedReceiver,
            UnboundedSender,
        },
    },
    task::JoinHandle,
};

use crate::{
    core::{
        error::InteractionError,
        stream::{
            MessageStream,
            StreamMessage,
        },
    },
    message::{
        common::abort_message_for_error,
        message::Message,
    },
    serializer::serializer::Serializer,
    transport::transport::Transport,
};

/// A handle to an asynchronously-running [`Service`].
pub struct ServiceHandle {
    start_handle: JoinHandle<()>,
    cancel_tx: broadcast::Sender<()>,
    message_tx: UnboundedSender<Message>,
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
    pub fn message_tx(&self) -> UnboundedSender<Message> {
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
    stream: MessageStream,
    message_tx: broadcast::Sender<Message>,
    end_tx: broadcast::Sender<()>,
    _end_rx: broadcast::Receiver<()>,
    cancel_tx: broadcast::Sender<()>,
    cancel_rx: broadcast::Receiver<()>,

    user_message_tx: UnboundedSender<Message>,
    user_message_rx: UnboundedReceiver<Message>,
}

impl Service {
    /// Creates a new service with the given transport and serialization.
    pub fn new(
        name: String,
        transport: Box<dyn Transport>,
        serializer: Box<dyn Serializer>,
    ) -> Self {
        let stream = MessageStream::new(transport, serializer);
        let (message_tx, _) = broadcast::channel(16);
        let (end_tx, end_rx) = broadcast::channel(1);
        let (cancel_tx, cancel_rx) = broadcast::channel(1);
        let (user_message_tx, user_message_rx) = unbounded_channel();
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

    async fn run(mut self) {
        if let Err(err) = self.service_loop().await {
            error!("Service {} failed: {err}", self.name);
        }
        if let Err(err) = self.end().await {
            error!("Failed to end service {}: {err}", self.name);
        }
    }

    async fn service_loop(&mut self) -> Result<()> {
        loop {
            tokio::select! {
                message = self.stream.next() => {
                    match message {
                        Some(Ok(StreamMessage::Ping(data))) => {
                            // Ping the message back.
                            self.stream.send(StreamMessage::Ping(data)).await?;
                        },
                        Some(Ok(StreamMessage::Message(message))) => {
                            // Send the message out for handling.
                            self.message_tx.send(message)?;
                        }
                        Some(Err(err)) => {
                            // Failed to parse the message.
                            //
                            // Inject an ABORT message at this layer, since the stream will be abruptly closed, and we have no way of determining what the downstream intent was.
                            //
                            // Ignore the error because the stream may be closed.
                            self.stream.send(StreamMessage::Message(abort_message_for_error(&InteractionError::ProtocolViolation("stream abruptly closed".to_owned()).into()))).await.ok();
                            return Err(err);
                        }
                        None => {
                            return Ok(());
                        }
                    }
                }
                message = self.user_message_rx.recv() => {
                    match message {
                        Some(message) => {
                            self.stream.send(StreamMessage::Message(message)).await?;
                        }
                        None => {
                            return Err(Error::msg("user message stream closed"));
                        }
                    }
                }
                // We expect that cancellation is the correct way to cleanly exit the service.
                _ = self.cancel_rx.recv() => {
                    return Ok(());
                }
                // Timeout is implemented at this layer so that ping messages are considered
                // for keeping the connection alive.
                //
                // Notice that we do not close the connection nicely.
                _ = tokio::time::sleep(Duration::from_secs(300)) => {
                    return Err(Error::msg("timed out"));
                }
            }
        }
    }

    async fn end(&mut self) -> Result<()> {
        // Ignore error with the stream, since it may already be closed.
        self.stream.close().await.ok();
        self.end_tx.send(())?;
        Ok(())
    }
}
