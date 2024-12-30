use std::{
    net::{
        IpAddr,
        Ipv4Addr,
        SocketAddr,
    },
    sync::Arc,
};

use anyhow::{
    Error,
    Result,
};
use futures_util::lock::Mutex;
use log::{
    debug,
    error,
    info,
};
use tokio::{
    net::{
        TcpListener,
        TcpStream,
    },
    sync::{
        broadcast,
        mpsc::{
            unbounded_channel,
            UnboundedReceiver,
            UnboundedSender,
        },
    },
    task::JoinHandle,
};
use tokio_tungstenite::MaybeTlsStream;
use uuid::Uuid;

use crate::{
    core::{
        close::CloseReason,
        hash::HashSet,
        id::{
            Id,
            IdAllocator,
            RandomIdAllocator,
        },
        roles::RouterRole,
        service::Service,
        stream::{
            DirectMessageStream,
            MessageStream,
            TransportMessageStream,
        },
        uri::Uri,
    },
    router::{
        acceptor::acceptor::AcceptorFactory,
        app::{
            pub_sub::PubSubPolicies,
            rpc::RpcPolicies,
        },
        connection::Connection,
        context::RouterContext,
        realm::{
            Realm,
            RealmConfig,
            RealmManager,
        },
    },
    serializer::serializer::{
        new_serializer,
        SerializerType,
    },
    transport::transport::TransportFactory,
};

const DEFAULT_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "-", env!("CARGO_PKG_VERSION"));

/// Configuration for a [`Router`].
#[derive(Debug)]
pub struct RouterConfig {
    /// IP address the router starts on.
    pub address: IpAddr,
    /// Network port the router starts on.
    pub port: u16,
    /// Agent name, communicated to peers.
    pub agent: String,
    /// Roles implemented by the router.
    pub roles: HashSet<RouterRole>,
    /// Allowed serializers.
    ///
    /// The actual serializer will be selected when the connection with the router is established.
    pub serializers: HashSet<SerializerType>,
    /// Realms available on the router.
    pub realms: Vec<RealmConfig>,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            address: IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)),
            port: 0,
            agent: DEFAULT_AGENT.to_owned(),
            roles: HashSet::from_iter([RouterRole::Broker, RouterRole::Dealer]),
            serializers: HashSet::from_iter([SerializerType::Json, SerializerType::MessagePack]),
            realms: Vec::default(),
        }
    }
}

/// A direct connection made to a router, managed externally in the same process.
pub struct DirectConnection {
    uuid: Uuid,
    stream: Box<dyn MessageStream>,
}

impl DirectConnection {
    /// The unique identifier of the connection.
    pub fn uuid(&self) -> Uuid {
        self.uuid
    }

    /// The message transmission channel.
    pub fn stream(self) -> Box<dyn MessageStream> {
        self.stream
    }
}

/// A message for controlling the router as it is running.
#[derive(Debug)]
pub enum RouterControlMessage {
    /// Ends the session with the given ID in a realm.
    EndSession { realm: Uri, id: Id },
}

/// A handle to an asynchronously-running [`Router`].
///
/// The router's ownership is transferred away when it starts. This handle allows interaction with
/// the router as it is running asynchronously.
#[derive(Clone)]
pub struct RouterHandle {
    local_addr: SocketAddr,
    cancel_tx: broadcast::Sender<()>,
    control_tx: UnboundedSender<RouterControlMessage>,
    direct_connect_fn: Arc<Box<dyn Fn() -> DirectConnection + Send + Sync + 'static>>,
}

impl RouterHandle {
    /// Cancels the router.
    ///
    /// Cancellation is asynchronous. Use the [`JoinHandle`] returned from [`Router::start`] to wait
    /// for the router to stop.
    pub fn cancel(&self) -> Result<()> {
        self.cancel_tx.send(()).map(|_| ()).map_err(Error::new)
    }

    /// The local address of the router.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Starts a direct connection to the router.
    pub fn direct_connect(&self) -> DirectConnection {
        (self.direct_connect_fn)()
    }

    /// Ends the session with the given ID in a realm.
    pub fn end_session(&self, realm: Uri, id: Id) -> Result<()> {
        self.control_tx
            .send(RouterControlMessage::EndSession { realm, id })
            .map_err(Error::new)
    }
}

/// A WAMP router (a.k.a., server) that accepts incoming connections and handles sessions.
pub struct Router<S> {
    /// The router configuration when created.
    pub(crate) config: RouterConfig,

    /// Policies for pub/sub functionality.
    pub(crate) pub_sub_policies: Box<dyn PubSubPolicies<S>>,

    pub(crate) rpc_policies: Box<dyn RpcPolicies<S>>,

    /// Realm manager.
    pub(crate) realm_manager: RealmManager,

    /// The factory for acceptors.
    pub(crate) acceptor_factory: Mutex<Box<dyn AcceptorFactory<S>>>,

    /// The factory for transports.
    pub(crate) transport_factory: Mutex<Box<dyn TransportFactory<S>>>,

    // Allocator for global IDs.
    pub(crate) id_allocator: Box<dyn IdAllocator>,

    cancel_tx: broadcast::Sender<()>,
    cancel_rx: broadcast::Receiver<()>,
    end_tx: broadcast::Sender<()>,
    end_rx: broadcast::Receiver<()>,
}

impl<S> Router<S> {
    /// Receiver channel for determining when the router ends.
    pub(crate) fn end_rx(&self) -> broadcast::Receiver<()> {
        self.end_rx.resubscribe()
    }
}

impl<S> Router<S>
where
    S: Send + 'static,
{
    /// Creates a new [`Router`].
    pub fn new(
        config: RouterConfig,
        pub_sub_policies: Box<dyn PubSubPolicies<S>>,
        rpc_policies: Box<dyn RpcPolicies<S>>,
        acceptor_factory: Box<dyn AcceptorFactory<S>>,
        transport_factory: Box<dyn TransportFactory<S>>,
    ) -> Result<Self> {
        let mut realm_manager = RealmManager::default();
        for realm_config in &config.realms {
            realm_manager.insert(Realm::new(realm_config.clone()));
        }
        let (cancel_tx, cancel_rx) = broadcast::channel(1);
        let (end_tx, end_rx) = broadcast::channel(1);
        Ok(Self {
            config,
            pub_sub_policies,
            rpc_policies,
            realm_manager,
            acceptor_factory: Mutex::new(acceptor_factory),
            transport_factory: Mutex::new(transport_factory),
            id_allocator: Box::new(RandomIdAllocator::default()),
            cancel_tx,
            cancel_rx,
            end_tx,
            end_rx,
        })
    }

    /// Starts the router asynchronously.
    ///
    /// The returned handle can be used to interact with the router since its ownership is
    /// transferred away.
    pub async fn start(self) -> Result<(RouterHandle, JoinHandle<()>), Error> {
        let addr = format!("{}:{}", self.config.address, self.config.port);
        info!(
            "Starting router {} at {addr}: {:?}",
            self.config.agent, self.config
        );
        let listener = TcpListener::bind(&addr).await?;
        let local_addr = listener.local_addr()?;

        // Subscribe to cancellations as soon as possible, so we don't miss messages while we
        // asynchronously set up the connection loop.
        let cancel_rx = self.cancel_rx.resubscribe();

        let cancel_tx = self.cancel_tx.clone();
        let (control_tx, control_rx) = unbounded_channel();
        let context = RouterContext::new(self);
        let start_handle = tokio::spawn(Self::handle_connections(
            context.clone(),
            listener,
            cancel_rx,
            control_rx,
        ));

        Ok((
            RouterHandle {
                local_addr,
                cancel_tx,
                control_tx,
                direct_connect_fn: |context: RouterContext<S>| -> Arc<
                    Box<dyn Fn() -> DirectConnection + Send + Sync + 'static>,
                > {
                    Arc::new(Box::new(move || -> DirectConnection {
                        Router::direct_connect(&context)
                    }))
                }(context.clone()),
            },
            start_handle,
        ))
    }

    async fn handle_connections(
        context: RouterContext<S>,
        listener: TcpListener,
        cancel_rx: broadcast::Receiver<()>,
        control_rx: UnboundedReceiver<RouterControlMessage>,
    ) {
        Self::connection_loop(&context, listener, cancel_rx, control_rx).await;
        Self::shut_down(&context).await;
        if let Err(err) = context.router().end_tx.send(()) {
            error!("Failed to write to end_tx channel after router connection loop ended: {err}");
        }
    }

    async fn connection_loop(
        context: &RouterContext<S>,
        listener: TcpListener,
        mut cancel_rx: broadcast::Receiver<()>,
        mut control_rx: UnboundedReceiver<RouterControlMessage>,
    ) {
        loop {
            tokio::select! {
                accept = listener.accept() => {
                    let (stream, addr) = match accept {
                        Ok((stream, addr)) => (stream, addr),
                        Err(_) => break,
                    };
                    tokio::spawn(Self::handle_connection(
                        context.clone(),
                        addr,
                        MaybeTlsStream::Plain(stream),
                    ));
                }
                control_message = control_rx.recv() => {
                    if let Some(control_message) = control_message {
                        tokio::spawn(Self::handle_control_message(context.clone(), control_message));
                    }
                }
                _ = cancel_rx.recv() => {
                    break;
                }
            }
        }
    }

    async fn handle_connection(
        context: RouterContext<S>,
        addr: SocketAddr,
        stream: MaybeTlsStream<TcpStream>,
    ) {
        if let Err(err) = Self::start_connection(&context, addr, stream).await {
            error!("Failed to start handling connection from {addr}: {err}");
        }
    }

    async fn start_connection(
        context: &RouterContext<S>,
        addr: SocketAddr,
        stream: MaybeTlsStream<TcpStream>,
    ) -> Result<()> {
        debug!("Incoming TCP connection from {addr}");
        let acceptor = context
            .router()
            .acceptor_factory
            .lock()
            .await
            .new_acceptor();
        let acceptance = acceptor.accept(&context, stream).await?;
        debug!("WAMP connection established with {addr}");

        let serializer = new_serializer(acceptance.serializer);
        let transport = context
            .router()
            .transport_factory
            .lock()
            .await
            .new_transport(acceptance.stream, acceptance.serializer);

        Self::start_connection_over_stream(
            context,
            Box::new(TransportMessageStream::new(transport, serializer)),
        );
        Ok(())
    }

    fn start_connection_over_stream(
        context: &RouterContext<S>,
        stream: Box<dyn MessageStream>,
    ) -> Uuid {
        let connection = Connection::new();
        let uuid = connection.uuid();
        info!(
            "Created connection {uuid} over {}",
            stream.message_stream_type()
        );

        let service = Service::new(connection.uuid().to_string(), stream);
        connection.start(context.clone(), service);
        uuid
    }

    async fn handle_control_message(
        context: RouterContext<S>,
        control_message: RouterControlMessage,
    ) {
        match control_message {
            RouterControlMessage::EndSession { realm, id } => {
                if let Err(err) = Self::end_session(&context, &realm, id).await {
                    error!("Failed to end session {id} in realm {realm}: {err}");
                }
            }
        }
    }

    async fn end_session(context: &RouterContext<S>, realm: &Uri, id: Id) -> Result<()> {
        let context = context.realm_context(realm)?;
        match context.session(id).await {
            Some(session) => session.session.close(CloseReason::Killed)?,
            None => (),
        }
        Ok(())
    }

    async fn shut_down(context: &RouterContext<S>) {
        let realms = context
            .router()
            .realm_manager
            .uris()
            .cloned()
            .collect::<Vec<_>>();
        for uri in realms {
            if let Err(err) =
                Self::shut_down_realm(context, &uri, CloseReason::SystemShutdown).await
            {
                error!("Failed to shut down realm {uri}: {err}");
            }
        }
    }

    async fn shut_down_realm(
        context: &RouterContext<S>,
        realm: &Uri,
        close_reason: CloseReason,
    ) -> Result<()> {
        let realm = match context.router().realm_manager.get(realm) {
            Some(realm) => realm,
            None => return Ok(()),
        };
        realm.shut_down(close_reason).await
    }

    fn direct_connect(context: &RouterContext<S>) -> DirectConnection {
        let (router_to_peer_tx, router_to_peer_rx) = unbounded_channel();
        let (peer_to_router_tx, peer_to_router_rx) = unbounded_channel();
        let router_stream = DirectMessageStream::new(router_to_peer_tx, peer_to_router_rx);
        let peer_stream = DirectMessageStream::new(peer_to_router_tx, router_to_peer_rx);
        let uuid = Self::start_connection_over_stream(context, Box::new(router_stream));
        DirectConnection {
            uuid,
            stream: Box::new(peer_stream),
        }
    }
}
