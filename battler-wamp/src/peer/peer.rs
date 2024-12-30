use std::sync::Arc;

use ahash::{
    HashMap,
    HashSet,
};
use anyhow::{
    Error,
    Result,
};
use futures_util::lock::Mutex;
use log::{
    error,
    info,
};
use thiserror::Error;
use tokio::sync::{
    broadcast::{
        self,
        error::RecvError,
    },
    mpsc::{
        unbounded_channel,
        UnboundedReceiver,
        UnboundedSender,
    },
};

use crate::{
    core::{
        close::CloseReason,
        id::{
            Id,
            IdAllocator,
            SequentialIdAllocator,
        },
        roles::PeerRole,
        service::{
            Service,
            ServiceHandle,
        },
        stream::{
            MessageStream,
            TransportMessageStream,
        },
        types::{
            Dictionary,
            List,
            Value,
        },
        uri::Uri,
    },
    message::{
        common::goodbye_with_close_reason,
        message::{
            CallMessage,
            HelloMessage,
            Message,
            PublishMessage,
            RegisterMessage,
            SubscribeMessage,
            UnregisterMessage,
            UnsubscribeMessage,
        },
    },
    peer::{
        connector::connector::ConnectorFactory,
        session::{
            Event,
            Invocation,
            Session,
            SessionHandle,
        },
    },
    serializer::serializer::{
        new_serializer,
        SerializerType,
    },
    transport::transport::TransportFactory,
};

const DEFAULT_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "-", env!("CARGO_PKG_VERSION"));

/// Configuration for WebSocket-specific WAMP connections.
#[derive(Debug, Default)]
pub struct WebSocketConfig {
    /// Additional headers to include in the WebSocket handshake request.
    pub headers: HashMap<String, String>,
}

/// Configuration for a [`Peer`].
#[derive(Debug)]
pub struct PeerConfig {
    /// Name of the peer, mostly for logging.
    pub name: String,
    /// Agent name, communicated to the router.
    pub agent: String,
    /// Roles implemented by the peer.
    pub roles: HashSet<PeerRole>,
    /// Allowed serializers.
    ///
    /// The actual serializer will be selected when the connection with the router is established.
    pub serializers: HashSet<SerializerType>,
    /// Additional configuration for WebSocket-specific connections.
    pub web_socket: Option<WebSocketConfig>,
}

impl PeerConfig {
    fn validate(&self) -> Result<()> {
        if self.serializers.is_empty() {
            return Err(Error::msg("at least one serializer is required"));
        }
        Ok(())
    }
}

impl Default for PeerConfig {
    fn default() -> Self {
        Self {
            name: DEFAULT_AGENT.to_owned(),
            agent: DEFAULT_AGENT.to_owned(),
            roles: HashSet::from_iter([
                PeerRole::Callee,
                PeerRole::Caller,
                PeerRole::Publisher,
                PeerRole::Subscriber,
            ]),
            serializers: HashSet::from_iter([SerializerType::Json, SerializerType::MessagePack]),
            web_socket: None,
        }
    }
}

struct PeerState {
    service: ServiceHandle,
    session: SessionHandle,

    message_tx: UnboundedSender<Message>,
}

/// A subscription to a topic.
#[derive(Debug)]
pub struct Subscription {
    /// The subscription ID.
    pub id: Id,
    /// The event receiver channel.
    pub event_rx: broadcast::Receiver<Event>,
}

/// A registration of a procedure.
#[derive(Debug)]
pub struct Procedure {
    /// The registration ID.
    pub id: Id,
    /// The invocation receiver channel.
    pub invocation_rx: broadcast::Receiver<Invocation>,
}

impl Clone for Procedure {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            invocation_rx: self.invocation_rx.resubscribe(),
        }
    }
}

/// A procedure call.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RpcCall {
    pub arguments: List,
    pub arguments_keyword: Dictionary,
}

/// A result of a procedure call.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RpcResult {
    pub arguments: List,
    pub arguments_keyword: Dictionary,
}

#[derive(Debug, Error)]
#[error("peer is not connected")]
pub struct PeerNotConnectedError;

/// A WAMP peer (a.k.a., client) that connects to a WAMP router, establishes sessions in a realm,
/// and interacts with resources in the realm.
pub struct Peer<S> {
    config: PeerConfig,
    connector_factory: Box<dyn ConnectorFactory<S>>,
    transport_factory: Box<dyn TransportFactory<S>>,
    #[allow(unused)]
    id_allocator: Box<dyn IdAllocator>,

    session_finished_tx: broadcast::Sender<()>,
    drop_tx: broadcast::Sender<()>,

    peer_state: Arc<Mutex<Option<PeerState>>>,
}

impl<S> Peer<S>
where
    S: Send + 'static,
{
    /// Creates a new peer.
    pub fn new(
        config: PeerConfig,
        connector_factory: Box<dyn ConnectorFactory<S>>,
        transport_factory: Box<dyn TransportFactory<S>>,
    ) -> Result<Self> {
        config.validate()?;
        let (session_finished_tx, _) = broadcast::channel(16);
        let (drop_tx, _) = broadcast::channel(1);
        Ok(Self {
            config,
            connector_factory,
            transport_factory,
            id_allocator: Box::new(SequentialIdAllocator::default()),
            session_finished_tx,
            drop_tx,
            peer_state: Arc::new(Mutex::new(None)),
        })
    }

    /// Receiver channel for a single session finishing, for reconnection logic.
    pub fn session_finished_rx(&self) -> broadcast::Receiver<()> {
        self.session_finished_tx.subscribe()
    }

    /// The current session ID, as given by the router.
    ///
    /// Since a peer is reused across multiple router sessions, this ID is subject to change at any
    /// point.
    pub async fn current_session_id(&self) -> Option<Id> {
        self.get_from_peer_state_async(async |peer_state: &PeerState| {
            peer_state.session.current_session_id().await
        })
        .await
        .ok()
        .flatten()
    }

    /// Connects to a router.
    ///
    /// This method merely establishes a network connection with the router. It does not establish
    /// any WAMP session. This allows the underlying network connection to be reused across multiple
    /// WAMP sessions, if the router allows.
    ///
    /// The connection and message service is maintained asynchronously. If the peer loses
    /// connection to the router, the connection is dropped in the background and methods depending
    /// on the connection will fail. The peer can reconnect to the router by calling this method
    /// again.
    pub async fn connect(&self, uri: &str) -> Result<()> {
        let connector = self.connector_factory.new_connector();
        let connection = connector.connect(&self.config, uri).await?;
        info!(
            "WAMP connection established with {uri} for peer {}",
            self.config.name
        );

        let serializer = new_serializer(connection.serializer);
        let transport = self
            .transport_factory
            .new_transport(connection.stream, connection.serializer);
        self.direct_connect(Box::new(TransportMessageStream::new(transport, serializer)))
            .await
    }

    /// Directly connects to a router with the given message stream.
    pub async fn direct_connect(&self, stream: Box<dyn MessageStream>) -> Result<()> {
        // Start the service and message handler.
        let service = Service::new(self.config.name.clone(), stream);
        let (message_tx, message_rx) = unbounded_channel();
        let service_message_rx = service.message_rx();
        let end_rx = service.end_rx();
        let drop_rx = self.drop_tx.subscribe();

        let service_handle = service.start();

        let session = Session::new(self.config.name.clone(), service_handle.message_tx());
        let session_handle = session.session_handle();

        let mut peer_state = self.peer_state.lock().await;
        *peer_state = Some(PeerState {
            service: service_handle,
            session: session_handle,
            message_tx,
        });

        tokio::spawn(Self::message_handler(
            session,
            self.peer_state.clone(),
            self.session_finished_tx.clone(),
            message_rx,
            service_message_rx,
            end_rx,
            drop_rx,
        ));

        Ok(())
    }

    async fn message_handler(
        mut session: Session,
        peer_state: Arc<Mutex<Option<PeerState>>>,
        session_finished_tx: broadcast::Sender<()>,
        mut message_rx: UnboundedReceiver<Message>,
        service_message_rx: broadcast::Receiver<Message>,
        end_rx: broadcast::Receiver<()>,
        drop_rx: broadcast::Receiver<()>,
    ) {
        loop {
            let result = Self::session_loop_with_errors(
                &mut session,
                &mut message_rx,
                service_message_rx.resubscribe(),
                end_rx.resubscribe(),
                drop_rx.resubscribe(),
            )
            .await;

            // Notify the outside world that a session finished, for reconnection logic.
            session_finished_tx.send(()).ok();

            match result {
                Ok(done) => {
                    info!("Peer session {} finished", session.name());
                    if !done {
                        continue;
                    }
                }
                Err(err) => {
                    error!("Peer session {} failed: {err:#}", session.name());
                }
            }

            info!(
                "Peer session {} is disconnecting from the router",
                session.name()
            );
            break;
        }

        peer_state.lock().await.take();
    }

    async fn session_loop_with_errors(
        session: &mut Session,
        message_rx: &mut UnboundedReceiver<Message>,
        mut service_message_rx: broadcast::Receiver<Message>,
        mut end_rx: broadcast::Receiver<()>,
        mut drop_rx: broadcast::Receiver<()>,
    ) -> Result<bool> {
        let mut finish_on_close = false;
        loop {
            tokio::select! {
                // Received a message from this peer object.
                message = message_rx.recv() => {
                    let message = match message {
                        Some(message) => message,
                        None => return Err(Error::msg("failed to receive message from peer channel (channel unexpectedly closed)")),
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
                    let message_name = message.message_name();
                    if let Err(err) = session.handle_message(message).await {
                        return Err(err.context(format!("failed to handle {message_name} message")));
                    }
                }
                // Service ended, which is unexpected.
                //
                // The service is intended to wrap the session's entire lifecycle.
                _ = end_rx.recv() => return Err(Error::msg("service ended abruptly")),
                // Peer was dropped, which is unexpected.
                _ = drop_rx.recv() => return Err(Error::msg("peer dropped unexpectedly")),
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

    async fn get_from_peer_state<F, T>(&self, f: F) -> Result<T, Error>
    where
        F: Fn(&PeerState) -> T,
    {
        match self.peer_state.lock().await.as_ref() {
            Some(peer_state) => Ok(f(peer_state)),
            None => Err(PeerNotConnectedError.into()),
        }
    }

    async fn get_from_peer_state_async<F, T>(&self, f: F) -> Result<T, Error>
    where
        F: AsyncFn(&PeerState) -> T,
    {
        match self.peer_state.lock().await.as_ref() {
            Some(peer_state) => Ok(f(peer_state).await),
            None => Err(PeerNotConnectedError.into()),
        }
    }

    /// Joins the realm, establishing a WAMP session.
    ///
    /// The session exists for as long as the router allows it to. The session will be lost in the
    /// following scenarios:
    /// 1. [`Self::leave_realm`] is called.
    /// 1. The router terminates the session due to an error.
    /// 1. The underlying connection to the router is lost.
    ///
    /// To join a different realm, [`Self::leave_realm`] should be called first.
    pub async fn join_realm(&self, realm: &str) -> Result<()> {
        let (message_tx, mut established_session_rx) = self
            .get_from_peer_state(|peer_state| {
                (
                    peer_state.message_tx.clone(),
                    peer_state.session.established_session_rx(),
                )
            })
            .await?;

        let mut details = Dictionary::default();
        details.insert("agent".to_owned(), Value::String(self.config.agent.clone()));
        details.insert(
            "roles".to_owned(),
            Value::Dictionary(
                self.config
                    .roles
                    .iter()
                    .map(|role| {
                        (
                            role.key_for_details().to_owned(),
                            Value::Dictionary(Dictionary::default()),
                        )
                    })
                    .collect(),
            ),
        );

        message_tx.send(Message::Hello(HelloMessage {
            realm: Uri::try_from(realm)?,
            details,
        }))?;

        let result = established_session_rx
            .recv()
            .await?
            .map_err(|err| err.into_error())?;
        if result.realm.as_ref() != realm {
            return Err(Error::msg(format!(
                "joined realm {}, expected {realm}",
                result.realm
            )));
        }

        Ok(())
    }

    /// Leaves the realm, closing the WAMP session.
    pub async fn leave_realm(&self) -> Result<()> {
        let (message_tx, mut closed_session_rx) = self
            .get_from_peer_state(|peer_state| {
                (
                    peer_state.message_tx.clone(),
                    peer_state.session.closed_session_rx(),
                )
            })
            .await?;

        message_tx.send(goodbye_with_close_reason(CloseReason::Normal))?;
        closed_session_rx.recv().await?;
        Ok(())
    }

    /// Disconnects from the router.
    pub async fn disconnect(&self) -> Result<()> {
        let mut peer_state = self.peer_state.lock().await;

        match peer_state.take() {
            Some(peer_state) => {
                info!(
                    "Peer {} was instructed to disconnect from the router",
                    self.config.name
                );
                peer_state.service.cancel()?;
                peer_state.service.join().await?;
            }
            None => (),
        }
        Ok(())
    }

    /// Subscribes to a topic in the realm.
    ///
    /// The resulting subscription contains an event receiver stream for published events. The
    /// stream automatically closes when the peer unsubscribes from the topic or when the session
    /// ends.
    pub async fn subscribe(&self, topic: Uri) -> Result<Subscription> {
        let (message_tx, id_allocator, mut subscribed_rx) = self
            .get_from_peer_state(|peer_state| {
                (
                    peer_state.message_tx.clone(),
                    peer_state.session.id_allocator(),
                    peer_state.session.subscribed_rx(),
                )
            })
            .await?;
        let request_id = id_allocator.generate_id().await;

        message_tx.send(Message::Subscribe(SubscribeMessage {
            request: request_id,
            options: Dictionary::default(),
            topic,
        }))?;

        let mut session_finished_rx = self.session_finished_rx();
        loop {
            tokio::select! {
                subscription = subscribed_rx.recv() => {
                    match subscription? {
                        Ok(subscription) => {
                            if subscription.request_id == request_id {
                                return Ok(Subscription {
                                    id: subscription.subscription_id,
                                    event_rx: subscription.event_rx,
                                });
                            }
                        }
                        Err(err) => {
                            if err.request_id.is_some_and(|id| id == request_id) {
                                return Err(err.into_error());
                            }
                        }
                    }
                }
                _ = session_finished_rx.recv() => {
                    return Err(PeerNotConnectedError.into());
                }
            }
        }
    }

    /// Removes a subscription.
    ///
    /// The subscription ID is received after subscribing to the topic.
    pub async fn unsubscribe(&self, id: Id) -> Result<()> {
        let (message_tx, id_allocator, mut unsubscribed_rx) = self
            .get_from_peer_state(|peer_state| {
                (
                    peer_state.message_tx.clone(),
                    peer_state.session.id_allocator(),
                    peer_state.session.unsubscribed_rx(),
                )
            })
            .await?;
        let request_id = id_allocator.generate_id().await;

        message_tx.send(Message::Unsubscribe(UnsubscribeMessage {
            request: request_id,
            subscribed_subscription: id,
        }))?;

        let mut session_finished_rx = self.session_finished_rx();
        loop {
            tokio::select! {
                unsubscription = unsubscribed_rx.recv() => {
                    match unsubscription? {
                        Ok(unsubscription) => {
                            if unsubscription.request_id == request_id {
                                return Ok(());
                            }
                        }
                        Err(err) => {
                            if err.request_id.is_some_and(|id| id == request_id) {
                                return Err(err.into_error());
                            }
                        }
                    }
                }
                _ = session_finished_rx.recv() => {
                    return Err(PeerNotConnectedError.into());
                }
            }
        }
    }

    /// Publishes an event to a topic.
    pub async fn publish(&self, topic: Uri, event: Event) -> Result<()> {
        let (message_tx, id_allocator, mut published_rx) = self
            .get_from_peer_state(|peer_state| {
                (
                    peer_state.message_tx.clone(),
                    peer_state.session.id_allocator(),
                    peer_state.session.published_rx(),
                )
            })
            .await?;
        let request_id = id_allocator.generate_id().await;

        message_tx.send(Message::Publish(PublishMessage {
            request: request_id,
            options: Dictionary::default(),
            topic,
            arguments: event.arguments,
            arguments_keyword: event.arguments_keyword,
        }))?;

        let mut session_finished_rx = self.session_finished_rx();
        loop {
            tokio::select! {
                publication = published_rx.recv() => {
                    match publication? {
                        Ok(publication) => {
                            if publication.request_id == request_id {
                                return Ok(());
                            }
                        }
                        Err(err) => {
                            if err.request_id.is_some_and(|id| id == request_id) {
                                return Err(err.into_error());
                            }
                        }
                    }
                }
                _ = session_finished_rx.recv() => {
                    return Err(PeerNotConnectedError.into());
                }
            }
        }
    }

    /// Registers a procedure to an endpoint.
    ///
    /// The resulting procedure contains an invocation receiver stream. The stream automatically
    /// closes when the peer deregisters the procedure or when the session ends.
    pub async fn register(&self, procedure: Uri) -> Result<Procedure> {
        let (message_tx, id_allocator, mut registered_rx) = self
            .get_from_peer_state(|peer_state| {
                (
                    peer_state.message_tx.clone(),
                    peer_state.session.id_allocator(),
                    peer_state.session.registered_rx(),
                )
            })
            .await?;
        let request_id = id_allocator.generate_id().await;

        message_tx.send(Message::Register(RegisterMessage {
            request: request_id,
            options: Dictionary::default(),
            procedure,
        }))?;

        let mut session_finished_rx = self.session_finished_rx();
        loop {
            tokio::select! {
                registration = registered_rx.recv() => {
                    match registration? {
                        Ok(registration) => {
                            if registration.request_id == request_id {
                                return Ok(Procedure {
                                    id: registration.registration_id,
                                    invocation_rx: registration.invocation_rx,
                                });
                            }
                        }
                        Err(err) => {
                            if err.request_id.is_some_and(|id| id == request_id) {
                                return Err(err.into_error());
                            }
                        }
                    }
                }
                _ = session_finished_rx.recv() => {
                    return Err(PeerNotConnectedError.into());
                }
            }
        }
    }

    /// Removes a procedure.
    ///
    /// The registration ID is received after registering the procedure.
    pub async fn unregister(&self, id: Id) -> Result<()> {
        let (message_tx, id_allocator, mut unregistered_rx) = self
            .get_from_peer_state(|peer_state| {
                (
                    peer_state.message_tx.clone(),
                    peer_state.session.id_allocator(),
                    peer_state.session.unregistered_rx(),
                )
            })
            .await?;
        let request_id = id_allocator.generate_id().await;

        message_tx.send(Message::Unregister(UnregisterMessage {
            request: request_id,
            registered_registration: id,
        }))?;

        let mut session_finished_rx = self.session_finished_rx();
        loop {
            tokio::select! {
                unregistration = unregistered_rx.recv() => {
                    match unregistration? {
                        Ok(unregistration) => {
                            if unregistration.request_id == request_id {
                                return Ok(());
                            }
                        }
                        Err(err) => {
                            if err.request_id.is_some_and(|id| id == request_id) {
                                return Err(err.into_error());
                            }
                        }
                    }
                }
                _ = session_finished_rx.recv() => {
                    return Err(PeerNotConnectedError.into());
                }
            }
        }
    }
    /// Calls a procedure.
    pub async fn call(&self, procedure: Uri, rpc_call: RpcCall) -> Result<RpcResult> {
        let (message_tx, id_allocator, mut rpc_result_rx) = self
            .get_from_peer_state(|peer_state| {
                (
                    peer_state.message_tx.clone(),
                    peer_state.session.id_allocator(),
                    peer_state.session.rpc_result_rx(),
                )
            })
            .await?;
        let request_id = id_allocator.generate_id().await;

        message_tx.send(Message::Call(CallMessage {
            request: request_id,
            options: Dictionary::default(),
            procedure,
            arguments: rpc_call.arguments,
            arguments_keyword: rpc_call.arguments_keyword,
        }))?;

        let mut session_finished_rx = self.session_finished_rx();
        loop {
            tokio::select! {
                rpc_result = rpc_result_rx.recv() => {
                    match rpc_result? {
                        Ok(rpc_result) => {
                            if rpc_result.request_id == request_id {
                                return Ok(RpcResult {
                                    arguments: rpc_result.arguments,
                                    arguments_keyword: rpc_result.arguments_keyword,
                                });
                            }
                        }
                        Err(err) => {
                            if err.request_id.is_some_and(|id| id == request_id) {
                                return Err(err.into_error());
                            }
                        }
                    }
                }
                _ = session_finished_rx.recv() => {
                    return Err(PeerNotConnectedError.into());
                }
            }
        }
    }
}

impl<S> Drop for Peer<S> {
    fn drop(&mut self) {
        self.drop_tx.send(()).ok();
    }
}
