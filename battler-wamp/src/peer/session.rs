use anyhow::{
    Error,
    Result,
};
use log::{
    info,
    trace,
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
        id::Id,
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct EstablishingSessionState {
    realm: Uri,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct EstablishedSessionState {
    session_id: Id,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
enum SessionState {
    #[default]
    Closed,
    Establishing(EstablishingSessionState),
    Established(EstablishedSessionState),
    Closing,
}

impl SessionState {
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

pub mod peer_session_message {
    use crate::{
        core::{
            error::{
                error_from_uri_reason_and_message,
                extract_error_uri_reason_and_message,
            },
            uri::Uri,
        },
        message::message::Message,
    };

    #[derive(Debug, Clone)]
    pub struct Error {
        pub reason: Uri,
        pub message: String,
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
            })
        }
    }

    pub type Result<T> = std::result::Result<T, Error>;

    #[derive(Debug, Clone)]
    pub struct EstablishedSession {
        pub realm: Uri,
    }
}

pub struct SessionHandle {
    message_tx: UnboundedSender<Message>,
    established_session_rx:
        broadcast::Receiver<peer_session_message::Result<peer_session_message::EstablishedSession>>,
    closed_session_rx: broadcast::Receiver<()>,
}

impl SessionHandle {
    pub fn message_tx(&self) -> UnboundedSender<Message> {
        self.message_tx.clone()
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
}

pub struct Session {
    name: String,
    message_tx: UnboundedSender<Message>,
    service_message_tx: UnboundedSender<Message>,
    state: SessionState,

    established_session_tx:
        broadcast::Sender<peer_session_message::Result<peer_session_message::EstablishedSession>>,
    closed_session_tx: broadcast::Sender<()>,
}

impl Session {
    pub fn new(
        name: String,
        message_tx: UnboundedSender<Message>,
        service_message_tx: UnboundedSender<Message>,
    ) -> Self {
        let (established_session_tx, _) = broadcast::channel(16);
        let (closed_session_tx, _) = broadcast::channel(16);
        Self {
            name,
            message_tx,
            service_message_tx,
            state: SessionState::default(),
            established_session_tx,
            closed_session_tx,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn closed(&self) -> bool {
        self.state == SessionState::Closed
    }

    pub fn session_handle(&self) -> SessionHandle {
        SessionHandle {
            message_tx: self.message_tx.clone(),
            established_session_rx: self.established_session_tx.subscribe(),
            closed_session_rx: self.closed_session_tx.subscribe(),
        }
    }

    fn established_session_state(&self) -> Result<&EstablishedSessionState> {
        match &self.state {
            SessionState::Established(state) => Ok(state),
            _ => Err(Error::msg("session is not in the established state")),
        }
    }

    fn establishing_session_state(&self) -> Result<&EstablishingSessionState> {
        match &self.state {
            SessionState::Establishing(state) => Ok(state),
            _ => Err(Error::msg("session is not in the establishing state")),
        }
    }

    pub fn send_message(&mut self, message: Message) -> Result<()> {
        self.transition_state_from_sending_message(&message)?;
        self.service_message_tx.send(message).map_err(Error::new)
    }

    fn transition_state_from_sending_message(&mut self, message: &Message) -> Result<()> {
        let next_state = match message {
            Message::Hello(message) => SessionState::Establishing(EstablishingSessionState {
                realm: message.realm.clone(),
            }),
            Message::Abort(_) => SessionState::Closed,
            Message::Goodbye(_) => match self.state {
                SessionState::Closing => SessionState::Closed,
                _ => SessionState::Closing,
            },
            _ => return Ok(()),
        };
        self.transition_state(next_state)
    }

    pub async fn handle_message(&mut self, message: Message) -> Result<()> {
        trace!("Peer {} received message: {message:?}", self.name);
        if let Err(err) = self.handle_message_on_state_machine(message).await {
            self.send_message(abort_message_for_error(&err))?;
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
                }))?;
                info!(
                    "Peer {} started session {} on realm {realm}",
                    self.name,
                    self.established_session_state()?.session_id
                );
                self.established_session_tx
                    .send(Ok(peer_session_message::EstablishedSession { realm }))?;
                Ok(())
            }
            message @ Message::Abort(_) => {
                self.transition_state(SessionState::Closed)?;
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
                self.transition_state(SessionState::Closed)
            }
            Message::Goodbye(_) => self.send_message(goodbye_and_out()),
            _ => Err(InteractionError::ProtocolViolation(format!(
                "received {} message on an established session",
                message.message_name()
            ))
            .into()),
        }
    }

    async fn handle_closing(&mut self, message: Message) -> Result<()> {
        match message {
            Message::Goodbye(_) => Ok(()),
            _ => Err(InteractionError::ProtocolViolation(format!(
                "received {} message on a closing session",
                message.message_name()
            ))
            .into()),
        }
    }

    fn transition_state(&mut self, state: SessionState) -> Result<()> {
        if state == self.state {
            return Ok(());
        }

        if !self.state.allowed_state_transition(&state) {
            return Err(BasicError::Internal(format!(
                "invalid state transition from {:?} to {state:?}",
                self.state
            ))
            .into());
        }

        trace!(
            "Peer {} transitioned from {:?} to {state:?}",
            self.name,
            self.state
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
}
