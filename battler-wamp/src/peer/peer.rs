use std::{
    sync::{
        Arc,
        Weak,
    },
    time::Duration,
};

use ahash::{
    HashMap,
    HashSet,
};
use anyhow::{
    Error,
    Result,
};
use battler_wamp_uri::{
    Uri,
    WildcardUri,
};
use battler_wamp_values::{
    Dictionary,
    List,
    Value,
    WampSerialize,
};
use futures_util::{
    Stream,
    StreamExt,
    lock::Mutex,
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
        mpsc,
    },
    task::JoinHandle,
};

use crate::{
    auth::{
        AuthMethod,
        GenericClientAuthenticator,
        make_generic_client_authenticator,
        scram,
        undisputed,
    },
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
        invocation_policy::InvocationPolicy,
        match_style::MatchStyle,
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
    },
    message::{
        common::{
            abort_message_for_error,
            goodbye_with_close_reason,
        },
        message::{
            AuthenticateMessage,
            CallMessage,
            CancelMessage,
            ChallengeMessage,
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
        ConnectorFactory,
        session::{
            ProcedureMessage,
            PublishedEvent,
            ReceivedEvent,
            Session,
            SessionHandle,
            peer_session_message,
        },
    },
    serializer::serializer::{
        SerializerType,
        new_serializer,
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

/// Configuration for a [`Peer`] acting as a callee.
#[derive(Debug, Default)]
pub struct CalleeConfig {
    /// The callee can enforce timeouts for procedure invocations.
    pub enforce_timeouts: bool,
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
    /// Additional configuration for the callee role.
    ///
    /// Ignored if [`PeerRole::Callee`] is not added to [`Self::roles`].
    pub callee: CalleeConfig,
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
            callee: CalleeConfig::default(),
        }
    }
}

/// Options for subscribing to a topic.
#[derive(Debug, Default, Clone)]
pub struct SubscriptionOptions {
    /// How the subscription should be matched for published events.
    pub match_style: Option<MatchStyle>,
}

/// Options for registering a procedure.
#[derive(Debug, Default, Clone)]
pub struct ProcedureOptions {
    /// How the procedure should be matched for procedure calls.
    pub match_style: Option<MatchStyle>,
    /// How a callee should be selected for invocations.
    pub invocation_policy: InvocationPolicy,
    /// The caller's identity should be disclosed.
    pub disclose_caller: bool,
}

struct PeerState {
    service: ServiceHandle,
    session: SessionHandle,

    message_tx: mpsc::Sender<Message>,
    session_references: Arc<()>,
}

/// A subscription to a topic.
#[derive(Debug)]
pub struct Subscription {
    /// The subscription ID.
    pub id: Id,
    /// The event receiver channel.
    pub event_rx: broadcast::Receiver<ReceivedEvent>,
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
    pub timeout: Option<Duration>,
}

/// A result of a procedure call.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RpcResult {
    pub arguments: List,
    pub arguments_keyword: Dictionary,
    pub progress: bool,
}

#[derive(Debug)]
struct PendingRpc {
    results_join_handle: JoinHandle<Result<()>>,
    result_rx: mpsc::Receiver<ChannelTransmittableResult<RpcResult>>,
    cancel_tx: mpsc::Sender<CallCancelMode>,
}

impl Drop for PendingRpc {
    fn drop(&mut self) {
        self.results_join_handle.abort();
    }
}

/// A simple pending RPC, which is expected to produce one result.
#[derive(Debug)]
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
            .await
            .map_err(Error::new)
    }

    /// Kills the pending call.
    ///
    /// The end error, or result, can still be read from [`Self::result`].
    pub async fn kill(&self) -> Result<()> {
        self.pending
            .cancel_tx
            .send(CallCancelMode::Kill)
            .await
            .map_err(Error::new)
    }
}

/// A progressive pending RPC, which is expected to produce one or more results.
#[derive(Debug)]
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
        // Do not set the canceled flag, since we expect the router to send the final error.
        self.pending
            .cancel_tx
            .send(CallCancelMode::KillNoWait)
            .await
            .map_err(Error::new)
    }

    /// Kills the pending call.
    ///
    /// The end error, or result, can still be read from [`Self::next_result`].
    pub async fn kill(&mut self) -> Result<()> {
        // Set the canceled flag, since whatever the last result is will be the termination of this
        // call, even if it is not an error (in the case that the callee finishes the invocation).
        self.canceled = true;
        self.pending
            .cancel_tx
            .send(CallCancelMode::Kill)
            .await
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

/// Error for a peer not being connected for some operation.
#[derive(Debug, Error)]
#[error("peer is not connected")]
pub struct PeerNotConnectedError;

/// Supported authentication types for a peer.
#[derive(Debug, Clone)]
pub enum SupportedAuthMethod {
    /// WAMP-SCRAM.
    WampScram { id: String, password: String },
    /// Undisputed.
    Undisputed { id: String, role: String },
}

impl SupportedAuthMethod {
    /// The corresponding [`AuthMethod`].
    pub fn auth_method(&self) -> AuthMethod {
        match self {
            Self::WampScram { .. } => AuthMethod::WampScram,
            Self::Undisputed { .. } => AuthMethod::Undisputed,
        }
    }

    /// Creates a new authenticator for the supported authentication method.
    pub async fn new_authenticator(&self) -> Result<Box<dyn GenericClientAuthenticator>> {
        match self {
            Self::WampScram { id, password } => Ok(make_generic_client_authenticator(Box::new(
                scram::ClientAuthenticator::new(id.clone(), password.clone()),
            ))),
            Self::Undisputed { id, role } => Ok(make_generic_client_authenticator(Box::new(
                undisputed::ClientAuthenticator::new(id.clone(), role.clone()),
            ))),
        }
    }
}

/// A WAMP peer (a.k.a., client) that connects to a WAMP router, establishes sessions in a realm,
/// and interacts with resources in the realm.
pub struct Peer<S> {
    config: PeerConfig,
    connector_factory: Box<dyn ConnectorFactory<S>>,
    transport_factory: Box<dyn TransportFactory<S>>,
    #[allow(unused)]
    id_allocator: Box<dyn IdAllocator>,

    session_finished_tx: broadcast::Sender<()>,
    connection_finished_tx: broadcast::Sender<()>,
    drop_tx: broadcast::Sender<()>,
    end_active_connection_tx: broadcast::Sender<()>,

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
        let (connection_finished_tx, _) = broadcast::channel(16);
        let (drop_tx, _) = broadcast::channel(1);
        let (end_active_connection_tx, _) = broadcast::channel(1);
        Ok(Self {
            config,
            connector_factory,
            transport_factory,
            id_allocator: Box::new(SequentialIdAllocator::default()),
            session_finished_tx,
            connection_finished_tx,
            drop_tx,
            end_active_connection_tx,
            peer_state: Arc::new(Mutex::new(None)),
        })
    }

    /// Receiver channel for a single session finishing, for reconnection logic.
    pub fn session_finished_rx(&self) -> broadcast::Receiver<()> {
        self.session_finished_tx.subscribe()
    }

    fn connection_finished_rx(&self) -> broadcast::Receiver<()> {
        self.connection_finished_tx.subscribe()
    }

    /// The current session ID, as given by the router.
    ///
    /// Since a peer is reused across multiple router sessions, this ID is subject to change at any
    /// point.
    pub async fn current_session_id(&self) -> Option<Id> {
        self.get_from_peer_state(async |peer_state: &PeerState| {
            peer_state.session.current_session_id().await
        })
        .await
        .ok()
        .map(|(_, val)| val)
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
        // End any active connection.
        self.end_active_connection_tx.send(()).ok();

        // Start the service and message handler.
        let service = Service::new(self.config.name.clone(), stream);
        let (message_tx, message_rx) = mpsc::channel(16);
        let service_message_rx = service.message_rx();
        let end_rx = service.end_rx();
        let drop_rx = self.drop_tx.subscribe();
        let end_active_connection_rx = self.end_active_connection_tx.subscribe();

        let service_handle = service.start();

        let session = Session::new(self.config.name.clone(), service_handle.message_tx());
        let session_handle = session.session_handle();

        let mut peer_state = self.peer_state.lock().await;
        let session_references = Arc::new(());
        *peer_state = Some(PeerState {
            service: service_handle,
            session: session_handle,
            message_tx,
            session_references: session_references.clone(),
        });

        tokio::spawn(Self::message_handler_awaiting_zero_references(
            session,
            self.peer_state.clone(),
            Arc::downgrade(&session_references),
            self.session_finished_tx.clone(),
            self.connection_finished_tx.clone(),
            message_rx,
            service_message_rx,
            end_rx,
            drop_rx,
            end_active_connection_rx,
        ));

        Ok(())
    }

    async fn message_handler_awaiting_zero_references(
        mut session: Session,
        peer_state: Arc<Mutex<Option<PeerState>>>,
        session_references: Weak<()>,
        session_finished_tx: broadcast::Sender<()>,
        connection_finished_tx: broadcast::Sender<()>,
        message_rx: mpsc::Receiver<Message>,
        service_message_rx: broadcast::Receiver<Message>,
        end_rx: broadcast::Receiver<()>,
        drop_rx: broadcast::Receiver<()>,
        end_active_connection_rx: broadcast::Receiver<()>,
    ) {
        // Pass all channels to this internal function, so that when the handler exits, the channels
        // close.
        Self::message_handler(
            &mut session,
            peer_state,
            session_finished_tx,
            connection_finished_tx,
            message_rx,
            service_message_rx,
            end_rx,
            drop_rx,
            end_active_connection_rx,
        )
        .await;

        let references = Weak::strong_count(&session_references);
        log::debug!(
            "Session {} has {} reference(s) after message handler loop exited",
            session.name(),
            references
        );

        // Wait for there to be no known references to the session (connection) before we exit this
        // function and drop the session object.
        if references > 0 {
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(1)) => {
                        if Weak::strong_count(&session_references) == 0 {
                            break;
                        }
                    }
                }
            }
        }
    }

    async fn message_handler(
        session: &mut Session,
        peer_state: Arc<Mutex<Option<PeerState>>>,
        session_finished_tx: broadcast::Sender<()>,
        connection_finished_tx: broadcast::Sender<()>,
        mut message_rx: mpsc::Receiver<Message>,
        service_message_rx: broadcast::Receiver<Message>,
        end_rx: broadcast::Receiver<()>,
        drop_rx: broadcast::Receiver<()>,
        mut end_active_connection_rx: broadcast::Receiver<()>,
    ) {
        loop {
            let result = Self::session_loop_with_errors(
                session,
                &mut message_rx,
                service_message_rx.resubscribe(),
                end_rx.resubscribe(),
                drop_rx.resubscribe(),
                &mut end_active_connection_rx,
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

        // No longer allow references to this session (which is reused across the entire connection)
        // to be taken.
        peer_state.lock().await.take();
        connection_finished_tx.send(()).ok();
    }

    async fn session_loop_with_errors(
        session: &mut Session,
        message_rx: &mut mpsc::Receiver<Message>,
        mut service_message_rx: broadcast::Receiver<Message>,
        mut end_rx: broadcast::Receiver<()>,
        mut drop_rx: broadcast::Receiver<()>,
        end_active_connection_rx: &mut broadcast::Receiver<()>,
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
                // Connection was overwritten with a new one.
                _ = end_active_connection_rx.recv() => return Err(Error::msg("peer started another connection")),
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

    async fn get_from_peer_state<F, T>(&self, f: F) -> Result<(Arc<()>, T), Error>
    where
        F: AsyncFn(&PeerState) -> T,
    {
        match self.peer_state.lock().await.as_ref() {
            Some(peer_state) => Ok((peer_state.session_references.clone(), f(peer_state).await)),
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
        self.join_realm_internal(realm, &[]).await
    }

    /// Joins the realm, establishing a WAMP session, with a list of supported authentication
    /// methods.
    ///
    /// Behaves the same as [`Self::join_realm`], but allows authentication to be used if challenged
    /// by the router.
    pub async fn join_realm_with_authentication(
        &self,
        realm: &str,
        auth_methods: &[SupportedAuthMethod],
    ) -> Result<()> {
        self.join_realm_internal(realm, auth_methods).await
    }

    async fn join_realm_internal(
        &self,
        realm: &str,
        auth_methods: &[SupportedAuthMethod],
    ) -> Result<()> {
        let (_reference, (message_tx, mut established_session_rx, mut auth_challenge_rx)) = self
            .get_from_peer_state(async |peer_state| {
                (
                    peer_state.message_tx.clone(),
                    peer_state.session.established_session_rx(),
                    peer_state.session.auth_challenge_rx(),
                )
            })
            .await?;

        let mut details = Dictionary::default();
        details.insert("agent".to_owned(), Value::String(self.config.agent.clone()));

        let pub_sub_features = PubSubFeatures {};
        let rpc_features = RpcFeatures {
            call_canceling: true,
            progressive_call_results: true,
            call_timeout: self.config.callee.enforce_timeouts,
            shared_registration: true,
            caller_identification: true,
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

        let mut message = HelloMessage {
            realm: Uri::try_from(realm)?,
            details,
        };

        let mut authenticators = HashMap::default();
        for auth_method in auth_methods {
            authenticators.insert(
                auth_method.auth_method(),
                auth_method.new_authenticator().await?,
            );
        }

        for (_, authenticator) in &authenticators {
            authenticator
                .hello()
                .await?
                .embed_into_hello_message(&mut message)?;
        }

        message_tx.send(Message::Hello(message)).await?;

        let mut connection_finished_rx = self.connection_finished_rx();

        let mut selected_auth_method = None;
        loop {
            tokio::select! {
                result = established_session_rx.recv() => {
                    let result = result?.map_err(|err| Into::<Error>::into(err))?;
                    match self.validate_new_established_session(result, &authenticators, realm, &selected_auth_method).await {
                        Ok(()) => break,
                        Err(err) => {
                            message_tx.send(abort_message_for_error(&err)).await?;
                            return Err(err.context("failed to validate newly established session"));
                        }
                    }
                }
                challenge = auth_challenge_rx.recv() => {
                    match self.handle_challenge(challenge?, &authenticators).await {
                        Ok((auth_method, response)) =>  {
                            message_tx.send(Message::Authenticate(response)).await?;
                            selected_auth_method = Some(auth_method);
                        },
                        Err(err) => {
                            message_tx.send(abort_message_for_error(&err)).await?;
                            return Err(err.context("failed to handle authentication challenge"));
                        },
                    }
                }
                _ = connection_finished_rx.recv() => {
                    // The reason we disconnected may have been communicated.
                    if let Ok(Err(err)) = established_session_rx.try_recv() {
                        return Err(err.into());
                    }
                    return Err(PeerNotConnectedError.into());
                }
            }
        }

        Ok(())
    }

    async fn handle_challenge(
        &self,
        challenge: ChallengeMessage,
        authenticators: &HashMap<AuthMethod, Box<dyn GenericClientAuthenticator>>,
    ) -> Result<(AuthMethod, AuthenticateMessage)> {
        let authenticator = authenticators
            .get(&challenge.auth_method)
            .ok_or_else(|| Error::msg("received unsupported auth method"))?;
        Ok((
            challenge.auth_method,
            authenticator.handle_challenge(&challenge).await?,
        ))
    }

    async fn validate_new_established_session(
        &self,
        session: peer_session_message::EstablishedSession,
        authenticators: &HashMap<AuthMethod, Box<dyn GenericClientAuthenticator>>,
        realm: &str,
        auth_method: &Option<AuthMethod>,
    ) -> Result<()> {
        if session.realm.as_ref() != realm {
            return Err(Error::msg(format!(
                "joined realm {}, expected {realm}",
                session.realm
            )));
        }
        if let Some(auth_method) = auth_method {
            let authenticator = authenticators
                .get(&auth_method)
                .ok_or_else(|| Error::msg("expected authenticator to exist"))?;
            authenticator
                .verify_signature(&session.welcome_message)
                .await?;
        }
        Ok(())
    }

    /// Leaves the realm, closing the WAMP session.
    pub async fn leave_realm(&self) -> Result<()> {
        let (_reference, (message_tx, mut closed_session_rx)) = self
            .get_from_peer_state(async |peer_state| {
                (
                    peer_state.message_tx.clone(),
                    peer_state.session.closed_session_rx(),
                )
            })
            .await?;

        message_tx
            .send(goodbye_with_close_reason(CloseReason::Normal))
            .await?;

        let mut connection_finished_rx = self.connection_finished_rx();
        tokio::select! {
            result = closed_session_rx.recv() => {
                result.map_err(Error::new)
            }
            _ = connection_finished_rx.recv() => {
                // We may have closed successfully.
                if let Ok(result) = closed_session_rx.try_recv() {
                    return Ok(result);
                }
                Err(PeerNotConnectedError.into())
            }
        }
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

    async fn subscribe_internal(
        &self,
        topic: WildcardUri,
        options: SubscriptionOptions,
    ) -> Result<Subscription> {
        let (_reference, (message_tx, id_allocator, mut subscribed_rx)) = self
            .get_from_peer_state(async |peer_state| {
                (
                    peer_state.message_tx.clone(),
                    peer_state.session.id_allocator(),
                    peer_state.session.subscribed_rx(),
                )
            })
            .await?;
        let request_id = id_allocator.generate_id().await;

        let mut message_options = Dictionary::default();
        if let Some(match_style) = options.match_style {
            message_options.insert("match".to_owned(), Value::String(match_style.into()));
        }

        message_tx
            .send(Message::Subscribe(SubscribeMessage {
                request: request_id,
                options: message_options,
                topic,
            }))
            .await?;

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

    /// Subscribes to a topic in the realm.
    ///
    /// The resulting subscription contains an event receiver stream for published events. The
    /// stream automatically closes when the peer unsubscribes from the topic or when the session
    /// ends.
    pub async fn subscribe(&self, topic: Uri) -> Result<Subscription> {
        self.subscribe_internal(topic.into(), SubscriptionOptions::default())
            .await
    }

    /// Subscribes to a topic in the realm with additional options.
    ///
    /// The resulting subscription contains an event receiver stream for published events. The
    /// stream automatically closes when the peer unsubscribes from the topic or when the session
    /// ends.
    pub async fn subscribe_with_options(
        &self,
        topic: WildcardUri,
        options: SubscriptionOptions,
    ) -> Result<Subscription> {
        self.subscribe_internal(topic.into(), options).await
    }

    /// Removes a subscription.
    ///
    /// The subscription ID is received after subscribing to the topic.
    pub async fn unsubscribe(&self, id: Id) -> Result<()> {
        let (_reference, (message_tx, id_allocator, mut unsubscribed_rx)) = self
            .get_from_peer_state(async |peer_state| {
                (
                    peer_state.message_tx.clone(),
                    peer_state.session.id_allocator(),
                    peer_state.session.unsubscribed_rx(),
                )
            })
            .await?;
        let request_id = id_allocator.generate_id().await;

        message_tx
            .send(Message::Unsubscribe(UnsubscribeMessage {
                request: request_id,
                subscribed_subscription: id,
            }))
            .await?;

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
    pub async fn publish(&self, topic: Uri, event: PublishedEvent) -> Result<()> {
        let (_reference, (message_tx, id_allocator, mut published_rx)) = self
            .get_from_peer_state(async |peer_state| {
                (
                    peer_state.message_tx.clone(),
                    peer_state.session.id_allocator(),
                    peer_state.session.published_rx(),
                )
            })
            .await?;
        let request_id = id_allocator.generate_id().await;

        message_tx
            .send(Message::Publish(PublishMessage {
                request: request_id,
                options: event
                    .options
                    .wamp_serialize()?
                    .dictionary()
                    .ok_or_else(|| {
                        Error::msg("expected publish options to serialize as a dictionary")
                    })?
                    .clone(),
                topic,
                arguments: event.arguments,
                arguments_keyword: event.arguments_keyword,
            }))
            .await?;

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
    async fn register_internal(
        &self,
        procedure: WildcardUri,
        options: ProcedureOptions,
    ) -> Result<Procedure> {
        let (_reference, (message_tx, id_allocator, mut registered_rx)) = self
            .get_from_peer_state(async |peer_state| {
                (
                    peer_state.message_tx.clone(),
                    peer_state.session.id_allocator(),
                    peer_state.session.registered_rx(),
                )
            })
            .await?;
        let request_id = id_allocator.generate_id().await;

        let mut message_options = Dictionary::default();
        if let Some(match_style) = options.match_style {
            message_options.insert("match".to_owned(), Value::String(match_style.into()));
        }
        message_options.insert(
            "invoke".to_owned(),
            Value::String(options.invocation_policy.into()),
        );
        if options.disclose_caller {
            message_options.insert("disclose_caller".to_owned(), Value::Bool(true));
        }
        message_tx
            .send(Message::Register(RegisterMessage {
                request: request_id,
                options: message_options,
                procedure: procedure.into(),
            }))
            .await?;

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

    /// Registers a procedure to an endpoint.
    ///
    /// The resulting procedure contains an invocation receiver stream. The stream automatically
    /// closes when the peer deregisters the procedure or when the session ends.
    pub async fn register(&self, procedure: Uri) -> Result<Procedure> {
        self.register_internal(procedure.into(), ProcedureOptions::default())
            .await
    }

    /// Registers a procedure to an endpoint with additional options.
    pub async fn register_with_options(
        &self,
        procedure: WildcardUri,
        options: ProcedureOptions,
    ) -> Result<Procedure> {
        self.register_internal(procedure.into(), options).await
    }

    /// Removes a procedure.
    ///
    /// The registration ID is received after registering the procedure.
    pub async fn unregister(&self, id: Id) -> Result<()> {
        let (_reference, (message_tx, id_allocator, mut unregistered_rx)) = self
            .get_from_peer_state(async |peer_state| {
                (
                    peer_state.message_tx.clone(),
                    peer_state.session.id_allocator(),
                    peer_state.session.unregistered_rx(),
                )
            })
            .await?;
        let request_id = id_allocator.generate_id().await;

        message_tx
            .send(Message::Unregister(UnregisterMessage {
                request: request_id,
                registered_registration: id,
            }))
            .await?;

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
        mut cancel_rx: mpsc::Receiver<CallCancelMode>,
        message_tx: mpsc::Sender<Message>,
        rpc_result_tx: mpsc::Sender<ChannelTransmittableResult<RpcResult>>,
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
                                })).await?;
                            }
                        }
                        Err(err) => {
                            if err.request_id.is_some_and(|id| id == request_id) {
                                rpc_result_tx.send(Err(err)).await?;
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
                    })).await?;
                }
                _ = session_finished_rx.recv() => {
                    rpc_result_tx.send(Err(Into::<Error>::into(PeerNotConnectedError).into())).await?;
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
        let (_reference, (message_tx, id_allocator, session_rpc_result_rx)) = self
            .get_from_peer_state(async |peer_state| {
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
        if let Some(timeout) = rpc_call.timeout {
            options.insert(
                "timeout".to_owned(),
                Value::Integer(timeout.as_millis() as u64),
            );
        }

        message_tx
            .send(Message::Call(CallMessage {
                request: request_id,
                options,
                procedure: procedure.into(),
                arguments: rpc_call.arguments,
                arguments_keyword: rpc_call.arguments_keyword,
            }))
            .await?;

        let session_finished_rx = self.session_finished_rx();
        let (rpc_result_tx, rpc_result_rx) = mpsc::channel(16);
        let (cancel_tx, cancel_rx) = mpsc::channel(16);
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
