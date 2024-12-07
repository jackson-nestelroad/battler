use std::fmt::Debug;

use anyhow::{
    Error,
    Result,
};
use log::{
    debug,
    info,
    warn,
};
use tokio::sync::{
    broadcast,
    mpsc::UnboundedSender,
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
            Message,
            PublishMessage,
            PublishedMessage,
            SubscribeMessage,
            SubscribedMessage,
            UnsubscribeMessage,
            UnsubscribedMessage,
            WelcomeMessage,
        },
    },
    router::{
        context::RouterContext,
        realm::RealmSession,
        topic::TopicManager,
    },
};

struct EstablishedSessionState {
    realm: Uri,
    id_allocator: Box<dyn IdAllocator>,
    subscriptions: HashMap<Id, Uri>,
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

pub struct SessionHandle {
    message_tx: UnboundedSender<Message>,
    closed_session_rx: broadcast::Receiver<()>,
}

impl SessionHandle {
    pub fn send_message(&self, message: Message) -> Result<()> {
        self.message_tx.send(message).map_err(Error::new)
    }

    pub fn close(&self, close_reason: CloseReason) -> Result<()> {
        self.message_tx
            .send(goodbye_with_close_reason(close_reason))
            .map_err(Error::new)
    }

    pub fn closed_session_rx_mut(&mut self) -> &mut broadcast::Receiver<()> {
        &mut self.closed_session_rx
    }
}

pub struct Session {
    id: Id,
    message_tx: UnboundedSender<Message>,
    service_message_tx: UnboundedSender<Message>,
    state: SessionState,

    closed_session_tx: broadcast::Sender<()>,
}

impl Session {
    pub fn new(
        id: Id,
        message_tx: UnboundedSender<Message>,
        service_message_tx: UnboundedSender<Message>,
    ) -> Self {
        let (closed_session_tx, _) = broadcast::channel(16);
        Self {
            id,
            message_tx,
            service_message_tx,
            state: SessionState::default(),

            closed_session_tx,
        }
    }

    pub fn id(&self) -> Id {
        self.id
    }

    pub fn closed(&self) -> bool {
        match self.state {
            SessionState::Closed => true,
            _ => false,
        }
    }

    pub fn session_handle(&self) -> SessionHandle {
        SessionHandle {
            message_tx: self.message_tx.clone(),
            closed_session_rx: self.closed_session_tx.subscribe(),
        }
    }

    fn established_session_state(&self) -> Result<&EstablishedSessionState> {
        match &self.state {
            SessionState::Established(state) => Ok(state),
            _ => Err(Error::msg("session is not in the established state")),
        }
    }

    fn established_session_state_mut(&mut self) -> Result<&mut EstablishedSessionState> {
        match &mut self.state {
            SessionState::Established(state) => Ok(state),
            _ => Err(Error::msg("session is not in the established state")),
        }
    }

    pub fn send_message(&mut self, message: Message) -> Result<()> {
        self.transition_state_from_sending_message(&message)?;
        self.service_message_tx.send(message).map_err(Error::new)
    }

    fn transition_state_from_sending_message(&mut self, message: &Message) -> Result<()> {
        let next_state = match message {
            Message::Abort(_) => SessionState::Closed,
            Message::Goodbye(_) => match self.state {
                SessionState::Closing => SessionState::Closed,
                _ => SessionState::Closing,
            },
            _ => return Ok(()),
        };
        self.transition_state(next_state)
    }

    pub async fn handle_message<S>(
        &mut self,
        context: &RouterContext<S>,
        message: Message,
    ) -> Result<()> {
        debug!("Received message for session {}: {message:?}", self.id);
        if let Err(err) = self.handle_message_on_state_machine(context, message).await {
            self.send_message(abort_message_for_error(&err))?;
            return Err(err);
        }
        Ok(())
    }

    async fn handle_message_on_state_machine<S>(
        &mut self,
        context: &RouterContext<S>,
        message: Message,
    ) -> Result<()> {
        match self.state {
            SessionState::Closed => self.handle_closed(context, message).await,
            SessionState::Established(_) => self.handle_established(context, message).await,
            SessionState::Closing => self.handle_closing(context, message).await,
        }
    }

    async fn handle_closed<S>(
        &mut self,
        context: &RouterContext<S>,
        message: Message,
    ) -> Result<()> {
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
                    id_allocator: Box::new(SequentialIdAllocator::default()),
                    subscriptions: HashMap::default(),
                }))?;

                self.send_message(Message::Welcome(WelcomeMessage {
                    session: self.id,
                    details,
                }))
            }
            _ => Err(InteractionError::ProtocolViolation(format!(
                "received {} message on a closed session",
                message.message_name()
            ))
            .into()),
        }
    }

    async fn handle_established<S>(
        &mut self,
        context: &RouterContext<S>,
        message: Message,
    ) -> Result<()> {
        match message {
            Message::Abort(_) => {
                warn!("Router session {} aborted by peer: {message:?}", self.id);
                self.transition_state(SessionState::Closed)
            }
            Message::Goodbye(_) => {
                self.transition_state(SessionState::Closing)?;
                self.send_message(goodbye_and_out())
            }
            ref whole_message @ Message::Subscribe(ref message) => {
                if let Err(err) = self.handle_subscribe(context, message).await {
                    return self.send_message(error_for_request(&whole_message, &err));
                }
                Ok(())
            }
            ref whole_message @ Message::Unsubscribe(ref message) => {
                if let Err(err) = self.handle_unsubscribe(context, message).await {
                    return self.send_message(error_for_request(&whole_message, &err));
                }
                Ok(())
            }
            ref whole_message @ Message::Publish(ref message) => {
                if let Err(err) = self.handle_publish(context, message).await {
                    return self.send_message(error_for_request(&whole_message, &err));
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
        &mut self,
        context: &RouterContext<S>,
        message: &SubscribeMessage,
    ) -> Result<()> {
        let mut context = context
            .realm_context(&self.established_session_state()?.realm)
            .await?;
        let subscription = TopicManager::subscribe(
            &mut context,
            self.id,
            message.topic.clone(),
            self.established_session_state()?
                .id_allocator
                .generate_id()
                .await,
        )
        .await?;
        self.established_session_state_mut()?
            .subscriptions
            .insert(subscription, message.topic.clone());
        self.send_message(Message::Subscribed(SubscribedMessage {
            subscribe_request: message.request,
            subscription,
        }))
    }

    async fn handle_unsubscribe<S>(
        &mut self,
        context: &RouterContext<S>,
        message: &UnsubscribeMessage,
    ) -> Result<()> {
        let topic = self
            .established_session_state_mut()?
            .subscriptions
            .remove(&message.subscribed_subscription)
            .ok_or_else(|| InteractionError::NoSuchSubscription)?;
        let mut context = context
            .realm_context(&self.established_session_state()?.realm)
            .await?;
        TopicManager::unsubscribe(&mut context, self.id, &topic).await;
        self.send_message(Message::Unsubscribed(UnsubscribedMessage {
            unsubscribe_request: message.request,
        }))
    }

    async fn handle_publish<S>(
        &mut self,
        context: &RouterContext<S>,
        message: &PublishMessage,
    ) -> Result<()> {
        let mut context = context
            .realm_context(&self.established_session_state()?.realm)
            .await?;
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
    }

    async fn handle_closing<S>(&mut self, _: &RouterContext<S>, message: Message) -> Result<()> {
        match message {
            Message::Goodbye(_) => self.transition_state(SessionState::Closed),
            _ => Ok(()),
        }
    }

    fn transition_state(&mut self, state: SessionState) -> Result<()> {
        if state.is_same_state(&self.state) {
            return Ok(());
        }

        if !self.state.allowed_state_transition(&state) {
            return Err(BasicError::Internal(format!(
                "invalid state transition from {:?} to {state:?}",
                self.state
            ))
            .into());
        }

        debug!(
            "Router session {} transitioned from {:?} to {state:?}",
            self.id, self.state
        );
        self.state = state;

        match self.state {
            SessionState::Closed => {
                self.closed_session_tx.send(())?;
            }
            _ => (),
        }

        Ok(())
    }

    pub async fn destroy<S>(self, context: &RouterContext<S>) {
        if let Ok(state) = self.established_session_state() {
            if let Ok(mut context) = context.realm_context(&state.realm).await {
                context.realm_mut().sessions.remove(&self.id);
            }
        }
    }
}
