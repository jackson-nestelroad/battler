use std::sync::Arc;

use anyhow::{
    Error,
    Result,
};
use log::{
    error,
    info,
};
use tokio::sync::{
    broadcast::{
        self,
        error::RecvError,
    },
    mpsc,
};
use uuid::Uuid;

use crate::{
    core::{
        error::ChannelTransmittableResult,
        id::Id,
        peer_info::ConnectionType,
        service::Service,
    },
    message::message::Message,
    router::{
        context::RouterContext,
        session::{
            ProcedureMessage,
            Session,
        },
    },
};

/// A connection from a router to a client.
///
/// On its own, a connection is not very meaningful. When started, it uses a WAMP [`Service`] to
/// send and receive messages on an underlying transport. Messages are used to set up and manage a
/// [`Session`], which handles all interactions with the router.
#[derive(Debug)]
pub struct Connection {
    uuid: Uuid,
}

impl Connection {
    /// Creates a new connection.
    pub fn new() -> Self {
        Self {
            uuid: Uuid::new_v4(),
        }
    }

    /// The unique identifier of the connection.
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    // Starts the connection on the runtime.
    pub fn start<S>(self, context: RouterContext<S>, service: Service) {
        tokio::spawn(self.run(context, service));
    }

    async fn run<S>(self, context: RouterContext<S>, service: Service) {
        self.run_service(&context, service).await;
    }

    async fn run_service<S>(&self, context: &RouterContext<S>, service: Service) {
        let connection_type = service.connection_type();
        let mut message_rx = service.message_rx();
        let end_rx = service.end_rx();

        let service_handle = service.start();
        loop {
            if !self
                .run_session(
                    context,
                    connection_type.clone(),
                    service_handle.message_tx(),
                    &mut message_rx,
                    end_rx.resubscribe(),
                )
                .await
            {
                continue;
            }

            info!("Connection {} will have no more sessions", self.uuid);
            break;
        }

        if let Err(err) = service_handle.cancel() {
            error!(
                "Failed to cancel service for connection {}: {err}",
                self.uuid
            );
        }

        if let Err(err) = service_handle.join().await {
            error!("Failed to join service for connection {}: {err}", self.uuid);
        }
    }

    async fn run_session<S>(
        &self,
        context: &RouterContext<S>,
        connection_type: ConnectionType,
        service_message_tx: mpsc::Sender<Message>,
        service_message_rx: &mut broadcast::Receiver<Message>,
        end_rx: broadcast::Receiver<()>,
    ) -> bool {
        let session_id = context.router().id_allocator.generate_id().await;
        let (message_tx, message_rx) = mpsc::channel(16);
        let session = Session::new(session_id, connection_type, message_tx, service_message_tx);

        info!(
            "Proactively starting router session {} for connection {}",
            session_id, self.uuid
        );

        self.session_loop(context, session, message_rx, service_message_rx, end_rx)
            .await
    }

    async fn session_loop<S>(
        &self,
        context: &RouterContext<S>,
        session: Session,
        message_rx: mpsc::Receiver<Message>,
        service_message_rx: &mut broadcast::Receiver<Message>,
        end_rx: broadcast::Receiver<()>,
    ) -> bool {
        let session = Arc::new(session);
        let (session_loop_done_tx, session_loop_done_rx) = broadcast::channel(1);
        let done = match self
            .session_loop_with_errors(
                context,
                session.clone(),
                message_rx,
                service_message_rx,
                end_rx,
                session_loop_done_rx,
            )
            .await
        {
            Ok(done) => {
                info!(
                    "Router session {} for connection {} finished",
                    self.uuid,
                    session.id()
                );
                done
            }
            Err(err) => {
                error!(
                    "Router session {} for connection {} failed: {err:#}",
                    self.uuid,
                    session.id()
                );
                true
            }
        };

        session_loop_done_tx.send(()).ok();
        session.clean_up(context).await;
        done
    }

    async fn handle_message<S>(
        context: RouterContext<S>,
        session: Arc<Session>,
        message: Message,
        handle_message_result_tx: mpsc::Sender<ChannelTransmittableResult<()>>,
    ) {
        let message_name = message.message_name();
        handle_message_result_tx
            .send(
                session
                    .handle_message(context.clone(), message)
                    .await
                    .map_err(|err| {
                        err.context(format!("failed to handle {message_name} message"))
                            .into()
                    }),
            )
            .await
            .ok();
    }

    async fn publish_loop<S>(
        context: RouterContext<S>,
        session: Arc<Session>,
        session_loop_done_rx: broadcast::Receiver<()>,
        handle_message_result_tx: mpsc::Sender<ChannelTransmittableResult<()>>,
    ) {
        if let Err(err) =
            Self::publish_loop_with_errors(context, session, session_loop_done_rx).await
        {
            handle_message_result_tx.send(Err(err.into())).await.ok();
        }
    }

    async fn publish_loop_with_errors<S>(
        context: RouterContext<S>,
        session: Arc<Session>,
        mut session_loop_done_rx: broadcast::Receiver<()>,
    ) -> Result<()> {
        let mut publish_rx = session.publish_rx();
        loop {
            tokio::select! {
                // Received a publish message.
                publish_message = publish_rx.recv() => {
                    session.handle_ordered_publish(&context, publish_message?).await?;
                }
                // The session loop is done, so we should also be done.
                _ = session_loop_done_rx.recv() => {
                    break;
                }
            }
        }
        Ok(())
    }

    async fn call_loop<S>(
        context: RouterContext<S>,
        session: Arc<Session>,
        session_loop_done_rx: broadcast::Receiver<()>,
        handle_message_result_tx: mpsc::Sender<ChannelTransmittableResult<()>>,
    ) {
        if let Err(err) = Self::call_loop_with_errors(
            context,
            session,
            session_loop_done_rx,
            handle_message_result_tx.clone(),
        )
        .await
        {
            handle_message_result_tx.send(Err(err.into())).await.ok();
        }
    }

    async fn handle_invocation<S>(
        context: RouterContext<S>,
        session: Arc<Session>,
        call_request_id: Id,
        handle_message_result_tx: mpsc::Sender<ChannelTransmittableResult<()>>,
    ) {
        if let Err(err) = session.handle_invocation(&context, call_request_id).await {
            handle_message_result_tx.send(Err(err.into())).await.ok();
        }
    }

    async fn call_loop_with_errors<S>(
        context: RouterContext<S>,
        session: Arc<Session>,
        mut session_loop_done_rx: broadcast::Receiver<()>,
        handle_message_result_tx: mpsc::Sender<ChannelTransmittableResult<()>>,
    ) -> Result<()> {
        let mut procedure_message_rx = session.procedure_message_rx();
        loop {
            tokio::select! {
                // Received an ordered message.
                message = procedure_message_rx.recv() => {
                    match message? {
                        ProcedureMessage::Call(call_message) => {
                            let call_request_id = match session.handle_ordered_call(&context, call_message).await? {
                                Some(call_request_id) => call_request_id,
                                None => continue,
                            };
                            // Handle the invocation asynchronously.
                            tokio::spawn(Self::handle_invocation(context.clone(), session.clone(), call_request_id, handle_message_result_tx.clone()));
                        },
                        ProcedureMessage::Cancel(cancel_message) => {
                            session.handle_ordered_cancel(&context, cancel_message).await?;
                        }
                    }
                }
                // The session loop is done, so we should also be done.
                _ = session_loop_done_rx.recv() => {
                    break;
                }
            }
        }
        Ok(())
    }

    async fn session_loop_with_errors<S>(
        &self,
        context: &RouterContext<S>,
        session: Arc<Session>,
        mut message_rx: mpsc::Receiver<Message>,
        service_message_rx: &mut broadcast::Receiver<Message>,
        mut end_rx: broadcast::Receiver<()>,
        session_loop_done_rx: broadcast::Receiver<()>,
    ) -> Result<bool> {
        let mut finish_on_close = false;
        let mut router_end_rx = context.router().end_rx();
        let (handle_message_result_tx, mut handle_message_result_rx) = mpsc::channel(16);

        // Start two separate loops for ordering guarantees of PUBLISH and CALL messages.
        tokio::spawn(Self::publish_loop(
            context.clone(),
            session.clone(),
            session_loop_done_rx.resubscribe(),
            handle_message_result_tx.clone(),
        ));
        tokio::spawn(Self::call_loop(
            context.clone(),
            session.clone(),
            session_loop_done_rx.resubscribe(),
            handle_message_result_tx.clone(),
        ));

        loop {
            tokio::select! {
                // Received a message from some part of the router.
                message = message_rx.recv() => {
                    let message = match message {
                        Some(message) => message,
                        None => return Err(Error::msg("failed to receive message from connection channel")),
                    };
                    let message_name = message.message_name();
                    if let Err(err) = session.send_message(message).await {
                        return Err(err.context(format!("failed to send {message_name} message")));
                    }
                }
                // Received a message from the service.
                message = service_message_rx.recv() => {
                    let message = match message {
                        Ok(message) => message,
                        Err(RecvError::Closed) => return Ok(true),
                        Err(err) => return Err(Error::context(err.into(), "failed to receive message")),
                    };

                    Self::handle_message(context.clone(), session.clone(), message, handle_message_result_tx.clone()).await;
                }
                // Finished handling a message.
                result = handle_message_result_rx.recv() => {
                    match result {
                        Some(Ok(())) => (),
                        Some(Err(err)) => return Err(err.into()),
                        None => return Err(Error::msg("handle_message_error_rx unexpectedly closed")),
                    }
                }
                // Service ended, which is unexpected.
                //
                // The service is intended to wrap the session's entire lifecycle.
                _ = end_rx.recv() => return Err(Error::msg("service ended abruptly")),
                // Router ended, which is unexpected.
                //
                // The router should shut down realms and sessions, which sends ABORT to downstream clients.
                // In this scenario, the session would exit cleanly. However, somehow the server ended already.
                // Since this is unexpected, terminating the connection abruptly is OK.
                _ = router_end_rx.recv() => return Err(Error::msg("router ended abruptly")),
            }

            if session.closed().await {
                if finish_on_close {
                    break;
                }
            } else {
                finish_on_close = true;
            }
        }

        Ok(false)
    }
}
