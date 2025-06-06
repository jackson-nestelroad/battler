use std::{
    fmt::Debug,
    sync::Arc,
    time::Duration,
};

use anyhow::{
    Error,
    Result,
};
use battler_wamp_values::{
    Dictionary,
    List,
    Value,
};
use log::{
    debug,
    error,
    info,
    warn,
};
use thiserror::Error;
use tokio::sync::{
    RwLock,
    broadcast,
    mpsc,
};

use crate::{
    auth::Identity,
    core::{
        error::{
            BasicError,
            ChannelTransmittableResult,
            InteractionError,
            WampError,
        },
        hash::HashMap,
        id::{
            Id,
            IdAllocator,
            SequentialIdAllocator,
        },
        publish_options::PublishOptions,
        uri::Uri,
    },
    message::{
        common::{
            abort_message_for_error,
            error_for_request,
            goodbye_and_out,
        },
        message::{
            ChallengeMessage,
            InvocationMessage,
            Message,
            WelcomeMessage,
            YieldMessage,
        },
    },
};

#[derive(Debug)]
struct EstablishingSessionState {
    realm: Uri,
    authenticating: bool,
}

struct EstablishedSessionState {
    session_id: Id,
    realm: Uri,
    welcome_message: WelcomeMessage,
    subscriptions: HashMap<Id, Subscription>,
    procedures: HashMap<Id, Procedure>,
    active_invocations: HashMap<Id, Id>,
}

impl Debug for EstablishedSessionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[derive(Debug)]
        #[allow(unused)]
        struct DebugEstablishedSessionState<'a> {
            session_id: &'a Id,
            realm: &'a Uri,
        }

        DebugEstablishedSessionState {
            session_id: &self.session_id,
            realm: &self.realm,
        }
        .fmt(f)
    }
}

#[derive(Debug, Default)]
enum SessionState {
    #[default]
    Closed,
    Establishing(EstablishingSessionState),
    Established(EstablishedSessionState),
    Closing,
}

impl SessionState {
    fn is_same_state(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Closed, Self::Closed) => true,
            (Self::Establishing(_), Self::Establishing(_)) => true,
            (Self::Established(_), Self::Established(_)) => true,
            (Self::Closing, Self::Closing) => true,
            _ => false,
        }
    }
    fn allowed_state_transition(&self, next: &Self) -> bool {
        match (self, next) {
            (Self::Closed, Self::Establishing(_)) => true,
            (Self::Establishing(_), Self::Closed) => true,
            (Self::Establishing(_), Self::Established(_)) => true,
            (Self::Established(_), Self::Closing) => true,
            (Self::Established(_), Self::Closed) => true,
            (Self::Closing, Self::Closed) => true,
            _ => false,
        }
    }
}

/// An event published to a topic.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct PublishedEvent {
    pub arguments: List,
    pub arguments_keyword: Dictionary,
    pub options: PublishOptions,
}

/// An event published to a topic.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReceivedEvent {
    pub arguments: List,
    pub arguments_keyword: Dictionary,

    pub topic: Option<Uri>,
}

/// The result of an RPC, generated by the callee.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct RpcYield {
    pub arguments: List,
    pub arguments_keyword: Dictionary,
}

/// Error for a callee attempting to respond to an RPC with a progressive result when it is not
/// supported by the caller.
#[derive(Debug, Error)]
#[error("invocation does not support progressive results")]
pub struct ProgressiveResultNotSupportedError;

/// An invocation of a procedure.
#[derive(Debug, Clone)]
pub struct Invocation {
    pub arguments: List,
    pub arguments_keyword: Dictionary,

    pub timeout: Duration,
    pub procedure: Option<Uri>,

    pub identity: Identity,

    id: Id,
    message_tx: mpsc::Sender<Message>,
    receive_progress: bool,
}

impl Invocation {
    /// The invocation ID.
    pub fn id(&self) -> Id {
        self.id
    }

    /// Responds to the invocation with a progressive result.
    ///
    /// Fails if the invocation (a.k.a., the caller) does not support progressive results.
    pub async fn progress(&self, rpc_yield: RpcYield) -> Result<()> {
        if !self.receive_progress {
            return Err(ProgressiveResultNotSupportedError.into());
        }
        self.message_tx
            .send(Message::Yield(YieldMessage {
                invocation_request: self.id,
                options: Dictionary::from_iter([("progress".to_owned(), Value::Bool(true))]),
                arguments: rpc_yield.arguments,
                arguments_keyword: rpc_yield.arguments_keyword,
            }))
            .await
            .map_err(Error::new)
    }

    /// Responds to the invocation with a result.
    pub async fn respond<E>(self, rpc_yield: Result<RpcYield, E>) -> Result<()>
    where
        E: Into<WampError>,
    {
        match rpc_yield {
            Ok(rpc_yield) => {
                self.message_tx
                    .send(Message::Yield(YieldMessage {
                        invocation_request: self.id,
                        options: Dictionary::default(),
                        arguments: rpc_yield.arguments,
                        arguments_keyword: rpc_yield.arguments_keyword,
                    }))
                    .await?
            }
            Err(err) => {
                self.message_tx
                    .send(error_for_request(
                        &Message::Invocation(InvocationMessage {
                            request: self.id,
                            ..Default::default()
                        }),
                        &Into::<WampError>::into(err).into(),
                    ))
                    .await?
            }
        }
        Ok(())
    }

    /// Responds to the invocation with a successful result.
    pub async fn respond_ok(self, rpc_yield: RpcYield) -> Result<()> {
        self.respond::<WampError>(Ok(rpc_yield)).await
    }

    /// Responds to the invocation with an error.
    pub async fn respond_error<E>(self, error: E) -> Result<()>
    where
        E: Into<WampError>,
    {
        self.respond(Err(error)).await
    }
}

/// An interrupt of an invocation.
#[derive(Debug, Clone)]
pub struct Interrupt {
    id: Id,
}

impl Interrupt {
    pub fn id(&self) -> Id {
        self.id
    }
}

/// A message for a single procedure that must be strongly ordered.
#[derive(Debug, Clone)]
pub enum ProcedureMessage {
    Invocation(Invocation),
    Interrupt(Interrupt),
}

pub(crate) mod peer_session_message {
    use battler_wamp_values::{
        Dictionary,
        List,
    };
    use tokio::sync::broadcast;

    use crate::{
        core::{
            id::Id,
            uri::Uri,
        },
        message::message::WelcomeMessage,
        peer::{
            ReceivedEvent,
            session::ProcedureMessage,
        },
    };

    /// The result of establishing a session.
    #[derive(Debug, Clone)]
    pub struct EstablishedSession {
        pub realm: Uri,
        pub welcome_message: WelcomeMessage,
    }

    /// A subscription made on a topic.
    #[derive(Debug)]
    pub struct Subscription {
        pub request_id: Id,
        pub subscription_id: Id,
        pub event_rx: broadcast::Receiver<ReceivedEvent>,
    }

    impl Clone for Subscription {
        fn clone(&self) -> Self {
            Self {
                request_id: self.request_id,
                subscription_id: self.subscription_id,
                event_rx: self.event_rx.resubscribe(),
            }
        }
    }

    /// A confirmation that a subscription was dropped.
    #[derive(Debug, Clone)]
    pub struct Unsubscription {
        pub request_id: Id,
    }

    /// A confirmation that an event was published.
    #[derive(Debug, Clone)]
    pub struct Publication {
        pub request_id: Id,
    }

    /// A confirmation that a procedure was registered.
    #[derive(Debug)]
    pub struct Registration {
        pub request_id: Id,
        pub registration_id: Id,
        pub procedure_message_rx: broadcast::Receiver<ProcedureMessage>,
    }

    impl Clone for Registration {
        fn clone(&self) -> Self {
            Self {
                request_id: self.request_id,
                registration_id: self.registration_id,
                procedure_message_rx: self.procedure_message_rx.resubscribe(),
            }
        }
    }

    /// A confirmation that a procedure was deregistered.
    #[derive(Debug, Clone)]
    pub struct Unregistration {
        pub request_id: Id,
    }

    /// A result of a procedure call.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct RpcResult {
        pub request_id: Id,
        pub arguments: List,
        pub arguments_keyword: Dictionary,
        pub progress: bool,
    }
}

#[derive(Debug, Clone)]
struct Subscription {
    event_tx: broadcast::Sender<ReceivedEvent>,
}

#[derive(Debug, Clone)]
struct Procedure {
    procedure_tx: broadcast::Sender<ProcedureMessage>,
}

/// A handle to an asynchronously-running peer session.
pub struct SessionHandle {
    state: Arc<RwLock<SessionState>>,
    id_allocator: Arc<Box<dyn IdAllocator>>,

    established_session_rx:
        broadcast::Receiver<ChannelTransmittableResult<peer_session_message::EstablishedSession>>,
    closed_session_rx: broadcast::Receiver<()>,

    auth_challenge_rx: broadcast::Receiver<ChallengeMessage>,

    subscribed_rx:
        broadcast::Receiver<ChannelTransmittableResult<peer_session_message::Subscription>>,
    unsubscribed_rx:
        broadcast::Receiver<ChannelTransmittableResult<peer_session_message::Unsubscription>>,
    published_rx:
        broadcast::Receiver<ChannelTransmittableResult<peer_session_message::Publication>>,
    registered_rx:
        broadcast::Receiver<ChannelTransmittableResult<peer_session_message::Registration>>,
    unregistered_rx:
        broadcast::Receiver<ChannelTransmittableResult<peer_session_message::Unregistration>>,
    rpc_result_rx: broadcast::Receiver<ChannelTransmittableResult<peer_session_message::RpcResult>>,
}

impl SessionHandle {
    /// The current session ID, as given by the router.
    ///
    /// Since a peer session is reused across multiple router sessions for the same peer, this ID is
    /// subject to change at any point.
    pub async fn current_session_id(&self) -> Option<Id> {
        match &*self.state.read().await {
            SessionState::Established(state) => Some(state.session_id),
            _ => None,
        }
    }

    /// A reference to the session's ID generator.
    pub fn id_allocator(&self) -> Arc<Box<dyn IdAllocator>> {
        self.id_allocator.clone()
    }

    /// The receiver channel for establishing a session (moving the session to the ESTABLISHED
    /// state).
    pub fn established_session_rx(
        &self,
    ) -> broadcast::Receiver<ChannelTransmittableResult<peer_session_message::EstablishedSession>>
    {
        self.established_session_rx.resubscribe()
    }

    /// The receiver channel for authentication challenges.
    pub fn auth_challenge_rx(&self) -> broadcast::Receiver<ChallengeMessage> {
        self.auth_challenge_rx.resubscribe()
    }

    /// The receiver channel, populated when the session moves to the CLOSED state.
    pub fn closed_session_rx(&self) -> broadcast::Receiver<()> {
        self.closed_session_rx.resubscribe()
    }

    /// The receiver channel for responses to SUBSCRIBE messages.
    pub fn subscribed_rx(
        &self,
    ) -> broadcast::Receiver<ChannelTransmittableResult<peer_session_message::Subscription>> {
        self.subscribed_rx.resubscribe()
    }

    /// The receiver channel for responses to UNSUBSCRIBE messages.
    pub fn unsubscribed_rx(
        &self,
    ) -> broadcast::Receiver<ChannelTransmittableResult<peer_session_message::Unsubscription>> {
        self.unsubscribed_rx.resubscribe()
    }

    /// The receiver channel for responses to PUBLISH messages.
    pub fn published_rx(
        &self,
    ) -> broadcast::Receiver<ChannelTransmittableResult<peer_session_message::Publication>> {
        self.published_rx.resubscribe()
    }

    /// The receiver channel for responses to REGISTER messages.
    pub fn registered_rx(
        &self,
    ) -> broadcast::Receiver<ChannelTransmittableResult<peer_session_message::Registration>> {
        self.registered_rx.resubscribe()
    }

    /// The receiver channel for responses to UNREGISTER messages.
    pub fn unregistered_rx(
        &self,
    ) -> broadcast::Receiver<ChannelTransmittableResult<peer_session_message::Unregistration>> {
        self.unregistered_rx.resubscribe()
    }

    /// The receiver channel for responses to CALL messages.
    pub fn rpc_result_rx(
        &self,
    ) -> broadcast::Receiver<ChannelTransmittableResult<peer_session_message::RpcResult>> {
        self.rpc_result_rx.resubscribe()
    }
}

/// The peer end of a WAMP session.
///
/// Handles WAMP messages in a state machine and holds all session-scoped state.
pub struct Session {
    name: String,
    service_message_tx: mpsc::Sender<Message>,
    state: Arc<RwLock<SessionState>>,
    id_allocator: Arc<Box<dyn IdAllocator>>,

    established_session_tx:
        broadcast::Sender<ChannelTransmittableResult<peer_session_message::EstablishedSession>>,
    closed_session_tx: broadcast::Sender<()>,

    auth_challenge_tx: broadcast::Sender<ChallengeMessage>,

    subscribed_tx:
        broadcast::Sender<ChannelTransmittableResult<peer_session_message::Subscription>>,
    unsubscribed_tx:
        broadcast::Sender<ChannelTransmittableResult<peer_session_message::Unsubscription>>,
    published_tx: broadcast::Sender<ChannelTransmittableResult<peer_session_message::Publication>>,
    registered_tx:
        broadcast::Sender<ChannelTransmittableResult<peer_session_message::Registration>>,
    unregistered_tx:
        broadcast::Sender<ChannelTransmittableResult<peer_session_message::Unregistration>>,
    rpc_result_tx: broadcast::Sender<ChannelTransmittableResult<peer_session_message::RpcResult>>,
}

impl Session {
    /// Creates a new session over a service.
    pub fn new(name: String, service_message_tx: mpsc::Sender<Message>) -> Self {
        let id_allocator = SequentialIdAllocator::default();
        let (established_session_tx, _) = broadcast::channel(16);
        let (closed_session_tx, _) = broadcast::channel(16);
        let (auth_challenge_tx, _) = broadcast::channel(16);
        let (subscribed_tx, _) = broadcast::channel(16);
        let (unsubscribed_tx, _) = broadcast::channel(16);
        let (published_tx, _) = broadcast::channel(16);
        let (registered_tx, _) = broadcast::channel(16);
        let (unregistered_tx, _) = broadcast::channel(16);
        let (rpc_result_tx, _) = broadcast::channel(16);
        Self {
            name,
            service_message_tx,
            state: Arc::new(RwLock::new(SessionState::default())),
            id_allocator: Arc::new(Box::new(id_allocator)),
            established_session_tx,
            closed_session_tx,
            auth_challenge_tx,
            subscribed_tx,
            unsubscribed_tx,
            published_tx,
            registered_tx,
            unregistered_tx,
            rpc_result_tx,
        }
    }

    /// The name of the session.
    pub fn name(&self) -> &str {
        &self.name
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
            state: self.state.clone(),
            id_allocator: self.id_allocator.clone(),
            established_session_rx: self.established_session_tx.subscribe(),
            closed_session_rx: self.closed_session_tx.subscribe(),
            auth_challenge_rx: self.auth_challenge_tx.subscribe(),
            subscribed_rx: self.subscribed_tx.subscribe(),
            unsubscribed_rx: self.unsubscribed_tx.subscribe(),
            published_rx: self.published_tx.subscribe(),
            registered_rx: self.registered_tx.subscribe(),
            unregistered_rx: self.unregistered_tx.subscribe(),
            rpc_result_rx: self.rpc_result_tx.subscribe(),
        }
    }

    async fn get_from_establishing_session_state<F, T>(&self, f: F) -> Result<T, Error>
    where
        F: Fn(&EstablishingSessionState) -> T,
    {
        match &*self.state.read().await {
            SessionState::Establishing(state) => Ok(f(&state)),
            _ => Err(Error::msg("session is not in the establishing state")),
        }
    }

    async fn modify_establishing_session_state<F, T>(&self, f: F) -> Result<T, Error>
    where
        F: FnOnce(&mut EstablishingSessionState) -> T,
        T: 'static,
    {
        match &mut *self.state.write().await {
            SessionState::Establishing(state) => Ok(f(state)),
            _ => Err(Error::msg("session is not in the establishing state")),
        }
    }

    async fn get_from_established_session_state<F, T>(&self, f: F) -> Result<T, Error>
    where
        F: Fn(&EstablishedSessionState) -> T,
        T: 'static,
    {
        match &*self.state.read().await {
            SessionState::Established(state) => Ok(f(&state)),
            _ => Err(Error::msg("session is not in the established state")),
        }
    }

    async fn modify_established_session_state<F, T>(&self, f: F) -> Result<T, Error>
    where
        F: FnOnce(&mut EstablishedSessionState) -> T,
        T: 'static,
    {
        match &mut *self.state.write().await {
            SessionState::Established(state) => Ok(f(state)),
            _ => Err(Error::msg("session is not in the established state")),
        }
    }

    /// Sends a message over the session.
    ///
    /// Messages should not be sent directly over the underlying service. By sending messages
    /// through the session, the session state can be updated accordingly.
    pub async fn send_message(&self, message: Message) -> Result<()> {
        match self.transition_state_from_sending_message(&message).await {
            Ok(()) => (),
            Err(err) => {
                // If we failed to transition state, we may need to communicate the error to the
                // peer waiting for join a realm.
                self.established_session_tx.send(Err((&err).into())).ok();
                return Err(err);
            }
        }
        self.service_message_tx
            .send(message)
            .await
            .map_err(Error::new)
    }

    async fn transition_state_from_sending_message(&self, message: &Message) -> Result<()> {
        match message {
            Message::Hello(message) => {
                self.transition_state(SessionState::Establishing(EstablishingSessionState {
                    realm: message.realm.clone(),
                    authenticating: false,
                }))
                .await
            }
            Message::Abort(_) => {
                self.transition_state(SessionState::Closed).await?;

                // The peer may be waiting to join a realm, so we must communicate that the session
                // closed.
                self.established_session_tx
                    .send(Err(Error::msg("session closed").into()))
                    .ok();
                Ok(())
            }
            Message::Goodbye(_) => {
                let next_state = match &*self.state.read().await {
                    SessionState::Closing => SessionState::Closed,
                    _ => SessionState::Closing,
                };
                self.transition_state(next_state).await
            }
            Message::Unsubscribe(message) => {
                self.modify_established_session_state(|state| {
                    state.subscriptions.remove(&message.subscribed_subscription)
                })
                .await?;
                Ok(())
            }
            Message::Unregister(message) => {
                self.modify_established_session_state(|state| {
                    state.procedures.remove(&message.registered_registration)
                })
                .await?;
                Ok(())
            }
            Message::Yield(message) => {
                self.modify_established_session_state(|state| {
                    state.active_invocations.remove(&message.invocation_request)
                })
                .await?;
                Ok(())
            }
            Message::Error(message) => {
                self.modify_established_session_state(|state| {
                    state.active_invocations.remove(&message.request)
                })
                .await?;
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Handles a message over the session state machine.
    pub async fn handle_message(&self, message: Message) -> Result<()> {
        debug!("Peer {} received message: {message:?}", self.name);
        if let Err(err) = self.handle_message_on_state_machine(message).await {
            if !self.state.read().await.is_same_state(&SessionState::Closed) {
                self.send_message(abort_message_for_error(&err)).await?;
            }
            return Err(err);
        }
        Ok(())
    }

    async fn handle_message_on_state_machine(&self, message: Message) -> Result<()> {
        // Read state separately from handling the message, so that we don't lock the session state.
        let mut establishing = false;
        let mut closing = false;
        let mut closed = false;
        match *self.state.read().await {
            SessionState::Establishing(_) => establishing = true,
            SessionState::Closed => closed = true,
            SessionState::Closing => closing = true,
            _ => (),
        }

        if closed {
            Err(InteractionError::ProtocolViolation(format!(
                "received {} message on a closed session",
                message.message_name()
            ))
            .into())
        } else if establishing {
            let result = self.handle_establishing(message).await;
            if let Err(err) = &result {
                self.transition_state(SessionState::Closed).await?;
                self.established_session_tx.send(Err(err.into())).ok();
            }
            result
        } else if closing {
            self.handle_closing(message).await
        } else {
            self.handle_established(message).await
        }
    }

    async fn handle_establishing(&self, message: Message) -> Result<()> {
        match message {
            Message::Welcome(message) => {
                let realm = self
                    .get_from_establishing_session_state(|state| state.realm.clone())
                    .await?;

                self.transition_state(SessionState::Established(EstablishedSessionState {
                    session_id: message.session,
                    realm,
                    welcome_message: message,
                    subscriptions: HashMap::default(),
                    procedures: HashMap::default(),
                    active_invocations: HashMap::default(),
                }))
                .await
            }
            Message::Challenge(message) => {
                let authenticating = self
                    .get_from_establishing_session_state(|state| state.authenticating)
                    .await?;
                if authenticating {
                    return Err(Error::msg("duplicate challenge received"));
                }
                self.modify_establishing_session_state(|state| state.authenticating = true)
                    .await?;
                self.auth_challenge_tx.send(message)?;
                Ok(())
            }
            message @ Message::Abort(_) => Err((&message).into()),
            _ => Err(InteractionError::ProtocolViolation(format!(
                "received {} message on an establishing session",
                message.message_name()
            ))
            .into()),
        }
    }

    async fn handle_established(&self, message: Message) -> Result<()> {
        match message {
            Message::Abort(_) => {
                warn!(
                    "Peer session {} for {} aborted by peer: {message:?}",
                    self.get_from_established_session_state(|state| state.session_id)
                        .await?,
                    self.name
                );
                self.transition_state(SessionState::Closed).await
            }
            Message::Goodbye(_) => {
                self.transition_state(SessionState::Closing).await?;
                self.send_message(goodbye_and_out()).await
            }
            ref message @ Message::Error(ref error_message) => {
                match error_message.request_type {
                    Message::SUBSCRIBE_TAG => {
                        self.subscribed_tx.send(Err(message.try_into()?))?;
                    }
                    Message::UNSUBSCRIBE_TAG => {
                        self.unsubscribed_tx.send(Err(message.try_into()?))?;
                    }
                    Message::PUBLISH_TAG => {
                        self.published_tx.send(Err(message.try_into()?))?;
                    }
                    Message::REGISTER_TAG => {
                        self.registered_tx.send(Err(message.try_into()?))?;
                    }
                    Message::UNREGISTER_TAG => {
                        self.registered_tx.send(Err(message.try_into()?))?;
                    }
                    Message::CALL_TAG => {
                        self.rpc_result_tx.send(Err(message.try_into()?))?;
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
            Message::Subscribed(message) => {
                let (event_tx, event_rx) = broadcast::channel(16);
                self.modify_established_session_state(|state| {
                    state
                        .subscriptions
                        .insert(message.subscription, Subscription { event_tx })
                })
                .await?;
                self.subscribed_tx
                    .send(Ok(peer_session_message::Subscription {
                        request_id: message.subscribe_request,
                        subscription_id: message.subscription,
                        event_rx,
                    }))?;
                Ok(())
            }
            Message::Unsubscribed(message) => {
                self.unsubscribed_tx
                    .send(Ok(peer_session_message::Unsubscription {
                        request_id: message.unsubscribe_request,
                    }))?;
                Ok(())
            }
            Message::Published(message) => {
                self.published_tx
                    .send(Ok(peer_session_message::Publication {
                        request_id: message.publish_request,
                    }))?;
                Ok(())
            }
            Message::Event(message) => {
                let subscription = match self
                    .get_from_established_session_state(|state| {
                        state
                            .subscriptions
                            .get(&message.subscribed_subscription)
                            .cloned()
                    })
                    .await?
                {
                    Some(subscription) => subscription,
                    None => return Ok(()),
                };
                let reported_topic = message
                    .details
                    .get("topic")
                    .and_then(|val| val.string())
                    .and_then(|val| Uri::try_from(val).ok());
                subscription.event_tx.send(ReceivedEvent {
                    arguments: message.publish_arguments,
                    arguments_keyword: message.publish_arguments_keyword,
                    topic: reported_topic,
                })?;
                Ok(())
            }
            Message::Registered(message) => {
                let (procedure_tx, procedure_message_rx) = broadcast::channel(16);
                self.modify_established_session_state(|state| {
                    state
                        .procedures
                        .insert(message.registration, Procedure { procedure_tx })
                })
                .await?;
                self.registered_tx
                    .send(Ok(peer_session_message::Registration {
                        request_id: message.register_request,
                        registration_id: message.registration,
                        procedure_message_rx,
                    }))?;
                Ok(())
            }
            Message::Unregistered(message) => {
                self.unregistered_tx
                    .send(Ok(peer_session_message::Unregistration {
                        request_id: message.unregister_request,
                    }))?;
                Ok(())
            }
            Message::Invocation(message) => {
                let procedure = match self
                    .get_from_established_session_state(|state| {
                        state
                            .procedures
                            .get(&message.registered_registration)
                            .cloned()
                    })
                    .await?
                {
                    Some(procedure) => procedure,
                    None => return Ok(()),
                };
                self.modify_established_session_state(|state| {
                    state
                        .active_invocations
                        .insert(message.request, message.registered_registration)
                })
                .await?;
                let receive_progress = message
                    .details
                    .get("receive_progress")
                    .and_then(|val| val.bool())
                    .unwrap_or(false);
                let timeout = message
                    .details
                    .get("timeout")
                    .and_then(|val| val.integer())
                    .unwrap_or(0);
                let timeout = Duration::from_millis(timeout);
                let reported_procedure = message
                    .details
                    .get("procedure")
                    .and_then(|val| val.string())
                    .and_then(|val| Uri::try_from(val).ok());

                let mut identity = Identity::default();
                if let Some(auth_id) = message.details.get("battler_wamp_authid") {
                    identity.id = auth_id.string().unwrap_or_default().to_owned();
                }
                if let Some(auth_role) = message.details.get("battler_wamp_authrole") {
                    identity.role = auth_role.string().unwrap_or_default().to_owned();
                }

                procedure
                    .procedure_tx
                    .send(ProcedureMessage::Invocation(Invocation {
                        arguments: message.call_arguments,
                        arguments_keyword: message.call_arguments_keyword,
                        timeout,
                        procedure: reported_procedure,
                        identity,
                        id: message.request,
                        message_tx: self.service_message_tx.clone(),
                        receive_progress,
                    }))?;
                Ok(())
            }
            Message::Result(message) => {
                let progress = message
                    .details
                    .get("progress")
                    .and_then(|val| val.bool())
                    .unwrap_or(false);
                self.rpc_result_tx
                    .send(Ok(peer_session_message::RpcResult {
                        request_id: message.call_request,
                        arguments: message.yield_arguments,
                        arguments_keyword: message.yield_arguments_keyword,
                        progress,
                    }))?;
                Ok(())
            }
            Message::Interrupt(message) => {
                let procedure = match self
                    .get_from_established_session_state(|state| {
                        state
                            .active_invocations
                            .get(&message.invocation_request)
                            .and_then(|id| state.procedures.get(&id))
                            .cloned()
                    })
                    .await?
                {
                    Some(procedure) => procedure,
                    None => return Ok(()),
                };
                procedure
                    .procedure_tx
                    .send(ProcedureMessage::Interrupt(Interrupt {
                        id: message.invocation_request,
                    }))?;
                Ok(())
            }
            _ => Err(InteractionError::ProtocolViolation(format!(
                "received {} message on an established session",
                message.message_name()
            ))
            .into()),
        }
    }

    async fn handle_closing(&self, message: Message) -> Result<()> {
        match message {
            Message::Goodbye(_) => self.transition_state(SessionState::Closed).await,
            _ => Err(InteractionError::ProtocolViolation(format!(
                "received {} message on a closing session",
                message.message_name()
            ))
            .into()),
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
            "Peer {} transitioned from {:?} to {state:?}",
            self.name,
            self.state.read().await
        );
        *self.state.write().await = state;

        match &*self.state.read().await {
            SessionState::Established(state) => {
                info!(
                    "Peer {} established session {} on realm {}",
                    self.name, state.session_id, state.realm
                );
                self.id_allocator.reset().await;
                self.established_session_tx
                    .send(Ok(peer_session_message::EstablishedSession {
                        realm: state.realm.clone(),
                        welcome_message: state.welcome_message.clone(),
                    }))?;
            }
            SessionState::Closed => {
                self.closed_session_tx.send(())?;
            }
            _ => (),
        }

        Ok(())
    }
}
