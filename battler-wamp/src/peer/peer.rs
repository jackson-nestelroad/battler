use std::sync::Arc;

use ahash::{
    HashMap,
    HashSet,
};
use anyhow::{
    Error,
    Result,
};
use battler_wamp_values::{
    Dictionary,
    List,
    Value,
    WampSerialize,
};
use futures_util::{
    lock::Mutex,
    Stream,
    StreamExt,
};
use log::{
    error,
    info,
};
use thiserror::Error;
use tokio::{
    sync::{
        broadcast::{
            self,
            error::RecvError,
        },
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
        cancel::CallCancelMode,
        close::CloseReason,
        error::ChannelTransmittableResult,
        features::{
            PubSubFeatures,
            RpcFeatures,
        },
        id::{
            Id,
            IdAllocator,
            SequentialIdAllocator,
        },
        roles::{
            PeerRole,
            PeerRoles,
        },
        service::{
            Service,
            ServiceHandle,
        },
        stream::{
            MessageStream,
            TransportMessageStream,
        },
        uri::Uri,
    },
    message::{
        common::goodbye_with_close_reason,
        message::{
            CallMessage,
            CancelMessage,
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
            peer_session_message,
            Event,
            ProcedureMessage,
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
    /// The message receiver channel.
    pub procedure_message_rx: broadcast::Receiver<ProcedureMessage>,
}

impl Clone for Procedure {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            procedure_message_rx: self.procedure_message_rx.resubscribe(),
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
    pub progress: bool,
}

struct PendingRpc {
    results_join_handle: JoinHandle<Result<()>>,
    result_rx: UnboundedReceiver<ChannelTransmittableResult<RpcResult>>,
    cancel_tx: UnboundedSender<CallCancelMode>,
}

impl Drop for PendingRpc {
    fn drop(&mut self) {
        self.results_join_handle.abort();
    }
}

/// A simple pending RPC, which is expected to produce one result.
pub struct SimplePendingRpc {
    pending: PendingRpc,
}

impl SimplePendingRpc {
    fn new(pending: PendingRpc) -> Self {
        Self { pending }
    }

    /// Waits for the result of the procedure call.
    pub async fn result(mut self) -> Result<RpcResult> {
        match self.pending.result_rx.recv().await {
            Some(result) => result.map_err(|err| err.into()),
            None => Err(Error::msg("procedure call finished with no result")),
        }
    }

    /// Cancels the pending call.
    pub async fn cancel(&self) -> Result<()> {
        self.pending
            .cancel_tx
            .send(CallCancelMode::KillNoWait)
            .map_err(Error::new)
    }

    /// Kills the pending call.
    ///
    /// The end error, or result, can still be read from [`Self::result`].
    pub async fn kill(&self) -> Result<()> {
        self.pending
            .cancel_tx
            .send(CallCancelMode::Kill)
            .map_err(Error::new)
    }
}

/// A progressive pending RPC, which is expected to produce one or more results.
pub struct ProgressivePendingRpc {
    pending: PendingRpc,
    done: bool,
    canceled: bool,
}

impl ProgressivePendingRpc {
    fn new(pending: PendingRpc) -> Self {
        Self {
            pending,
            done: false,
            canceled: false,
        }
    }

    // Returns true if the RPC has received all of its results.
    pub fn done(&self) -> bool {
        self.done
    }

    /// Waits for the next result of the procedure call.
    pub async fn next_result(&mut self) -> Result<Option<RpcResult>> {
        if self.done {
            return Ok(None);
        }
        match self.wait_for_next_result().await {
            Ok(result) => {
                self.done = self.canceled || !result.progress;
                Ok(Some(result))
            }
            Err(err) => {
                self.done = true;
                Err(err)
            }
        }
    }

    async fn wait_for_next_result(&mut self) -> Result<RpcResult> {
        match self.pending.result_rx.recv().await {
            Some(result) => result.map_err(|err| err.into()),
            None => Err(Error::msg("procedure call finished with no result")),
        }
    }

    /// Cancels the pending call.
    pub async fn cancel(&mut self) -> Result<()> {
        self.canceled = true;
        self.pending
            .cancel_tx
            .send(CallCancelMode::KillNoWait)
            .map_err(Error::new)
    }

    /// Kills the pending call.
    ///
    /// The end error, or result, can still be read from [`Self::result`].
    pub async fn kill(&mut self) -> Result<()> {
        self.canceled = true;
        self.pending
            .cancel_tx
            .send(CallCancelMode::Kill)
            .map_err(Error::new)
    }

    /// Wraps the pending RPC as a stream of results.
    ///
    /// The stream is finished on the last result or error.
    pub fn into_stream(self) -> impl Stream<Item = Result<RpcResult>> {
        futures_util::stream::unfold(self, move |mut rpc| async {
            match rpc.next_result().await {
                Ok(Some(result)) => Some((Ok(result), rpc)),
                Ok(None) => None,
                Err(err) => Some((Err(err), rpc)),
            }
        })
        .boxed()
    }
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

        let pub_sub_features = PubSubFeatures {};
        let rpc_features = RpcFeatures {
            call_canceling: true,
            progressive_call_results: true,
        };
        details.insert(
            "roles".to_owned(),
            PeerRoles::new(
                self.config.roles.iter().cloned(),
                pub_sub_features,
                rpc_features,
            )
            .wamp_serialize()?,
        );

        message_tx.send(Message::Hello(HelloMessage {
            realm: Uri::try_from(realm)?,
            details,
        }))?;

        let result = established_session_rx
            .recv()
            .await?
            .map_err(|err| Into::<Error>::into(err))?;
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
                                return Err(err.into());
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
                                return Err(err.into());
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
                                return Err(err.into());
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
                                    procedure_message_rx: registration.procedure_message_rx,
                                });
                            }
                        }
                        Err(err) => {
                            if err.request_id.is_some_and(|id| id == request_id) {
                                return Err(err.into());
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
                                return Err(err.into());
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

    async fn wait_for_results(
        request_id: Id,
        mut session_rpc_result_rx: broadcast::Receiver<
            ChannelTransmittableResult<peer_session_message::RpcResult>,
        >,
        mut session_finished_rx: broadcast::Receiver<()>,
        mut cancel_rx: UnboundedReceiver<CallCancelMode>,
        message_tx: UnboundedSender<Message>,
        rpc_result_tx: UnboundedSender<ChannelTransmittableResult<RpcResult>>,
    ) -> Result<()> {
        loop {
            tokio::select! {
                rpc_result = session_rpc_result_rx.recv() => {
                    match rpc_result? {
                        Ok(rpc_result) => {
                            if rpc_result.request_id == request_id {
                                rpc_result_tx.send(Ok(RpcResult {
                                    arguments: rpc_result.arguments,
                                    arguments_keyword: rpc_result.arguments_keyword,
                                    progress: rpc_result.progress,
                                }))?;
                            }
                        }
                        Err(err) => {
                            if err.request_id.is_some_and(|id| id == request_id) {
                                rpc_result_tx.send(Err(err))?;
                                break;
                            }
                        }
                    }
                }
                cancel_mode = cancel_rx.recv() => {
                    let cancel_mode = match cancel_mode {
                        Some(cancel_mode) => cancel_mode,
                        None => continue,
                    };
                    message_tx.send(Message::Cancel(CancelMessage {
                        call_request: request_id,
                        options: Dictionary::from_iter([("mode".to_owned(), Value::String(cancel_mode.into()))]),
                    }))?;
                }
                _ = session_finished_rx.recv() => {
                    rpc_result_tx.send(Err(Into::<Error>::into(PeerNotConnectedError).into()))?;
                    break;
                }
            }
        }

        Ok(())
    }

    async fn initiate_call(
        &self,
        procedure: Uri,
        rpc_call: RpcCall,
        receive_progress: bool,
    ) -> Result<PendingRpc> {
        let (message_tx, id_allocator, session_rpc_result_rx) = self
            .get_from_peer_state(|peer_state| {
                (
                    peer_state.message_tx.clone(),
                    peer_state.session.id_allocator(),
                    peer_state.session.rpc_result_rx(),
                )
            })
            .await?;
        let request_id = id_allocator.generate_id().await;

        let mut options = Dictionary::default();
        if receive_progress {
            options.insert("receive_progress".to_owned(), Value::Bool(true));
        }

        message_tx.send(Message::Call(CallMessage {
            request: request_id,
            options,
            procedure,
            arguments: rpc_call.arguments,
            arguments_keyword: rpc_call.arguments_keyword,
        }))?;

        let session_finished_rx = self.session_finished_rx();
        let (rpc_result_tx, rpc_result_rx) = unbounded_channel();
        let (cancel_tx, cancel_rx) = unbounded_channel();
        let results_join_handle = tokio::spawn(Self::wait_for_results(
            request_id,
            session_rpc_result_rx,
            session_finished_rx,
            cancel_rx,
            message_tx,
            rpc_result_tx,
        ));

        Ok(PendingRpc {
            results_join_handle,
            result_rx: rpc_result_rx,
            cancel_tx,
        })
    }

    /// Calls a procedure and waits for its result.
    pub async fn call_and_wait(&self, procedure: Uri, rpc_call: RpcCall) -> Result<RpcResult> {
        let pending = self.initiate_call(procedure, rpc_call, false).await?;
        // Wait for a single result.
        let simple = SimplePendingRpc::new(pending);
        simple.result().await
    }

    /// Calls a procedure, expecting one result.
    ///
    /// The caller can choose what to do with the pending RPC.
    pub async fn call(&self, procedure: Uri, rpc_call: RpcCall) -> Result<SimplePendingRpc> {
        let pending = self.initiate_call(procedure, rpc_call, false).await?;
        Ok(SimplePendingRpc::new(pending))
    }

    /// Calls a procedure, expecting one or more progressive results.
    ///
    /// The caller can choose what to do with the pending RPC.
    pub async fn call_with_progress(
        &self,
        procedure: Uri,
        rpc_call: RpcCall,
    ) -> Result<ProgressivePendingRpc> {
        let pending = self.initiate_call(procedure, rpc_call, true).await?;
        Ok(ProgressivePendingRpc::new(pending))
    }
}

impl<S> Drop for Peer<S> {
    fn drop(&mut self) {
        self.drop_tx.send(()).ok();
    }
}
