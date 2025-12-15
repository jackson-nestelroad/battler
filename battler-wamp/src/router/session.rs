use std::{
    fmt::Debug,
    future::Future,
    pin::Pin,
    sync::Arc,
    time::Duration,
};

use ahash::HashSet;
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
    WampDeserialize,
    WampSerialize,
};
use futures_util::lock::Mutex;
use log::{
    debug,
    error,
    info,
    warn,
};
use tokio::sync::{
    RwLock,
    broadcast::{
        self,
        error::RecvError,
    },
    mpsc,
};

use crate::{
    auth::{
        GenericServerAuthenticator,
        Identity,
    },
    core::{
        cancel::CallCancelMode,
        close::CloseReason,
        error::{
            BasicError,
            ChannelTransmittableResult,
            InteractionError,
        },
        features::{
            PubSubFeatures,
            RpcFeatures,
        },
        hash::HashMap,
        id::{
            Id,
            IdAllocator,
            SequentialIdAllocator,
        },
        invocation_policy::InvocationPolicy,
        match_style::MatchStyle,
        peer_info::ConnectionType,
        publish_options::PublishOptions,
        roles::{
            PeerRoles,
            RouterRoles,
        },
    },
    message::{
        common::{
            abort_message_for_error,
            error_for_request,
            goodbye_and_out,
            goodbye_with_close_reason,
        },
        message::{
            AuthenticateMessage,
            CallMessage,
            CancelMessage,
            HelloMessage,
            InterruptMessage,
            InvocationMessage,
            Message,
            PublishMessage,
            PublishedMessage,
            RegisterMessage,
            RegisteredMessage,
            ResultMessage,
            SubscribeMessage,
            SubscribedMessage,
            UnregisterMessage,
            UnregisteredMessage,
            UnsubscribeMessage,
            UnsubscribedMessage,
            WelcomeMessage,
            YieldMessage,
        },
    },
    router::{
        context::{
            RealmContext,
            RouterContext,
        },
        procedure::{
            ProcedureCallee,
            ProcedureManager,
        },
        realm::RealmSession,
        topic::TopicManager,
    },
};

struct EstablishedSessionState {
    realm: Uri,
    identity: Option<Identity>,
    subscriptions: HashMap<Id, WildcardUri>,
    procedures: HashMap<Id, WildcardUri>,
    active_invocations_by_call: HashMap<Id, RpcInvocation>,
}

impl Debug for EstablishedSessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Debug)]
        #[allow(unused)]
        struct DebugEstablishedSessionState<'a> {
            realm: &'a Uri,
        }

        DebugEstablishedSessionState { realm: &self.realm }.fmt(f)
    }
}

struct ChallengingSessionState {
    hello_message: HelloMessage,
    authenticator: Arc<Box<dyn GenericServerAuthenticator>>,
}

impl Debug for ChallengingSessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Debug)]
        #[allow(unused)]
        struct DebugChallengingSessionState<'a> {
            hello_message: &'a HelloMessage,
        }

        DebugChallengingSessionState {
            hello_message: &self.hello_message,
        }
        .fmt(f)
    }
}

#[derive(Debug, Default)]
enum SessionState {
    #[default]
    Closed,
    Challenging(ChallengingSessionState),
    Established(EstablishedSessionState),
    Closing,
}

impl SessionState {
    fn is_same_state(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Closed, Self::Closed) => true,
            (Self::Established(_), Self::Established(_)) => true,
            (Self::Closing, Self::Closing) => true,
            _ => false,
        }
    }

    fn allowed_state_transition(&self, next: &Self) -> bool {
        match (self, next) {
            (Self::Closed, Self::Challenging(_)) => true,
            (Self::Closed, Self::Established(_)) => true,
            (Self::Challenging(_), Self::Established(_)) => true,
            (Self::Challenging(_), Self::Closed) => true,
            (Self::Established(_), Self::Closing) => true,
            (Self::Established(_), Self::Closed) => true,
            (Self::Closing, Self::Closed) => true,
            _ => false,
        }
    }
}

#[derive(Default)]
struct SharedSessionState {
    roles: PeerRoles,
    identity: Option<Identity>,
}

mod router_session_message {
    use battler_wamp_values::{
        Dictionary,
        List,
    };

    use crate::core::id::Id;

    /// The result of an RPC invocation.
    #[derive(Debug, Clone)]
    pub struct RpcYield {
        pub request_id: Id,
        pub arguments: List,
        pub arguments_keyword: Dictionary,
        pub options: Dictionary,
    }
}

/// Details about the callee of an RPC invocation.
#[derive(Debug, Clone)]
struct RpcInvocationCalleeDetails {
    callee: ProcedureCallee,
    progressive_call_results: bool,
    forward_timeout_to_callee: bool,
}

#[derive(Debug, Default)]
struct RpcInvocationState {
    current_callee: Option<RpcInvocationCalleeDetails>,
    callees_attempted: HashSet<Id>,
    canceled: bool,
}

/// The result of an RPC invocation.
#[derive(Debug, Clone)]
struct RpcInvocation {
    invocation_request_id: Id,
    procedure: Uri,
    arguments: List,
    arguments_keyword: Dictionary,
    progressive_call_results: bool,
    timeout: Duration,
    state: Arc<Mutex<RpcInvocationState>>,
}

/// A message related to procedure calls that must be strongly ordered.
#[derive(Debug, Clone)]
pub(crate) enum ProcedureMessage {
    Call(CallMessage),
    Cancel(CancelMessage),
}

/// A handle to an asynchronously-running router session.
pub struct SessionHandle {
    id: Id,
    shared_state: Arc<RwLock<SharedSessionState>>,
    id_allocator: Arc<Box<dyn IdAllocator>>,
    message_tx: mpsc::Sender<Message>,
    closed_session_rx: broadcast::Receiver<()>,

    rpc_yield_rx: broadcast::Receiver<ChannelTransmittableResult<router_session_message::RpcYield>>,
}

impl SessionHandle {
    /// The session ID, as reported out to the peer.
    pub fn id(&self) -> Id {
        self.id
    }

    /// Returns the last known roles and features.
    ///
    /// Features are communicated when a session is established. If the session is not established,
    /// the roles may be missing or out of date. Since this data is only for advanced features that
    /// does not break correctness of the protocol, this is acceptable.
    pub async fn roles(&self) -> PeerRoles {
        self.shared_state.read().await.roles.clone()
    }

    /// Returns the last known identity.
    pub async fn identity(&self) -> Option<Identity> {
        self.shared_state.read().await.identity.clone()
    }

    /// A reference to the session's ID generator.
    pub fn id_generator(&self) -> Arc<Box<dyn IdAllocator>> {
        self.id_allocator.clone()
    }

    /// Sends a message over the session.
    pub async fn send_message(&self, message: Message) -> Result<()> {
        self.message_tx.send(message).await.map_err(Error::new)
    }

    /// Closes the session.
    pub async fn close(&self, close_reason: CloseReason) -> Result<()> {
        self.message_tx
            .send(goodbye_with_close_reason(close_reason))
            .await
            .map_err(Error::new)
    }

    /// The receiver channel that is populated when the session moves to the CLOSED state.
    pub fn closed_session_rx(&self) -> broadcast::Receiver<()> {
        self.closed_session_rx.resubscribe()
    }

    ///The receiver channel for responses to INVOCATION messages.
    pub fn rpc_yield_rx(
        &self,
    ) -> broadcast::Receiver<ChannelTransmittableResult<router_session_message::RpcYield>> {
        self.rpc_yield_rx.resubscribe()
    }
}

/// The router end of a WAMP session.
///
/// Handles WAMP messages in a state machine and holds all session-scoped state.
pub struct Session {
    id: Id,
    connection_type: ConnectionType,
    message_tx: mpsc::Sender<Message>,
    service_message_tx: mpsc::Sender<Message>,
    state: RwLock<SessionState>,
    shared_state: Arc<RwLock<SharedSessionState>>,
    id_allocator: Arc<Box<dyn IdAllocator>>,

    closed_session_tx: broadcast::Sender<()>,

    rpc_yield_tx: broadcast::Sender<ChannelTransmittableResult<router_session_message::RpcYield>>,
    rpc_yield_cancel_tx: broadcast::Sender<Id>,
    rpc_yield_cancel_rx: broadcast::Receiver<Id>,

    publish_tx: broadcast::Sender<PublishMessage>,
    procedure_message_tx: broadcast::Sender<ProcedureMessage>,
}

impl Session {
    /// Creates a new session over a service.
    pub fn new(
        id: Id,
        connection_type: ConnectionType,
        message_tx: mpsc::Sender<Message>,
        service_message_tx: mpsc::Sender<Message>,
    ) -> Self {
        let id_allocator = SequentialIdAllocator::default();
        let (closed_session_tx, _) = broadcast::channel(16);
        let (rpc_yield_tx, _) = broadcast::channel(16);
        let (rpc_yield_cancel_tx, rpc_yield_cancel_rx) = broadcast::channel(16);
        let (publish_tx, _) = broadcast::channel(16);
        let (procedure_message_tx, _) = broadcast::channel(16);
        Self {
            id,
            connection_type,
            shared_state: Arc::new(RwLock::new(SharedSessionState::default())),
            message_tx,
            service_message_tx,
            state: RwLock::new(SessionState::default()),
            id_allocator: Arc::new(Box::new(id_allocator)),
            closed_session_tx,
            rpc_yield_tx,
            rpc_yield_cancel_tx,
            rpc_yield_cancel_rx,
            publish_tx,
            procedure_message_tx,
        }
    }

    /// The session ID.
    pub fn id(&self) -> Id {
        self.id
    }

    /// Checks if the session is closed.
    pub async fn closed(&self) -> bool {
        match *self.state.read().await {
            SessionState::Closed => true,
            _ => false,
        }
    }

    /// Generates a handle to the session, which can be saved separately from the session's
    /// lifecycle.
    pub fn session_handle(&self) -> SessionHandle {
        SessionHandle {
            id: self.id,
            shared_state: self.shared_state.clone(),
            id_allocator: self.id_allocator.clone(),
            message_tx: self.message_tx.clone(),
            closed_session_rx: self.closed_session_tx.subscribe(),
            rpc_yield_rx: self.rpc_yield_tx.subscribe(),
        }
    }

    /// The receiver channel for publications, for strong ordering.
    pub fn publish_rx(&self) -> broadcast::Receiver<PublishMessage> {
        self.publish_tx.subscribe()
    }

    /// The receiver channel for procedure call messages, for strong ordering.
    pub fn procedure_message_rx(&self) -> broadcast::Receiver<ProcedureMessage> {
        self.procedure_message_tx.subscribe()
    }

    async fn get_from_challenging_session_state<F, T>(&self, f: F) -> Result<T, Error>
    where
        F: Fn(&ChallengingSessionState) -> T,
    {
        match &*self.state.read().await {
            SessionState::Challenging(state) => Ok(f(&state)),
            _ => Err(Error::msg("session is not in the challenging state")),
        }
    }

    async fn get_from_established_session_state<F, T>(&self, f: F) -> Result<T, Error>
    where
        F: Fn(&EstablishedSessionState) -> T,
    {
        match &*self.state.read().await {
            SessionState::Established(state) => Ok(f(&state)),
            _ => Err(Error::msg("session is not in the established state")),
        }
    }

    async fn modify_established_session_state<F, T>(&self, f: F) -> Result<T, Error>
    where
        F: FnOnce(&mut EstablishedSessionState) -> T,
    {
        match &mut *self.state.write().await {
            SessionState::Established(state) => Ok(f(state)),
            _ => Err(Error::msg("session is not in the established state")),
        }
    }

    pub async fn send_message(&self, message: Message) -> Result<()> {
        self.transition_state_from_sending_message(&message).await?;
        self.service_message_tx
            .send(message)
            .await
            .map_err(Error::new)
    }

    async fn transition_state_from_sending_message(&self, message: &Message) -> Result<()> {
        let next_state = match message {
            Message::Abort(_) => SessionState::Closed,
            Message::Goodbye(_) => match *self.state.read().await {
                SessionState::Closing => SessionState::Closed,
                _ => SessionState::Closing,
            },
            _ => return Ok(()),
        };
        self.transition_state(next_state).await
    }

    /// Handles a message over the session state machine.
    pub async fn handle_message<S>(
        &self,
        context: RouterContext<S>,
        message: Message,
    ) -> Result<()> {
        debug!("Received message for session {}: {message:?}", self.id);
        if let Err(err) = self
            .handle_message_on_state_machine(&context, message)
            .await
        {
            self.send_message(abort_message_for_error(&err)).await?;
            return Err(err);
        }
        Ok(())
    }

    async fn handle_message_on_state_machine<S>(
        &self,
        context: &RouterContext<S>,
        message: Message,
    ) -> Result<()> {
        // Read state separately from handling the message, so that we don't lock the session state.
        let mut closing = false;
        let mut closed = false;
        let mut challenging = false;
        match *self.state.read().await {
            SessionState::Closed => closed = true,
            SessionState::Closing => closing = true,
            SessionState::Challenging(_) => challenging = true,
            _ => (),
        }

        if closed {
            self.handle_closed(context, message).await
        } else if closing {
            self.handle_closing(context, message).await
        } else if challenging {
            self.handle_challenging(context, message).await
        } else {
            self.handle_established(context, message).await
        }
    }

    fn read_peer_roles(message: &HelloMessage) -> PeerRoles {
        let roles = match message.details.get("roles") {
            Some(roles) => roles.clone(),
            None => return PeerRoles::default(),
        };
        PeerRoles::wamp_deserialize(roles).unwrap_or_default()
    }

    async fn handle_closed<S>(&self, context: &RouterContext<S>, message: Message) -> Result<()> {
        match message {
            Message::Hello(message) => {
                if let Err(err) = self.handle_hello(context, &message).await {
                    return self.send_message(abort_message_for_error(&err)).await;
                }
                Ok(())
            }
            _ => Err(InteractionError::ProtocolViolation(format!(
                "received {} message on a closed session",
                message.message_name()
            ))
            .into()),
        }
    }

    async fn handle_hello<S>(
        &self,
        context: &RouterContext<S>,
        message: &HelloMessage,
    ) -> Result<()> {
        let realm_context = context.realm_context(&message.realm)?;
        let challenged = self
            .issue_authentication_challenge(&realm_context, &message)
            .await?;
        if challenged {
            return Ok(());
        }
        if realm_context.realm().config.authentication.required {
            return Err(InteractionError::AuthenticationRequired.into());
        }
        self.welcome_to_realm(context, &message, None, Box::new(|_| Ok(())))
            .await
    }

    async fn find_supported_authenticator<S>(
        &self,
        context: &RealmContext<'_, S>,
        message: &HelloMessage,
    ) -> Result<Option<Box<dyn GenericServerAuthenticator>>> {
        let auth_methods = message
            .details
            .get("authmethods")
            .and_then(|val| val.list());
        let auth_methods = match auth_methods {
            Some(auth_methods) => auth_methods,
            None => return Ok(None),
        };
        let mut authenticator = None;
        for auth_method in &context.realm().config.authentication.methods {
            if !auth_methods.contains(&Value::String(auth_method.auth_method().to_string())) {
                continue;
            }
            authenticator = Some(auth_method.new_authenticator().await?);
        }

        Ok(authenticator)
    }

    async fn issue_authentication_challenge<S>(
        &self,
        context: &RealmContext<'_, S>,
        message: &HelloMessage,
    ) -> Result<bool> {
        let authenticator = match self.find_supported_authenticator(context, message).await? {
            Some(authenticator) => authenticator,
            None => return Ok(false),
        };

        let challenge = authenticator.challenge(message).await?;

        self.transition_state(SessionState::Challenging(ChallengingSessionState {
            hello_message: message.clone(),
            authenticator: Arc::new(authenticator),
        }))
        .await?;

        self.send_message(Message::Challenge(challenge)).await?;
        Ok(true)
    }

    async fn welcome_to_realm<S>(
        &self,
        context: &RouterContext<S>,
        message: &HelloMessage,
        identity: Option<Identity>,
        modify_welcome_message: Box<dyn FnOnce(&mut WelcomeMessage) -> Result<()> + Send>,
    ) -> Result<()> {
        let context = context.realm_context(&message.realm)?;
        context.realm().sessions.write().await.insert(
            self.id,
            Arc::new(RealmSession {
                session: self.session_handle(),
            }),
        );
        info!("Session {} joined realm {}", self.id, context.realm().uri());

        let mut details = Dictionary::default();
        details.insert(
            "agent".to_owned(),
            Value::String(context.router().config.agent.clone()),
        );

        let pub_sub_features = PubSubFeatures {};
        let rpc_features = RpcFeatures {
            call_canceling: true,
            progressive_call_results: true,
            call_timeout: true,
            shared_registration: true,
        };
        details.insert(
            "roles".to_owned(),
            RouterRoles::new(
                context.router().config.roles.iter().cloned(),
                pub_sub_features,
                rpc_features,
            )
            .wamp_serialize()?,
        );

        self.shared_state.write().await.roles = Self::read_peer_roles(&message);
        self.transition_state(SessionState::Established(EstablishedSessionState {
            realm: context.realm().uri().clone(),
            identity,
            subscriptions: HashMap::default(),
            procedures: HashMap::default(),
            active_invocations_by_call: HashMap::default(),
        }))
        .await?;

        let mut message = WelcomeMessage {
            session: self.id,
            details,
        };

        modify_welcome_message(&mut message)?;

        self.send_message(Message::Welcome(message)).await
    }

    async fn handle_challenging<S>(
        &self,
        context: &RouterContext<S>,
        message: Message,
    ) -> Result<()> {
        match message {
            Message::Abort(_) => {
                warn!("Router session {} aborted by peer: {message:?}", self.id);
                self.transition_state(SessionState::Closed).await
            }
            Message::Goodbye(_) => {
                self.transition_state(SessionState::Closing).await?;
                self.send_message(goodbye_and_out()).await
            }
            Message::Authenticate(ref authenticate_message) => {
                if let Err(err) = self
                    .handle_authenticate(context, authenticate_message)
                    .await
                {
                    return self.send_message(abort_message_for_error(&err)).await;
                }
                Ok(())
            }
            _ => Err(InteractionError::ProtocolViolation(format!(
                "received {} message on a challenging session",
                message.message_name()
            ))
            .into()),
        }
    }

    async fn handle_authenticate<S>(
        &self,
        context: &RouterContext<S>,
        message: &AuthenticateMessage,
    ) -> Result<()> {
        let (hello_message, authenticator) = self
            .get_from_challenging_session_state(|state| {
                (state.hello_message.clone(), state.authenticator.clone())
            })
            .await?;
        let result = authenticator.authenticate(message).await?;

        self.shared_state.write().await.identity = Some(result.identity.clone());

        self.welcome_to_realm(
            context,
            &hello_message,
            Some(result.identity.clone()),
            Box::new(|message| result.embed_into_welcome_message(message)),
        )
        .await
    }

    async fn handle_established<S>(
        &self,
        context: &RouterContext<S>,
        message: Message,
    ) -> Result<()> {
        match message {
            Message::Abort(_) => {
                warn!("Router session {} aborted by peer: {message:?}", self.id);
                self.transition_state(SessionState::Closed).await
            }
            Message::Goodbye(_) => {
                self.transition_state(SessionState::Closing).await?;
                self.send_message(goodbye_and_out()).await
            }
            ref message @ Message::Subscribe(ref subscribe_message) => {
                if let Err(err) = self.handle_subscribe(context, subscribe_message).await {
                    return self.send_message(error_for_request(&message, &err)).await;
                }
                Ok(())
            }
            ref message @ Message::Unsubscribe(ref unsubscribe_message) => {
                if let Err(err) = self.handle_unsubscribe(context, unsubscribe_message).await {
                    return self.send_message(error_for_request(&message, &err)).await;
                }
                Ok(())
            }
            ref message @ Message::Publish(ref publish_message) => {
                if let Err(err) = self.handle_publish(context, publish_message).await {
                    return self.send_message(error_for_request(&message, &err)).await;
                }
                Ok(())
            }
            ref message @ Message::Register(ref register_message) => {
                if let Err(err) = self.handle_register(context, register_message).await {
                    return self.send_message(error_for_request(&message, &err)).await;
                }
                Ok(())
            }
            ref message @ Message::Unregister(ref unregister_message) => {
                if let Err(err) = self.handle_unregister(context, unregister_message).await {
                    return self.send_message(error_for_request(&message, &err)).await;
                }
                Ok(())
            }
            ref message @ Message::Call(ref call_message) => {
                if let Err(err) = self.handle_call(context, call_message).await {
                    return self.send_message(error_for_request(&message, &err)).await;
                }
                Ok(())
            }
            ref message @ Message::Yield(ref yield_message) => {
                if let Err(err) = self.handle_yield(context, yield_message).await {
                    return self.send_message(error_for_request(&message, &err)).await;
                }
                Ok(())
            }
            ref message @ Message::Cancel(ref cancel_message) => {
                if let Err(err) = self.handle_cancel(context, cancel_message).await {
                    return self.send_message(error_for_request(&message, &err)).await;
                }
                Ok(())
            }
            ref message @ Message::Error(ref error_message) => {
                match error_message.request_type {
                    Message::INVOCATION_TAG => {
                        self.rpc_yield_tx.send(Err(message.try_into()?))?;
                    }
                    _ => {
                        error!(
                            "Invalid ERROR message with request type {} received from the router: {error_message:?}",
                            error_message.request_type
                        );
                        return Err(
                            BasicError::InvalidArgument("invalid request type".to_owned()).into(),
                        );
                    }
                }
                Ok(())
            }
            _ => Err(InteractionError::ProtocolViolation(format!(
                "received {} message on an established session",
                message.message_name()
            ))
            .into()),
        }
    }

    async fn handle_subscribe<S>(
        &self,
        context: &RouterContext<S>,
        message: &SubscribeMessage,
    ) -> Result<()> {
        let realm = self
            .get_from_established_session_state(|state| state.realm.clone())
            .await?;
        let context = context.realm_context(&realm)?;
        let match_style = message
            .options
            .get("match")
            .and_then(|val| val.string())
            .and_then(|val| MatchStyle::try_from(val).ok());
        let subscription =
            TopicManager::subscribe(&context, self.id, message.topic.clone(), match_style).await?;
        self.modify_established_session_state(|state| {
            state
                .subscriptions
                .insert(subscription, message.topic.clone())
        })
        .await?;
        self.send_message(Message::Subscribed(SubscribedMessage {
            subscribe_request: message.request,
            subscription,
        }))
        .await?;
        // Activate the subscription only after sending the response, so that the peer does not
        // receive events prior to the confirmation.
        TopicManager::activate_subscription(&context, self.id, &message.topic).await;
        Ok(())
    }

    async fn handle_unsubscribe<S>(
        &self,
        context: &RouterContext<S>,
        message: &UnsubscribeMessage,
    ) -> Result<()> {
        let topic = self
            .modify_established_session_state(|state| {
                state
                    .subscriptions
                    .remove(&message.subscribed_subscription)
                    .ok_or_else(|| InteractionError::NoSuchSubscription)
            })
            .await??;
        let realm = self
            .get_from_established_session_state(|state| state.realm.clone())
            .await?;
        let mut context = context.realm_context(&realm)?;
        TopicManager::unsubscribe(&mut context, self.id, &topic).await;
        self.send_message(Message::Unsubscribed(UnsubscribedMessage {
            unsubscribe_request: message.request,
        }))
        .await
    }

    async fn handle_publish<S>(
        &self,
        _: &RouterContext<S>,
        message: &PublishMessage,
    ) -> Result<()> {
        self.publish_tx
            .send(message.clone())
            .map(|_| ())
            .map_err(Error::new)
    }

    /// Handles an ordered publication from the peer.
    ///
    /// Returns when the publication has been sent to all subscribers.
    pub async fn handle_ordered_publish<S>(
        &self,
        context: &RouterContext<S>,
        message: PublishMessage,
    ) -> Result<()> {
        if let Err(err) = self
            .handle_ordered_publish_internal(context, &message)
            .await
        {
            self.send_message(error_for_request(&Message::Publish(message), &err))
                .await?;
        }
        Ok(())
    }

    async fn handle_ordered_publish_internal<S>(
        &self,
        context: &RouterContext<S>,
        message: &PublishMessage,
    ) -> Result<()> {
        let options = PublishOptions::try_from(message)?;
        let realm = self
            .get_from_established_session_state(|state| state.realm.clone())
            .await?;
        let mut context = context.realm_context(&realm)?;
        let publication = TopicManager::publish(
            &mut context,
            self.id,
            &message.topic,
            message.arguments.clone(),
            message.arguments_keyword.clone(),
            options,
        )
        .await?;
        self.send_message(Message::Published(PublishedMessage {
            publish_request: message.request,
            publication,
        }))
        .await
    }

    async fn handle_register<S>(
        &self,
        context: &RouterContext<S>,
        message: &RegisterMessage,
    ) -> Result<()> {
        let realm = self
            .get_from_established_session_state(|state| state.realm.clone())
            .await?;
        let mut context = context.realm_context(&realm)?;

        let match_style = message
            .options
            .get("match")
            .and_then(|val| val.string())
            .and_then(|val| MatchStyle::try_from(val).ok());
        let invocation_policy = message
            .options
            .get("invoke")
            .and_then(|val| val.string())
            .and_then(|val| InvocationPolicy::try_from(val).ok())
            .unwrap_or_default();
        let registration = ProcedureManager::register(
            &mut context,
            self.id,
            message.procedure.clone(),
            match_style,
            invocation_policy,
        )
        .await?;
        self.modify_established_session_state(|state| {
            state
                .procedures
                .insert(registration, message.procedure.clone())
        })
        .await?;
        self.send_message(Message::Registered(RegisteredMessage {
            register_request: message.request,
            registration,
        }))
        .await?;
        // Activate the procedure only after sending the response, so that the peer does not
        // receive invocations prior to the confirmation.
        ProcedureManager::activate_procedure(&mut context, &message.procedure).await;
        Ok(())
    }

    async fn handle_unregister<S>(
        &self,
        context: &RouterContext<S>,
        message: &UnregisterMessage,
    ) -> Result<()> {
        let procedure = self
            .modify_established_session_state(|state| {
                state
                    .procedures
                    .remove(&message.registered_registration)
                    .ok_or_else(|| InteractionError::NoSuchRegistration)
            })
            .await??;
        let realm = self
            .get_from_established_session_state(|state| state.realm.clone())
            .await?;
        let mut context = context.realm_context(&realm)?;
        ProcedureManager::unregister(&mut context, &procedure).await;
        self.send_message(Message::Unregistered(UnregisteredMessage {
            unregister_request: message.request,
        }))
        .await
    }

    async fn handle_call<S>(&self, _: &RouterContext<S>, message: &CallMessage) -> Result<()> {
        let progressive_call_results = message
            .options
            .get("receive_progress")
            .and_then(|val| val.bool())
            .unwrap_or(false);

        let timeout = message
            .options
            .get("timeout")
            .and_then(|val| val.integer())
            .unwrap_or(0);
        let timeout = Duration::from_millis(timeout);

        let request_id = self.id_allocator.generate_id().await;

        let invocation = RpcInvocation {
            invocation_request_id: request_id,
            procedure: message.procedure.clone(),
            arguments: message.arguments.clone(),
            arguments_keyword: message.arguments_keyword.clone(),
            progressive_call_results,
            timeout,
            state: Arc::new(Mutex::new(RpcInvocationState::default())),
        };

        // Store the active invocation immediately before handling another message.
        self.modify_established_session_state(|state| {
            state
                .active_invocations_by_call
                .insert(message.request, invocation)
        })
        .await?;

        self.procedure_message_tx
            .send(ProcedureMessage::Call(message.clone()))
            .map(|_| ())
            .map_err(Error::new)
    }

    /// Handles an ordered procedure call from the peer.
    ///
    /// Returns when the invocation has been sent to the callee. Returns the original call request
    /// ID for [`Self::handle_invocation`].
    pub async fn handle_ordered_call<S>(
        &self,
        context: &RouterContext<S>,
        message: CallMessage,
    ) -> Result<Option<Id>> {
        let call_request_id = match self.handle_ordered_call_internal(context, &message).await {
            Ok(call_request_id) => call_request_id,
            Err(err) => {
                self.send_message(error_for_request(&Message::Call(message), &err))
                    .await?;
                return Ok(None);
            }
        };
        Ok(Some(call_request_id))
    }

    async fn handle_ordered_call_internal<S>(
        &self,
        context: &RouterContext<S>,
        message: &CallMessage,
    ) -> Result<Id> {
        if self.shared_state.read().await.roles.caller.is_none() {
            return Err(BasicError::NotAllowed("peer is not a caller".to_owned()).into());
        }
        self.initiate_invocation(context, message.request).await?;
        Ok(message.request)
    }

    async fn select_callee_for_invocation<S>(
        &self,
        context: &RealmContext<'_, S>,
        invocation: &RpcInvocation,
    ) -> Result<ProcedureCallee> {
        let procedure = context
            .procedure(&invocation.procedure.clone().into())
            .await
            .ok_or_else(|| InteractionError::NoSuchProcedure)?;
        procedure
            .get_callee(&invocation.state.lock().await.callees_attempted)
            .await
    }

    async fn initiate_invocation<S>(
        &self,
        context: &RouterContext<S>,
        request_id: Id,
    ) -> Result<()> {
        let (realm, invocation) = self
            .get_from_established_session_state(|state| {
                (
                    state.realm.clone(),
                    state.active_invocations_by_call.get(&request_id).cloned(),
                )
            })
            .await?;
        let context = context.realm_context(&realm)?;
        let invocation = match invocation {
            Some(invocation) => invocation,
            // Invocation was lost, likely due to immediate cancellation.
            None => return Err(InteractionError::Canceled.into()),
        };

        // Do not re-initiate the invocation if it was canceled.
        if invocation.state.lock().await.canceled {
            return Err(InteractionError::Canceled.into());
        }

        // Select a callee and send the invocation to them.
        //
        // If sending to the callee fails, try again with another callee.
        loop {
            let callee = self
                .select_callee_for_invocation(&context, &invocation)
                .await?;
            if let Ok(id) = self
                .send_invocation_to_callee(&context, &invocation, callee)
                .await
            {
                return Ok(id);
            }

            // We will eventually run out of callees, and selection will fail.
        }
    }

    async fn save_current_callee<S>(
        &self,
        context: &RealmContext<'_, S>,
        invocation: &RpcInvocation,
        callee: ProcedureCallee,
    ) -> Result<RpcInvocationCalleeDetails> {
        invocation
            .state
            .lock()
            .await
            .callees_attempted
            .insert(callee.session);

        let session = context.session(callee.session).await.ok_or_else(|| {
            BasicError::NotFound(format!("callee session {} not found", callee.session))
        })?;
        let progressive_call_results = invocation.progressive_call_results
            && session
                .session
                .roles()
                .await
                .callee
                .is_some_and(|features| features.progressive_call_results);
        let forward_timeout_to_callee = session
            .session
            .roles()
            .await
            .callee
            .is_some_and(|features| features.call_timeout);

        let callee_details = RpcInvocationCalleeDetails {
            callee,
            progressive_call_results,
            forward_timeout_to_callee,
        };
        invocation.state.lock().await.current_callee = Some(callee_details.clone());
        Ok(callee_details)
    }

    async fn send_invocation_to_callee<S>(
        &self,
        context: &RealmContext<'_, S>,
        invocation: &RpcInvocation,
        callee: ProcedureCallee,
    ) -> Result<()> {
        let callee_details = self
            .save_current_callee(context, invocation, callee)
            .await?;

        let mut details = Dictionary::default();
        details.insert(
            "procedure".to_owned(),
            Value::String(invocation.procedure.to_string()),
        );
        if callee_details.progressive_call_results {
            details.insert("receive_progress".to_owned(), Value::Bool(true));
        }
        if !invocation.timeout.is_zero() && callee_details.forward_timeout_to_callee {
            details.insert(
                "timeout".to_owned(),
                Value::Integer(invocation.timeout.as_millis() as u64),
            );
        }

        match &self.connection_type {
            ConnectionType::Remote(addr) => {
                details.insert(
                    "battler_wamp_remote_addr".to_owned(),
                    Value::String(addr.clone()),
                );
            }
            ConnectionType::Direct => {
                details.insert("battler_wamp_direct_peer".to_owned(), Value::Bool(true));
            }
        }

        let identity = self
            .get_from_established_session_state(|state| state.identity.clone())
            .await?;
        if let Some(identity) = identity {
            details.insert("battler_wamp_authid".to_owned(), Value::String(identity.id));
            details.insert(
                "battler_wamp_authrole".to_owned(),
                Value::String(identity.role),
            );
        }

        let session = context
            .session(callee_details.callee.session)
            .await
            .ok_or_else(|| {
                BasicError::NotFound(format!(
                    "callee session {} not found",
                    callee_details.callee.session
                ))
            })?;
        session
            .session
            .send_message(Message::Invocation(InvocationMessage {
                request: invocation.invocation_request_id,
                registered_registration: callee_details.callee.registration,
                details,
                call_arguments: invocation.arguments.clone(),
                call_arguments_keyword: invocation.arguments_keyword.clone(),
            }))
            .await?;
        Ok(())
    }

    /// Handles the invocation mapped to the call request ID returned from
    /// [`Self::handle_ordered_call`].
    ///
    /// Returns when the result has been sent to the peer.
    pub async fn handle_invocation<S>(
        &self,
        context: &RouterContext<S>,
        call_request_id: Id,
    ) -> Result<()> {
        loop {
            if let Err(mut err) = self
                .handle_invocation_internal(context, call_request_id)
                .await
            {
                if let Some(&InteractionError::Unavailable) = err.downcast_ref::<InteractionError>()
                {
                    // Callee was unavailable, so retry.
                    match self.initiate_invocation(context, call_request_id).await {
                        Ok(()) => continue,
                        Err(invoke_err) => err = invoke_err,
                    }
                }

                self.send_message(error_for_request(
                    &Message::Call(CallMessage {
                        request: call_request_id,
                        ..Default::default()
                    }),
                    &err,
                ))
                .await?;
            }

            break;
        }

        // Forget the invocation only when everything is done.
        self.modify_established_session_state(|state| {
            state.active_invocations_by_call.remove(&call_request_id)
        })
        .await?;

        Ok(())
    }

    async fn handle_invocation_internal<S>(
        &self,
        context: &RouterContext<S>,
        call_request_id: Id,
    ) -> Result<()> {
        let (realm, invocation) = self
            .get_from_established_session_state(|state| {
                (
                    state.realm.clone(),
                    state
                        .active_invocations_by_call
                        .get(&call_request_id)
                        .cloned(),
                )
            })
            .await?;
        let invocation = invocation.ok_or_else(|| InteractionError::Canceled)?;
        let context = context.realm_context(&realm)?;

        let callee_details = invocation
            .state
            .lock()
            .await
            .current_callee
            .clone()
            .ok_or_else(|| {
                BasicError::Internal("expected invocation to have an assigned callee".to_owned())
            })?;
        let callee = context
            .session(callee_details.callee.session)
            .await
            .ok_or_else(|| Error::new(InteractionError::Canceled))?;
        let mut rpc_yield_rx = callee.session.rpc_yield_rx.resubscribe();
        let mut cancel_rx = self.rpc_yield_cancel_rx.resubscribe();
        let mut closed_session_rx = self.closed_session_tx.subscribe();

        // Check for cancellation before doing anything, since the call may already be canceled
        // after the resubscribe call on the rpc_yield_cancel_rx channel above.
        //
        // Note that we could still let the procedure be invoked and just wait for the first result
        // when the caller specifies the "kill" option. We do this as a sort of optimization, since
        // we know the caller's intent, and to make the router cancellation flow a bit simpler.
        if invocation.state.lock().await.canceled {
            return Err(InteractionError::Canceled.into());
        }

        loop {
            let rpc_yield = Self::wait_for_rpc_yield(
                &callee,
                &mut rpc_yield_rx,
                &mut cancel_rx,
                &mut closed_session_rx,
                invocation.invocation_request_id,
                callee
                    .session
                    .roles()
                    .await
                    .callee
                    .map(|feature| feature.call_canceling)
                    .unwrap_or(false),
                callee_details.progressive_call_results,
                if callee_details.forward_timeout_to_callee {
                    Duration::ZERO
                } else {
                    invocation.timeout
                },
            )
            .await?;

            let progress = callee_details.progressive_call_results
                && rpc_yield
                    .options
                    .get("progress")
                    .is_some_and(|val| val.bool().unwrap_or(false));
            let mut details = Dictionary::default();
            if progress {
                details.insert("progress".to_owned(), Value::Bool(true));
            }

            self.send_message(Message::Result(ResultMessage {
                call_request: call_request_id,
                details,
                yield_arguments: rpc_yield.arguments,
                yield_arguments_keyword: rpc_yield.arguments_keyword,
            }))
            .await?;
            let canceled = invocation.state.lock().await.canceled;

            if !progress || canceled {
                break;
            }
        }

        Ok(())
    }

    async fn wait_for_rpc_yield(
        callee: &RealmSession,
        rpc_yield_rx: &mut broadcast::Receiver<
            ChannelTransmittableResult<router_session_message::RpcYield>,
        >,
        cancel_rx: &mut broadcast::Receiver<Id>,
        closed_session_rx: &mut broadcast::Receiver<()>,
        request_id: Id,
        call_canceling: bool,
        progressive_call_results: bool,
        timeout: Duration,
    ) -> Result<router_session_message::RpcYield> {
        loop {
            let timeout: Pin<Box<dyn Future<Output = ()> + Send>> = if timeout.is_zero() {
                Box::pin(futures_util::future::pending())
            } else {
                Box::pin(tokio::time::sleep(timeout))
            };
            tokio::select! {
                rpc_yield = rpc_yield_rx.recv() => {
                    match rpc_yield.map_err(|err| match err {
                        RecvError::Closed => Error::new(InteractionError::Canceled),
                        _ => err.into()
                    })? {
                        Ok(rpc_yield) => {
                            if rpc_yield.request_id == request_id {
                                return Ok(rpc_yield);
                            }
                        }
                        Err(err) => {
                            if err.request_id.is_some_and(|id| id == request_id) {
                                return Err(err.into());
                            }
                        }
                    }
                }
                id = cancel_rx.recv() => {
                    if id.is_ok_and(|id| id == request_id) {
                        return Err(InteractionError::Canceled.into());
                    }
                }
                _ = closed_session_rx.recv() => {
                    // Caller left, so the invocation should be interrupted (if supported).
                    //
                    // Otherwise, the single message will just get disposed once we get it.
                    if progressive_call_results {
                        callee.session.send_message(
                            Message::Interrupt(InterruptMessage {
                                invocation_request: request_id,
                                ..Default::default()
                            })
                        ).await?;
                        return Err(InteractionError::Canceled.into());
                    }
                }
                _ = timeout => {
                    // Dealer-initiated timeout: interrupt the callee (if supported) and error out immediately.
                    if call_canceling {
                        callee.session.send_message(
                            Message::Interrupt(InterruptMessage {
                                invocation_request: request_id,
                                ..Default::default()
                            })
                        ).await?;
                    }
                    return Err(InteractionError::Canceled.into());
                }
            }
        }
    }

    async fn handle_yield<S>(&self, _: &RouterContext<S>, message: &YieldMessage) -> Result<()> {
        self.rpc_yield_tx
            .send(Ok(router_session_message::RpcYield {
                request_id: message.invocation_request,
                arguments: message.arguments.clone(),
                arguments_keyword: message.arguments_keyword.clone(),
                options: message.options.clone(),
            }))?;
        Ok(())
    }

    async fn handle_cancel<S>(&self, _: &RouterContext<S>, message: &CancelMessage) -> Result<()> {
        self.procedure_message_tx
            .send(ProcedureMessage::Cancel(message.clone()))
            .map(|_| ())
            .map_err(Error::new)
    }

    /// Handles an ordered procedure call cancel from the peer.
    pub async fn handle_ordered_cancel<S>(
        &self,
        context: &RouterContext<S>,
        message: CancelMessage,
    ) -> Result<()> {
        let mut mode = match message.options.get("mode").and_then(|mode| mode.string()) {
            Some(mode) => CallCancelMode::try_from(mode).unwrap_or_default(),
            None => CallCancelMode::default(),
        };

        let (realm, invocation) = self
            .get_from_established_session_state(|state| {
                (
                    state.realm.clone(),
                    state
                        .active_invocations_by_call
                        .get(&message.call_request)
                        .cloned(),
                )
            })
            .await?;

        // If there is no invocation for the call being canceled, there is nothing to do.
        let invocation = match invocation {
            Some(invocation) => invocation,
            None => return Ok(()),
        };

        let context = context.realm_context(&realm)?;

        // If there is no callee, the call should already be canceled.
        let callee = match &invocation.state.lock().await.current_callee {
            Some(callee_details) => callee_details.callee.session,
            None => return Ok(()),
        };
        let callee = match context.session(callee).await {
            Some(callee) => callee,
            None => return Ok(()),
        };

        // Avoid sending an INTERRUPT message if the callee does not support it.
        if !callee
            .session
            .roles()
            .await
            .callee
            .is_some_and(|features| features.call_canceling)
        {
            mode = CallCancelMode::Skip;
        }

        let send_interrupt = mode != CallCancelMode::Skip;
        let immediate_error = mode != CallCancelMode::Kill;

        if send_interrupt {
            callee
                .session
                .send_message(Message::Interrupt(InterruptMessage {
                    invocation_request: invocation.invocation_request_id,
                    ..Default::default()
                }))
                .await?;
        }

        if immediate_error {
            // Notify the task that is waiting for YIELD messages to stop.
            self.rpc_yield_cancel_tx
                .send(invocation.invocation_request_id)?;
        }

        // Mark the invocation as canceled, so the task waiting for YIELD messages knows to stop.
        invocation.state.lock().await.canceled = true;

        Ok(())
    }

    async fn handle_closing<S>(&self, _: &RouterContext<S>, message: Message) -> Result<()> {
        match message {
            Message::Goodbye(_) => self.transition_state(SessionState::Closed).await,
            _ => Ok(()),
        }
    }

    async fn validate_state_transition(&self, state: &SessionState) -> Result<bool> {
        let current_state = self.state.read().await;
        if current_state.is_same_state(state) {
            return Ok(true);
        }

        if !current_state.allowed_state_transition(&state) {
            return Err(BasicError::Internal(format!(
                "invalid state transition from {:?} to {state:?}",
                self.state
            ))
            .into());
        }

        Ok(false)
    }

    async fn transition_state(&self, state: SessionState) -> Result<()> {
        if self.validate_state_transition(&state).await? {
            return Ok(());
        }

        debug!(
            "Router session {} transitioned from {:?} to {state:?}",
            self.id,
            self.state.read().await
        );
        *self.state.write().await = state;

        match &*self.state.read().await {
            SessionState::Established(_) => {
                self.id_allocator.reset().await;
            }
            SessionState::Closed => {
                self.closed_session_tx.send(()).ok();
            }
            _ => (),
        }

        Ok(())
    }

    pub async fn clean_up<S>(&self, context: &RouterContext<S>) {
        let id = self.id;

        // We only need to clean up if we have resources in a realm.
        let realm = match self
            .get_from_established_session_state(|state| state.realm.clone())
            .await
        {
            Ok(realm) => realm,
            Err(_) => return,
        };

        let mut context = match context.realm_context(&realm) {
            Ok(context) => context,
            Err(err) => {
                error!(
                    "Failed to clean up session {id}, due to error getting context for realm {realm}: {err:?}"
                );
                return;
            }
        };

        match &mut *self.state.write().await {
            SessionState::Established(state) => {
                for topic in state.subscriptions.values() {
                    TopicManager::unsubscribe(&mut context, id, &topic).await;
                }
                state.subscriptions.clear();

                for procedure in state.procedures.values() {
                    ProcedureManager::unregister(&mut context, &procedure).await;
                }
                state.procedures.clear();

                context.realm().sessions.write().await.remove(&id);
            }
            _ => (),
        }
    }
}
