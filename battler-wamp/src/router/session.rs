use std::{
    fmt::Debug,
    sync::Arc,
};

use anyhow::{
    Error,
    Result,
};
use log::{
    debug,
    error,
    info,
    warn,
};
use tokio::sync::{
    broadcast::{
        self,
        error::RecvError,
    },
    mpsc::UnboundedSender,
    RwLock,
};

use crate::{
    core::{
        close::CloseReason,
        error::{
            BasicError,
            InteractionError,
        },
        hash::HashMap,
        id::{
            Id,
            IdAllocator,
            SequentialIdAllocator,
        },
        types::{
            Dictionary,
            Value,
        },
        uri::Uri,
    },
    message::{
        common::{
            abort_message_for_error,
            error_for_request,
            goodbye_and_out,
            goodbye_with_close_reason,
        },
        message::{
            CallMessage,
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
        context::RouterContext,
        procedure::ProcedureManager,
        realm::RealmSession,
        topic::TopicManager,
    },
};

struct EstablishedSessionState {
    realm: Uri,
    subscriptions: HashMap<Id, Uri>,
    procedures: HashMap<Id, Uri>,
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

#[derive(Debug, Default)]
enum SessionState {
    #[default]
    Closed,
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
            (Self::Closed, Self::Established(_)) => true,
            (Self::Established(_), Self::Closing) => true,
            (Self::Established(_), Self::Closed) => true,
            (Self::Closing, Self::Closed) => true,
            _ => false,
        }
    }
}

mod router_session_message {
    use crate::{
        core::{
            error::{
                error_from_uri_reason_and_message,
                extract_error_uri_reason_and_message,
            },
            id::Id,
            types::{
                Dictionary,
                List,
            },
            uri::Uri,
        },
        message::message::Message,
    };

    /// An error that can be transmitted over peer session channels.
    #[derive(Debug, Clone)]
    pub struct Error {
        pub reason: Uri,
        pub message: String,
        pub request_id: Option<Id>,
    }

    impl Error {
        /// Converts the error into a real Error object that can be returned out.
        pub fn into_error(self) -> anyhow::Error {
            error_from_uri_reason_and_message(self.reason, self.message)
        }
    }

    impl TryFrom<&Message> for Error {
        type Error = anyhow::Error;
        fn try_from(value: &Message) -> std::result::Result<Self, Self::Error> {
            let (reason, message) = extract_error_uri_reason_and_message(&value)?;
            Ok(Self {
                reason: reason.to_owned(),
                message: message.to_owned(),
                request_id: value.request_id(),
            })
        }
    }

    impl From<&anyhow::Error> for Error {
        fn from(value: &anyhow::Error) -> Self {
            Self {
                reason: Uri::for_error(value),
                message: value.to_string(),
                request_id: None,
            }
        }
    }

    /// A result that can be transmitted over peer session channels.
    pub type Result<T> = std::result::Result<T, Error>;

    /// The result of an RPC invocation.
    #[derive(Debug, Clone)]
    pub struct RpcYield {
        pub request_id: Id,
        pub arguments: List,
        pub arguments_keyword: Dictionary,
    }
}

/// A handle to an asynchronously-running router session.
pub struct SessionHandle {
    id_allocator: Arc<Box<dyn IdAllocator>>,
    message_tx: UnboundedSender<Message>,
    closed_session_rx: broadcast::Receiver<()>,

    rpc_yield_rx:
        broadcast::Receiver<router_session_message::Result<router_session_message::RpcYield>>,
}

impl SessionHandle {
    /// A reference to the session's ID generator.
    pub fn id_generator(&self) -> Arc<Box<dyn IdAllocator>> {
        self.id_allocator.clone()
    }

    /// Sends a message over the session.
    pub fn send_message(&self, message: Message) -> Result<()> {
        self.message_tx.send(message).map_err(Error::new)
    }

    /// Closes the session.
    pub fn close(&self, close_reason: CloseReason) -> Result<()> {
        self.message_tx
            .send(goodbye_with_close_reason(close_reason))
            .map_err(Error::new)
    }

    /// A mutable reference to the receiver channel that is populated when the session moves to the
    /// CLOSED state.
    pub fn closed_session_rx_mut(&mut self) -> &mut broadcast::Receiver<()> {
        &mut self.closed_session_rx
    }

    ///The receiver channel for responses to INVOCATION messages.
    pub fn rpc_yield_rx(
        &self,
    ) -> broadcast::Receiver<router_session_message::Result<router_session_message::RpcYield>> {
        self.rpc_yield_rx.resubscribe()
    }
}

/// The router end of a WAMP session.
///
/// Handles WAMP messages in a state machine and holds all session-scoped state.
pub struct Session {
    id: Id,
    message_tx: UnboundedSender<Message>,
    service_message_tx: UnboundedSender<Message>,
    state: RwLock<SessionState>,
    id_allocator: Arc<Box<dyn IdAllocator>>,

    closed_session_tx: broadcast::Sender<()>,

    rpc_yield_tx:
        broadcast::Sender<router_session_message::Result<router_session_message::RpcYield>>,
}

impl Session {
    /// Creates a new session over a service.
    pub fn new(
        id: Id,
        message_tx: UnboundedSender<Message>,
        service_message_tx: UnboundedSender<Message>,
    ) -> Self {
        let id_allocator = SequentialIdAllocator::default();
        let (closed_session_tx, _) = broadcast::channel(16);
        let (rpc_yield_tx, _) = broadcast::channel(16);
        Self {
            id,
            message_tx,
            service_message_tx,
            state: RwLock::new(SessionState::default()),
            id_allocator: Arc::new(Box::new(id_allocator)),
            closed_session_tx,
            rpc_yield_tx,
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
            id_allocator: self.id_allocator.clone(),
            message_tx: self.message_tx.clone(),
            closed_session_rx: self.closed_session_tx.subscribe(),
            rpc_yield_rx: self.rpc_yield_tx.subscribe(),
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
        F: Fn(&mut EstablishedSessionState) -> T,
    {
        match &mut *self.state.write().await {
            SessionState::Established(ref mut state) => Ok(f(state)),
            _ => Err(Error::msg("session is not in the established state")),
        }
    }

    pub async fn send_message(&self, message: Message) -> Result<()> {
        self.transition_state_from_sending_message(&message).await?;
        self.service_message_tx.send(message).map_err(Error::new)
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
        context: &RouterContext<S>,
        message: Message,
    ) -> Result<()> {
        debug!("Received message for session {}: {message:?}", self.id);
        if let Err(err) = self.handle_message_on_state_machine(context, message).await {
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
        match *self.state.read().await {
            SessionState::Closed => closed = true,
            SessionState::Closing => closing = true,
            _ => (),
        }

        if closed {
            self.handle_closed(context, message).await
        } else if closing {
            self.handle_closing(context, message).await
        } else {
            self.handle_established(context, message).await
        }
    }

    async fn handle_closed<S>(&self, context: &RouterContext<S>, message: Message) -> Result<()> {
        match message {
            Message::Hello(message) => {
                let mut context = context.realm_context(&message.realm).await?;
                context.realm_mut().sessions.insert(
                    self.id,
                    RealmSession {
                        session: self.session_handle(),
                    },
                );
                info!("Session {} joined realm {}", self.id, context.realm().uri());

                let mut details = Dictionary::default();
                details.insert(
                    "agent".to_owned(),
                    Value::String(context.router().config.agent.clone()),
                );
                details.insert(
                    "roles".to_owned(),
                    Value::Dictionary(
                        context
                            .router()
                            .config
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

                self.transition_state(SessionState::Established(EstablishedSessionState {
                    realm: context.realm().uri().clone(),
                    subscriptions: HashMap::default(),
                    procedures: HashMap::default(),
                }))
                .await?;

                self.send_message(Message::Welcome(WelcomeMessage {
                    session: self.id,
                    details,
                }))
                .await
            }
            _ => Err(InteractionError::ProtocolViolation(format!(
                "received {} message on a closed session",
                message.message_name()
            ))
            .into()),
        }
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
        let mut context = context.realm_context(&realm).await?;
        let subscription =
            TopicManager::subscribe(&mut context, self.id, message.topic.clone()).await?;
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
        TopicManager::activate_subscription(&mut context, self.id, &message.topic);
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
        let mut context = context.realm_context(&realm).await?;
        TopicManager::unsubscribe(&mut context, self.id, &topic).await;
        self.send_message(Message::Unsubscribed(UnsubscribedMessage {
            unsubscribe_request: message.request,
        }))
        .await
    }

    async fn handle_publish<S>(
        &self,
        context: &RouterContext<S>,
        message: &PublishMessage,
    ) -> Result<()> {
        let realm = self
            .get_from_established_session_state(|state| state.realm.clone())
            .await?;
        let mut context = context.realm_context(&realm).await?;
        let publication = TopicManager::publish(
            &mut context,
            self.id,
            &message.topic,
            message.arguments.clone(),
            message.arguments_keyword.clone(),
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
        let mut context = context.realm_context(&realm).await?;
        let registration =
            ProcedureManager::register(&mut context, self.id, message.procedure.clone()).await?;
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
        ProcedureManager::activate_procedure(&mut context, &message.procedure);
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
        let mut context = context.realm_context(&realm).await?;
        ProcedureManager::unregister(&mut context, &procedure).await;
        self.send_message(Message::Unregistered(UnregisteredMessage {
            unregister_request: message.request,
        }))
        .await
    }

    async fn handle_call<S>(
        &self,
        context: &RouterContext<S>,
        message: &CallMessage,
    ) -> Result<()> {
        let realm = self
            .get_from_established_session_state(|state| state.realm.clone())
            .await?;
        let context = context.realm_context(&realm).await?;
        let procedure = context
            .realm()
            .procedure_manager
            .procedures
            .get(&message.procedure)
            .ok_or_else(|| InteractionError::NoSuchProcedure)?;
        let registration_id = procedure.registration_id;
        let callee = procedure.callee;
        let callee =
            context.realm().sessions.get(&callee).ok_or_else(|| {
                BasicError::NotFound("expected callee session to exist".to_owned())
            })?;
        let request_id = self.id_allocator.generate_id().await;
        let rpc_yield_rx = callee.session.rpc_yield_rx();
        callee
            .session
            .send_message(Message::Invocation(InvocationMessage {
                request: request_id,
                registered_registration: registration_id,
                details: Dictionary::default(),
                call_arguments: message.arguments.clone(),
                call_arguments_keyword: message.arguments_keyword.clone(),
            }))?;
        let rpc_yield = Self::wait_for_rpc_yield(rpc_yield_rx, request_id).await?;
        self.send_message(Message::Result(ResultMessage {
            call_request: message.request,
            details: Dictionary::default(),
            yield_arguments: rpc_yield.arguments,
            yield_arguments_keyword: rpc_yield.arguments_keyword,
        }))
        .await
    }

    async fn wait_for_rpc_yield(
        mut rpc_yield_rx: broadcast::Receiver<
            router_session_message::Result<router_session_message::RpcYield>,
        >,
        request_id: Id,
    ) -> Result<router_session_message::RpcYield> {
        loop {
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
                                return Err(err.into_error());
                            }
                        }
                    }
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
            }))?;
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
                self.closed_session_tx.send(())?;
            }
            _ => (),
        }

        Ok(())
    }

    pub async fn clean_up<S>(self, context: &RouterContext<S>) {
        let id = self.id;

        // We only need to clean up if we have resources in a realm.
        let realm = match self
            .get_from_established_session_state(|state| state.realm.clone())
            .await
        {
            Ok(realm) => realm,
            Err(_) => return,
        };

        let mut context = match context.realm_context(&realm).await {
            Ok(context) => context,
            Err(err) => {
                error!("Failed to clean up session {id}, due to error getting context for realm {realm}: {err:?}");
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

                context.realm_mut().sessions.remove(&id);
            }
            _ => (),
        }
    }
}
