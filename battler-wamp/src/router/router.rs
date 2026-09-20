use std::{
    net::{
        IpAddr,
        Ipv4Addr,
        SocketAddr,
    },
    sync::Arc,
    time::Duration,
};

use anyhow::{
    Error,
    Result,
};
use battler_wamp_uri::Uri;
use futures_util::lock::Mutex;
use log::{
    debug,
    error,
    info,
    warn,
};
use tokio::{
    net::{
        TcpListener,
        TcpStream,
    },
    sync::{
        broadcast,
        mpsc,
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
        rate_limiter::TokenBucketRateLimiter,
        roles::RouterRole,
        service::Service,
        stream::{
            DirectMessageStream,
            MessageStream,
            TransportMessageStream,
        },
    },
    router::{
        acceptor::acceptor::AcceptorFactory,
        app::{
            connection::ConnectionPolicies,
            pub_sub::PubSubPolicies,
            rpc::RpcPolicies,
        },
        connection::Connection,
        connection_tracker::{
            ConnectionGuard,
            ConnectionTracker,
        },
        context::RouterContext,
        realm::{
            Realm,
            RealmConfig,
            RealmManager,
        },
    },
    serializer::serializer::{
        SerializerType,
        new_serializer,
    },
    transport::transport::TransportFactory,
};

const DEFAULT_AGENT: &str = concat!(env!("CARGO_PKG_NAME"), "-", env!("CARGO_PKG_VERSION"));

/// Limits and protection configuration for a [`Router`].
#[derive(Clone, Debug)]
pub struct RouterLimitsConfig {
    /// Maximum concurrent network connections across the server.
    pub max_connections: Option<usize>,
    /// Maximum concurrent network connections allowed per client IP.
    pub max_connections_per_ip: Option<usize>,
    /// Whether loopback addresses (127.0.0.1, ::1) are exempt from per-IP limits.
    pub exempt_loopback: bool,
    /// Maximum incoming message size in bytes.
    pub max_message_size_bytes: usize,
    /// Maximum incoming frame size in bytes.
    pub max_frame_size_bytes: usize,
    /// Maximum burst of incoming messages allowed in a token bucket.
    pub rate_limit_burst: u32,
    /// Sustained message refill rate (tokens/second) per connection.
    pub rate_limit_refill_per_sec: u32,
    /// Idle timeout duration before disconnecting inactive connections.
    pub idle_timeout: Option<Duration>,
    /// Maximum allowed duration to complete the WebSocket upgrade handshake.
    pub handshake_timeout: Duration,
}

impl Default for RouterLimitsConfig {
    fn default() -> Self {
        Self {
            max_connections: Some(10_000),
            max_connections_per_ip: Some(10),
            exempt_loopback: true,
            max_message_size_bytes: 64 * 1024,
            max_frame_size_bytes: 16 * 1024,
            rate_limit_burst: 30,
            rate_limit_refill_per_sec: 10,
            idle_timeout: Some(Duration::from_secs(60)),
            handshake_timeout: Duration::from_secs(10),
        }
    }
}

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
    /// Limits and DDoS protection configuration.
    pub limits: RouterLimitsConfig,
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
            limits: RouterLimitsConfig::default(),
        }
    }
}

/// A direct connection made to a router, managed externally in the same process.
#[derive(Debug)]
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
    control_tx: mpsc::Sender<RouterControlMessage>,
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
    pub async fn end_session(&self, realm: Uri, id: Id) -> Result<()> {
        self.control_tx
            .send(RouterControlMessage::EndSession { realm, id })
            .await
            .map_err(Error::new)
    }
}

/// A WAMP router (a.k.a., server) that accepts incoming connections and handles sessions.
pub struct Router<S> {
    /// The router configuration when created.
    pub(crate) config: RouterConfig,

    /// Policies for connection functionality.
    pub(crate) connection_policies: Box<dyn ConnectionPolicies<S>>,

    /// Policies for pub/sub functionality.
    pub(crate) pub_sub_policies: Box<dyn PubSubPolicies<S>>,

    pub(crate) rpc_policies: Box<dyn RpcPolicies<S>>,

    /// Realm manager.
    pub(crate) realm_manager: RealmManager,

    /// The factory for acceptors.
    pub(crate) acceptor_factory: Mutex<Box<dyn AcceptorFactory<S>>>,

    /// The factory for transports.
    pub(crate) transport_factory: Mutex<Box<dyn TransportFactory<S>>>,

    /// Allocator for global IDs.
    pub(crate) id_allocator: Box<dyn IdAllocator>,

    /// Connection tracker for global and per-IP connection limits.
    pub(crate) connection_tracker: ConnectionTracker,

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
        connection_policies: Box<dyn ConnectionPolicies<S>>,
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
        let connection_tracker = ConnectionTracker::new(
            config.limits.max_connections,
            config.limits.max_connections_per_ip,
            config.limits.exempt_loopback,
        );
        Ok(Self {
            config,
            connection_policies,
            pub_sub_policies,
            rpc_policies,
            realm_manager,
            acceptor_factory: Mutex::new(acceptor_factory),
            transport_factory: Mutex::new(transport_factory),
            id_allocator: Box::new(RandomIdAllocator::default()),
            connection_tracker,
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

        for (uri, realm) in &self.realm_manager.realms {
            realm
                .initialize()
                .await
                .map_err(|err| err.context(format!("failed to initialize realm {uri}")))?;
        }

        let listener = TcpListener::bind(&addr).await?;
        let local_addr = listener.local_addr()?;

        // Subscribe to cancellations as soon as possible, so we don't miss messages while we
        // asynchronously set up the connection loop.
        let cancel_rx = self.cancel_rx.resubscribe();

        let cancel_tx = self.cancel_tx.clone();
        let (control_tx, control_rx) = mpsc::channel(48);
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
        control_rx: mpsc::Receiver<RouterControlMessage>,
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
        mut control_rx: mpsc::Receiver<RouterControlMessage>,
    ) {
        loop {
            tokio::select! {
                accept = listener.accept() => {
                    let (stream, addr) = match accept {
                        Ok((stream, addr)) => (stream, addr),
                        Err(err) => {
                            error!("Failed to accept incoming TCP connection: {err}");
                            tokio::time::sleep(Duration::from_millis(50)).await;
                            continue;
                        }
                    };
                    let guard = match context.router().connection_tracker.try_acquire(addr.ip()) {
                        Ok(guard) => guard,
                        Err(err) => {
                            warn!("Rejected TCP connection from {addr}: {err}");
                            continue;
                        }
                    };
                    tokio::spawn(Self::handle_connection(
                        context.clone(),
                        addr,
                        MaybeTlsStream::Plain(stream),
                        guard,
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
        guard: ConnectionGuard,
    ) {
        let handshake_timeout = context.router().config.limits.handshake_timeout;
        let start_res = tokio::time::timeout(
            handshake_timeout,
            Self::start_connection(&context, addr, stream, guard),
        )
        .await;

        match start_res {
            Ok(Ok(())) => (),
            Ok(Err(err)) => {
                debug!("Failed to start handling connection from {addr}: {err}");
            }
            Err(_) => {
                warn!(
                    "Terminating connection from {addr}: WebSocket handshake timed out after {handshake_timeout:?}"
                );
            }
        }
    }

    async fn start_connection(
        context: &RouterContext<S>,
        addr: SocketAddr,
        stream: MaybeTlsStream<TcpStream>,
        guard: ConnectionGuard,
    ) -> Result<()> {
        debug!("Incoming TCP connection from {addr}");
        let acceptor = context
            .router()
            .acceptor_factory
            .lock()
            .await
            .new_acceptor();
        let acceptance = acceptor.accept(context, stream).await?;
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
            Some(guard),
        );
        Ok(())
    }

    fn start_connection_over_stream(
        context: &RouterContext<S>,
        stream: Box<dyn MessageStream>,
        guard: Option<ConnectionGuard>,
    ) -> Uuid {
        let is_network_connection = guard.is_some();
        let connection = Connection::new(guard);
        let uuid = connection.uuid();
        info!(
            "Created connection {uuid} over {}",
            stream.message_stream_type()
        );

        let (rate_limiter, idle_timeout) = if is_network_connection {
            let limits = &context.router().config.limits;
            (
                Some(TokenBucketRateLimiter::new(
                    limits.rate_limit_burst,
                    limits.rate_limit_refill_per_sec,
                )),
                limits.idle_timeout,
            )
        } else {
            (None, None)
        };

        let service = Service::new(uuid.to_string(), stream, rate_limiter, idle_timeout);

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
            Some(session) => session.session.close(CloseReason::Killed).await?,
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
        let (router_to_peer_tx, router_to_peer_rx) = mpsc::channel(4096);
        let (peer_to_router_tx, peer_to_router_rx) = mpsc::channel(4096);
        let router_stream = DirectMessageStream::new(router_to_peer_tx, peer_to_router_rx);
        let peer_stream = DirectMessageStream::new(peer_to_router_tx, router_to_peer_rx);
        let uuid = Self::start_connection_over_stream(context, Box::new(router_stream), None);
        DirectConnection {
            uuid,
            stream: Box::new(peer_stream),
        }
    }
}
