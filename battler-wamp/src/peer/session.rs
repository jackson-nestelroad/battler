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
    broadcast,
    mpsc::UnboundedSender,
};

use crate::{
    core::{
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
            List,
        },
        uri::Uri,
    },
    message::{
        common::{
            abort_message_for_error,
            goodbye_and_out,
        },
        message::Message,
    },
};

#[derive(Debug)]
struct EstablishingSessionState {
    realm: Uri,
}

struct EstablishedSessionState {
    session_id: Id,
    realm: Uri,
    subscriptions: HashMap<Id, Subscription>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub arguments: List,
    pub arguments_keyword: Dictionary,
}

pub mod peer_session_message {

    use tokio::sync::broadcast;

    use crate::{
        core::{
            error::{
                error_from_uri_reason_and_message,
                extract_error_uri_reason_and_message,
            },
            id::Id,
            uri::Uri,
        },
        message::message::Message,
        peer::session::Event,
    };

    #[derive(Debug, Clone)]
    pub struct Error {
        pub reason: Uri,
        pub message: String,
        pub request_id: Option<Id>,
    }

    impl Error {
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

    pub type Result<T> = std::result::Result<T, Error>;

    #[derive(Debug, Clone)]
    pub struct EstablishedSession {
        pub realm: Uri,
    }

    #[derive(Debug)]
    pub struct Subscription {
        pub request_id: Id,
        pub subscription_id: Id,
        pub event_rx: broadcast::Receiver<Event>,
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

    #[derive(Debug, Clone)]
    pub struct Unsubscription {
        pub request_id: Id,
    }

    #[derive(Debug, Clone)]
    pub struct Publication {
        pub request_id: Id,
    }
}

struct Subscription {
    event_tx: broadcast::Sender<Event>,
}

pub struct SessionHandle {
    id_allocator: Arc<Box<dyn IdAllocator>>,

    established_session_rx:
        broadcast::Receiver<peer_session_message::Result<peer_session_message::EstablishedSession>>,
    closed_session_rx: broadcast::Receiver<()>,

    subscribed_rx:
        broadcast::Receiver<peer_session_message::Result<peer_session_message::Subscription>>,
    unsubscribed_rx:
        broadcast::Receiver<peer_session_message::Result<peer_session_message::Unsubscription>>,
    published_rx:
        broadcast::Receiver<peer_session_message::Result<peer_session_message::Publication>>,
}

impl SessionHandle {
    pub fn id_generator(&self) -> Arc<Box<dyn IdAllocator>> {
        self.id_allocator.clone()
    }

    pub fn established_session_rx(
        &self,
    ) -> broadcast::Receiver<peer_session_message::Result<peer_session_message::EstablishedSession>>
    {
        self.established_session_rx.resubscribe()
    }

    pub fn closed_session_rx(&self) -> broadcast::Receiver<()> {
        self.closed_session_rx.resubscribe()
    }

    pub fn subscribed_rx(
        &self,
    ) -> broadcast::Receiver<peer_session_message::Result<peer_session_message::Subscription>> {
        self.subscribed_rx.resubscribe()
    }

    pub fn unsubscribed_rx(
        &self,
    ) -> broadcast::Receiver<peer_session_message::Result<peer_session_message::Unsubscription>>
    {
        self.unsubscribed_rx.resubscribe()
    }

    pub fn published_rx(
        &self,
    ) -> broadcast::Receiver<peer_session_message::Result<peer_session_message::Publication>> {
        self.published_rx.resubscribe()
    }
}

pub struct Session {
    name: String,
    service_message_tx: UnboundedSender<Message>,
    state: SessionState,
    id_allocator: Arc<Box<dyn IdAllocator>>,

    established_session_tx:
        broadcast::Sender<peer_session_message::Result<peer_session_message::EstablishedSession>>,
    closed_session_tx: broadcast::Sender<()>,

    subscribed_tx:
        broadcast::Sender<peer_session_message::Result<peer_session_message::Subscription>>,
    unsubscribed_tx:
        broadcast::Sender<peer_session_message::Result<peer_session_message::Unsubscription>>,
    published_tx:
        broadcast::Sender<peer_session_message::Result<peer_session_message::Publication>>,
}

impl Session {
    pub fn new(name: String, service_message_tx: UnboundedSender<Message>) -> Self {
        let id_allocator = SequentialIdAllocator::default();
        let (established_session_tx, _) = broadcast::channel(16);
        let (closed_session_tx, _) = broadcast::channel(16);
        let (subscribed_tx, _) = broadcast::channel(16);
        let (unsubscribed_tx, _) = broadcast::channel(16);
        let (published_tx, _) = broadcast::channel(16);
        Self {
            name,
            service_message_tx,
            state: SessionState::default(),
            id_allocator: Arc::new(Box::new(id_allocator)),
            established_session_tx,
            closed_session_tx,
            subscribed_tx,
            unsubscribed_tx,
            published_tx,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn closed(&self) -> bool {
        match self.state {
            SessionState::Closed => true,
            _ => false,
        }
    }

    pub fn session_handle(&self) -> SessionHandle {
        SessionHandle {
            id_allocator: self.id_allocator.clone(),
            established_session_rx: self.established_session_tx.subscribe(),
            closed_session_rx: self.closed_session_tx.subscribe(),
            subscribed_rx: self.subscribed_tx.subscribe(),
            unsubscribed_rx: self.unsubscribed_tx.subscribe(),
            published_rx: self.published_tx.subscribe(),
        }
    }

    fn establishing_session_state(&self) -> Result<&EstablishingSessionState> {
        match &self.state {
            SessionState::Establishing(state) => Ok(state),
            _ => Err(Error::msg("session is not in the establishing state")),
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

    pub async fn send_message(&mut self, message: Message) -> Result<()> {
        match self.transition_state_from_sending_message(&message).await {
            Ok(()) => (),
            Err(err) => {
                match &message {
                    Message::Hello(_) => {
                        self.established_session_tx.send(Err((&err).into()))?;
                    }
                    _ => (),
                }
                return Err(err);
            }
        }
        self.service_message_tx.send(message).map_err(Error::new)
    }

    async fn transition_state_from_sending_message(&mut self, message: &Message) -> Result<()> {
        match message {
            Message::Hello(message) => {
                self.transition_state(SessionState::Establishing(EstablishingSessionState {
                    realm: message.realm.clone(),
                }))
                .await
            }
            Message::Abort(_) => self.transition_state(SessionState::Closed).await,
            Message::Goodbye(_) => {
                self.transition_state(match self.state {
                    SessionState::Closing => SessionState::Closed,
                    _ => SessionState::Closing,
                })
                .await
            }
            Message::Unsubscribe(message) => {
                self.established_session_state_mut()?
                    .subscriptions
                    .remove(&message.subscribed_subscription);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    pub async fn handle_message(&mut self, message: Message) -> Result<()> {
        debug!("Peer {} received message: {message:?}", self.name);
        if let Err(err) = self.handle_message_on_state_machine(message).await {
            self.send_message(abort_message_for_error(&err)).await?;
            return Err(err);
        }
        Ok(())
    }

    async fn handle_message_on_state_machine(&mut self, message: Message) -> Result<()> {
        match self.state {
            SessionState::Closed => Err(InteractionError::ProtocolViolation(format!(
                "received {} message on a closed session",
                message.message_name()
            ))
            .into()),
            SessionState::Establishing(_) => self.handle_establishing(message).await,
            SessionState::Established(_) => self.handle_established(message).await,
            SessionState::Closing => self.handle_closing(message).await,
        }
    }

    async fn handle_establishing(&mut self, message: Message) -> Result<()> {
        match message {
            Message::Welcome(message) => {
                let realm = self.establishing_session_state()?.realm.clone();
                self.transition_state(SessionState::Established(EstablishedSessionState {
                    session_id: message.session,
                    realm,
                    subscriptions: HashMap::default(),
                }))
                .await
            }
            message @ Message::Abort(_) => {
                self.transition_state(SessionState::Closed).await?;
                self.established_session_tx
                    .send(Err((&message).try_into()?))?;
                Ok(())
            }
            _ => Err(InteractionError::ProtocolViolation(format!(
                "received {} message on an establishing session",
                message.message_name()
            ))
            .into()),
        }
    }

    async fn handle_established(&mut self, message: Message) -> Result<()> {
        match message {
            Message::Abort(_) => {
                warn!(
                    "Peer session {} for {} aborted by peer: {message:?}",
                    self.established_session_state()?.session_id,
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
                    _ => {
                        error!(
                            "Invalid ERROR mesage with request type {} received from the router: {error_message:?}",
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
                self.established_session_state_mut()?
                    .subscriptions
                    .insert(message.subscription, Subscription { event_tx });
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
                    .established_session_state()?
                    .subscriptions
                    .get(&message.subscribed_subscription)
                {
                    Some(subscription) => subscription,
                    None => return Ok(()),
                };
                subscription.event_tx.send(Event {
                    arguments: message.publish_arguments,
                    arguments_keyword: message.publish_arguments_keyword,
                })?;
                Ok(())
            }
            _ => Err(InteractionError::ProtocolViolation(format!(
                "received {} message on an established session",
                message.message_name()
            ))
            .into()),
        }
    }

    async fn handle_closing(&mut self, message: Message) -> Result<()> {
        match message {
            Message::Goodbye(_) => self.transition_state(SessionState::Closed).await,
            _ => Err(InteractionError::ProtocolViolation(format!(
                "received {} message on a closing session",
                message.message_name()
            ))
            .into()),
        }
    }

    async fn transition_state(&mut self, state: SessionState) -> Result<()> {
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
            "Peer {} transitioned from {:?} to {state:?}",
            self.name, self.state
        );
        self.state = state;

        match &self.state {
            SessionState::Established(state) => {
                info!(
                    "Peer {} started session {} on realm {}",
                    self.name, state.session_id, state.realm
                );
                self.id_allocator.reset().await;
                self.established_session_tx
                    .send(Ok(peer_session_message::EstablishedSession {
                        realm: state.realm.clone(),
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
