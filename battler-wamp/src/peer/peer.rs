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
use tokio::sync::{
    broadcast,
    mpsc::{
        unbounded_channel,
        UnboundedReceiver,
    },
};

use crate::{
    core::{
        close::CloseReason,
        id::{
            IdAllocator,
            SequentialIdAllocator,
        },
        roles::PeerRole,
        service::{
            Service,
            ServiceHandle,
        },
        types::{
            Dictionary,
            Value,
        },
        uri::Uri,
    },
    message::{
        common::goodbye_with_close_reason,
        message::{
            HelloMessage,
            Message,
        },
    },
    peer::{
        connector::connector::ConnectorFactory,
        session::{
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

#[derive(Debug, Default)]
pub struct WebSocketConfig {
    pub headers: HashMap<String, String>,
}

#[derive(Debug)]
pub struct PeerConfig {
    pub name: String,
    pub agent: String,
    pub roles: HashSet<PeerRole>,
    pub serializers: HashSet<SerializerType>,
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
}

pub struct Peer<S> {
    config: PeerConfig,
    connector_factory: Box<dyn ConnectorFactory<S>>,
    transport_factory: Box<dyn TransportFactory<S>>,
    id_allocator: Box<dyn IdAllocator>,

    drop_tx: broadcast::Sender<()>,

    peer_state: Arc<Mutex<Option<PeerState>>>,
}

impl<S> Peer<S>
where
    S: Send + 'static,
{
    pub fn new(
        config: PeerConfig,
        connector_factory: Box<dyn ConnectorFactory<S>>,
        transport_factory: Box<dyn TransportFactory<S>>,
    ) -> Result<Self> {
        config.validate()?;
        let (drop_tx, _) = broadcast::channel(1);
        Ok(Self {
            config,
            connector_factory,
            transport_factory,
            id_allocator: Box::new(SequentialIdAllocator::default()),
            drop_tx,
            peer_state: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn connect(&mut self, uri: &str) -> Result<()> {
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

        // Start the service and message handler.
        let service = Service::new(self.config.name.clone(), transport, serializer);
        let (message_tx, message_rx) = unbounded_channel();
        let service_message_rx = service.message_rx();
        let end_rx = service.end_rx();
        let drop_rx = self.drop_tx.subscribe();

        let service_handle = service.start();

        let session = Session::new(
            self.config.name.clone(),
            message_tx,
            service_handle.message_tx(),
        );
        let session_handle = session.session_handle();
        tokio::spawn(Self::message_handler(
            session,
            self.peer_state.clone(),
            message_rx,
            service_message_rx,
            end_rx,
            drop_rx,
        ));

        let mut peer_state = self.peer_state.lock().await;
        *peer_state = Some(PeerState {
            service: service_handle,
            session: session_handle,
        });

        Ok(())
    }

    async fn message_handler(
        mut session: Session,
        peer_state: Arc<Mutex<Option<PeerState>>>,
        message_rx: UnboundedReceiver<Message>,
        service_message_rx: broadcast::Receiver<Message>,
        end_rx: broadcast::Receiver<()>,
        drop_rx: broadcast::Receiver<()>,
    ) {
        match Self::session_loop_with_errors(
            &mut session,
            message_rx,
            service_message_rx,
            end_rx,
            drop_rx,
        )
        .await
        {
            Ok(()) => info!("Peer session {} finished", session.name()),
            Err(err) => error!("Peer session {} failed: {err:#}", session.name()),
        }
        peer_state.lock().await.take();
    }

    async fn session_loop_with_errors(
        session: &mut Session,
        mut message_rx: UnboundedReceiver<Message>,
        mut service_message_rx: broadcast::Receiver<Message>,
        mut end_rx: broadcast::Receiver<()>,
        mut drop_rx: broadcast::Receiver<()>,
    ) -> Result<()> {
        loop {
            tokio::select! {
                // Received a message from this peer object.
                message = message_rx.recv() => {
                    let message = match message {
                        Some(message) => message,
                        None => break,
                    };
                    let message_name = message.message_name();
                    if let Err(err) = session.send_message(message) {
                        return Err(err.context(format!("failed to send {message_name} message")));
                    }
                }
                // Received a message from the service.
                message = service_message_rx.recv() => {
                    let message = match message {
                        Ok(message) => message,
                        Err(_) => break,
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

            if session.closed() {
                break;
            }
        }
        Ok(())
    }

    async fn get_from_peer_state<F, T>(&self, f: F) -> Result<T, Error>
    where
        F: Fn(&PeerState) -> T,
    {
        match &*self.peer_state.lock().await {
            Some(peer_state) => Ok(f(peer_state)),
            None => Err(Error::msg("peer is not connected")),
        }
    }

    pub async fn join_realm(&self, realm: &str) -> Result<()> {
        let (message_tx, mut established_session_rx) = self
            .get_from_peer_state(|peer_state| {
                (
                    peer_state.session.message_tx(),
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

    pub async fn leave_realm(&self) -> Result<()> {
        let (message_tx, mut closed_session_rx) = self
            .get_from_peer_state(|peer_state| {
                (
                    peer_state.session.message_tx(),
                    peer_state.session.closed_session_rx(),
                )
            })
            .await?;

        message_tx.send(goodbye_with_close_reason(CloseReason::Normal))?;
        closed_session_rx.recv().await?;
        Ok(())
    }

    pub async fn disconnect(&mut self) -> Result<()> {
        let mut peer_state = self.peer_state.lock().await;

        match peer_state.take() {
            Some(peer_state) => {
                peer_state.service.cancel()?;
                peer_state.service.join().await?;
            }
            None => (),
        }
        Ok(())
    }
}

impl<S> Drop for Peer<S> {
    fn drop(&mut self) {
        self.drop_tx.send(()).ok();
    }
}
