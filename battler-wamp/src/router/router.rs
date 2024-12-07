use std::net::{
    IpAddr,
    Ipv4Addr,
    SocketAddr,
};

use anyhow::{
    Error,
    Result,
};
use futures_util::lock::Mutex;
use log::{
    debug,
    error,
};
use tokio::{
    net::{
        TcpListener,
        TcpStream,
    },
    sync::broadcast,
    task::JoinHandle,
};
use tokio_tungstenite::MaybeTlsStream;

use crate::{
    core::{
        close::CloseReason,
        hash::HashSet,
        id::{
            IdAllocator,
            RandomIdAllocator,
        },
        roles::RouterRole,
        service::Service,
        uri::Uri,
    },
    router::{
        acceptor::acceptor::AcceptorFactory,
        app::pub_sub::PubSubPolicies,
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
    pub address: IpAddr,
    pub port: u16,
    pub agent: String,
    pub roles: HashSet<RouterRole>,
    pub serializers: HashSet<SerializerType>,
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

/// A handle to a running [`Router`].
///
/// The router's ownership is transferred away when it starts. This handle allows interaction with
/// the router as it is running asynchronously.
pub struct RouterHandle {
    start_handle: JoinHandle<()>,
    cancel_tx: broadcast::Sender<()>,
    local_addr: SocketAddr,
}

impl RouterHandle {
    /// Joins the router task, effectively waiting for the router to stop altogether.
    pub async fn join(self) -> Result<()> {
        self.start_handle.await.map_err(Error::new)
    }

    /// Cancels the router.
    ///
    /// Cancellation is asynchronous. Use [`Self::join`] to wait for the router to stop.
    pub fn cancel(&self) -> Result<()> {
        self.cancel_tx.send(()).map(|_| ()).map_err(Error::new)
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }
}

/// A WAMP router (a.k.a., server) that accepts incoming connections and handles sessions.
pub struct Router<S> {
    /// The router configuration when created.
    pub(crate) config: RouterConfig,

    pub(crate) pub_sub_policies: Mutex<Box<dyn PubSubPolicies<S>>>,

    pub(crate) realm_manager: RealmManager,

    /// The factory for acceptors.
    pub(crate) acceptor_factory: Mutex<Box<dyn AcceptorFactory<S>>>,

    /// The factory for transports.
    pub(crate) transport_factory: Mutex<Box<dyn TransportFactory<S>>>,

    // Allocator for global IDs.
    pub(crate) id_allocator: Box<dyn IdAllocator>,

    cancel_tx: broadcast::Sender<()>,
    end_tx: broadcast::Sender<()>,
    _end_rx: broadcast::Receiver<()>,
}

impl<S> Router<S> {
    pub(crate) fn end_rx(&self) -> broadcast::Receiver<()> {
        self.end_tx.subscribe()
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
        acceptor_factory: Box<dyn AcceptorFactory<S>>,
        transport_factory: Box<dyn TransportFactory<S>>,
    ) -> Result<Self> {
        let mut realm_manager = RealmManager::default();
        for realm_config in &config.realms {
            realm_manager.insert(Realm::new(realm_config.clone()));
        }
        let (cancel_tx, _) = broadcast::channel(1);
        let (end_tx, end_rx) = broadcast::channel(1);
        Ok(Self {
            config,
            pub_sub_policies: Mutex::new(pub_sub_policies),
            realm_manager,
            acceptor_factory: Mutex::new(acceptor_factory),
            transport_factory: Mutex::new(transport_factory),
            id_allocator: Box::new(RandomIdAllocator::default()),
            cancel_tx,
            end_tx,
            _end_rx: end_rx,
        })
    }

    /// Starts the router asynchronously.
    ///
    /// The returned handle can be used to interact with the router since its ownership is
    /// transferred away.
    pub async fn start(self) -> Result<RouterHandle, Error> {
        let addr = format!("{}:{}", self.config.address, self.config.port);
        let listener = TcpListener::bind(&addr).await?;
        let local_addr = listener.local_addr()?;

        let cancel_tx = self.cancel_tx.clone();
        let start_handle = tokio::spawn(self.handle_connections(listener));

        Ok(RouterHandle {
            start_handle,
            cancel_tx,
            local_addr,
        })
    }

    async fn handle_connections(self, listener: TcpListener) {
        let context = RouterContext::new(self);
        Self::connection_loop(&context, listener).await;
        Self::shut_down(&context).await;
        if let Err(err) = context.router().end_tx.send(()) {
            error!("Failed to write to end_tx channel after router connection loop ended: {err}");
        }
    }

    async fn connection_loop(context: &RouterContext<S>, listener: TcpListener) {
        let mut cancel_rx = context.router().cancel_tx.subscribe();

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

        let connection = Connection::new();
        let service = Service::new(connection.uuid().to_string(), transport, serializer);

        connection.start(context.clone(), service);

        Ok(())
    }

    async fn shut_down(context: &RouterContext<S>) {
        let realms = context
            .router()
            .realm_manager
            .lock()
            .await
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
        let mut realm_manager = context.router().realm_manager.lock().await;
        let realm = match realm_manager.remove(realm) {
            Some(realm) => realm,
            None => return Ok(()),
        };
        realm.shut_down(close_reason).await
    }
}
