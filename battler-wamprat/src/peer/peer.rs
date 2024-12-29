use std::{
    fmt::Debug,
    sync::Arc,
    time::Duration,
};

use anyhow::{
    Error,
    Result,
};
use battler_wamp::{
    core::{
        error::ChannelTransmittableError,
        uri::Uri,
    },
    message::message::Message,
    peer::{
        Event,
        Invocation,
        PeerNotConnectedError,
        RpcCall,
        RpcResult,
    },
    router::RouterHandle,
};
use log::error;
use tokio::{
    sync::{
        broadcast::{
            self,
            error::SendError,
        },
        RwLock,
    },
    task::JoinHandle,
};

use crate::{
    peer::error::{
        PeerConnectionError,
        ProcedureRegistrationError,
    },
    procedure::Procedure,
};

/// A preregistered procedure that will be registered on every new connection to a router.
pub(crate) struct PreregisteredProcedure {
    pub(crate) procedure: Arc<Box<dyn Procedure>>,
    pub(crate) ignore_registration_error: bool,
}

/// The type of connection a [`Peer`] should continually establish with a router.
pub enum PeerConnectionType {
    /// A remote connection to some URI.
    Remote(String),
    /// A direct connection with a [`Router`][`battler_wamp::router::Router`] running in the same
    /// process.
    Direct(Arc<RouterHandle>),
}

impl Debug for PeerConnectionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Remote(uri) => write!(f, "remote({})", uri),
            Self::Direct(_) => write!(f, "direct"),
        }
    }
}

/// Configuration for a [`Peer`]'s connection to a router.
pub struct PeerConnectionConfig {
    /// The type of connection.
    pub connection_type: PeerConnectionType,
    /// The maximum consecutive connection failures to tolerate before giving up.
    pub max_consecutive_failures: u32,
    /// The delay between connect attempts.
    pub reconnect_delay: Duration,
}

impl PeerConnectionConfig {
    /// Creates a new config over the given connection type.
    pub fn new(connection_type: PeerConnectionType) -> Self {
        Self {
            connection_type,
            max_consecutive_failures: 1,
            reconnect_delay: Duration::ZERO,
        }
    }
}

fn retryable_error(err: &Error) -> bool {
    err.is::<PeerNotConnectedError>() || err.is::<SendError<Message>>()
}

async fn repeat_while_retryable<F, T>(f: F) -> Result<T>
where
    F: AsyncFn() -> Result<T>,
{
    loop {
        let result = f().await;
        if let Err(err) = &result {
            if retryable_error(err) {
                continue;
            }
        }
        return result;
    }
}

/// A handle to an asynchronously-running [`Peer`].
///
/// The peer's ownership is transferred away when it starts. This handle allows interaction with the
/// peer as it is running asynchronously.
pub struct PeerHandle<S> {
    peer: Arc<battler_wamp::peer::Peer<S>>,

    start_handle: JoinHandle<()>,
    cancel_tx: broadcast::Sender<()>,
    error_rx: broadcast::Receiver<ChannelTransmittableError>,

    peer_state: Arc<RwLock<PeerState>>,
    session_established_rx: broadcast::Receiver<()>,
    session_ready_rx: broadcast::Receiver<()>,
}

impl<S> PeerHandle<S>
where
    S: Send + 'static,
{
    /// Joins the task running the service.
    pub async fn join(self) -> Result<()> {
        self.start_handle.await.map_err(Error::new)
    }

    /// Cancels the router.
    ///
    /// Cancellation is asynchronous. Use [`Self::join`] to wait for the peer to stop.
    pub fn cancel(&self) -> Result<()> {
        self.cancel_tx.send(()).map(|_| ()).map_err(Error::new)
    }

    /// The error receiver channel.
    ///
    /// Only errors that are fatal to peer are reported here, which means the peer is no longer
    /// running.
    pub fn error_rx(&self) -> broadcast::Receiver<ChannelTransmittableError> {
        self.error_rx.resubscribe()
    }

    async fn wait_until_session_established(&self) -> Result<()> {
        // Subscribe ahead of checking if we are currently established so that we don't miss an
        // update.
        let mut session_established_rx = self.session_established_rx.resubscribe();
        if self.peer_state.read().await.established() {
            return Ok(());
        }
        let mut error_rx = self.error_rx.resubscribe();
        tokio::select! {
            _ = session_established_rx.recv() => Ok(()),
            err = error_rx.recv() => match err {
                Ok(err) => Err(err.into_error()),
                Err(err) => Err(err.into()),
            },
        }
    }

    /// Waits until the peer is known to be in a ready state.
    ///
    /// A peer is "ready" when all of its resources (procedures, subscriptions) have been registered
    /// on the connected router. A peer will continually attempt to move itself back to this "ready"
    /// state when it disconnects from the router. Thus, this method returning `true` only means the
    /// peer was known to be ready at the time of returning the value. The peer may become "unready"
    /// by disconnecting immediately after.
    pub async fn wait_until_ready(&self) -> Result<()> {
        // Subscribe ahead of checking if we are currently ready so that we don't miss an update.
        let mut session_ready_rx = self.session_ready_rx.resubscribe();
        if self.peer_state.read().await.ready() {
            return Ok(());
        }
        let mut error_rx = self.error_rx.resubscribe();
        tokio::select! {
            _ = session_ready_rx.recv() => Ok(()),
            err = error_rx.recv() => match err {
                Ok(err) => Err(err.into_error()),
                Err(err) => Err(err.into()),
            },
        }
    }

    /// Publishes an event to a topic, without type checking.
    pub async fn publish_unchecked(&self, topic: Uri, event: Event) -> Result<()> {
        self.wait_until_session_established().await?;
        repeat_while_retryable(async || self.peer.publish(topic.clone(), event.clone()).await).await
    }

    /// Publishes an event to a topic.
    pub async fn publish<Payload>(&self, topic: Uri, payload: Payload) -> Result<()>
    where
        Payload: battler_wamprat_schema::WampApplicationMessage,
    {
        let (arguments, arguments_keyword) = payload.wamp_serialize_application_message()?;
        self.publish_unchecked(
            topic,
            Event {
                arguments,
                arguments_keyword,
            },
        )
        .await
    }

    /// Calls a procedure, without type checking.
    pub async fn call_unchecked(&self, procedure: Uri, rpc_call: RpcCall) -> Result<RpcResult> {
        self.wait_until_session_established().await?;
        repeat_while_retryable(async || self.peer.call(procedure.clone(), rpc_call.clone()).await)
            .await
    }

    /// Calls a procedure.
    pub async fn call<Input, Output>(&self, procedure: Uri, input: Input) -> Result<Output>
    where
        Input: battler_wamprat_schema::WampApplicationMessage,
        Output: battler_wamprat_schema::WampApplicationMessage,
    {
        let (arguments, arguments_keyword) = input.wamp_serialize_application_message()?;
        let result = self
            .call_unchecked(
                procedure,
                RpcCall {
                    arguments,
                    arguments_keyword,
                },
            )
            .await?;
        let output = Output::wamp_deserialize_application_message(
            result.arguments,
            result.arguments_keyword,
        )?;
        Ok(output)
    }
}

#[derive(Clone)]
enum PeerState {
    Disconnected,
    Established,
    Ready,
}

impl PeerState {
    fn established(&self) -> bool {
        match self {
            Self::Disconnected => false,
            _ => true,
        }
    }

    fn ready(&self) -> bool {
        match self {
            Self::Ready => true,
            _ => false,
        }
    }
}

/// A WAMP peer (a.k.a., client) that connects to a WAMP router, establishes sessions in a realm,
/// and interacts with resources in the realm.
///
/// This type is a wrapper around [`battler_wamp::peer::Peer`], extending it to provide automatic
/// reconnection and reregistration abilities. If a session is dropped, the peer will attempt to
/// re-establish the session in the background.
///
/// As such, this type operates similarly to [`battler_wamp::router::Router`]: its ownership is
/// owned by a background task, so users can only operate on the router using the returned
/// [`PeerHandle`].
pub struct Peer<S> {
    peer: Arc<battler_wamp::peer::Peer<S>>,
    connection_config: PeerConnectionConfig,
    realm: Uri,
    procedures: ahash::HashMap<Uri, PreregisteredProcedure>,

    peer_state: Arc<RwLock<PeerState>>,
    session_established_tx: broadcast::Sender<()>,
    session_established_rx: broadcast::Receiver<()>,
    session_ready_tx: broadcast::Sender<()>,
    session_ready_rx: broadcast::Receiver<()>,
}

impl<S> Peer<S>
where
    S: Send + 'static,
{
    pub(crate) fn new(
        peer: battler_wamp::peer::Peer<S>,
        connection_config: PeerConnectionConfig,
        realm: Uri,
        procedures: impl Iterator<Item = (Uri, PreregisteredProcedure)>,
    ) -> Self {
        let (session_established_tx, session_established_rx) = broadcast::channel(16);
        let (session_ready_tx, session_ready_rx) = broadcast::channel(16);

        Self {
            peer: Arc::new(peer),
            connection_config,
            realm,
            procedures: procedures.collect(),
            peer_state: Arc::new(RwLock::new(PeerState::Disconnected)),
            session_established_tx,
            session_established_rx,
            session_ready_tx,
            session_ready_rx,
        }
    }

    /// Starts the peer asynchronously.
    ///
    /// The returned handle should be used to interact with the peer as it runs.
    pub fn start(self) -> PeerHandle<S> {
        let peer = self.peer.clone();
        let (cancel_tx, cancel_rx) = broadcast::channel(16);
        let (error_tx, error_rx) = broadcast::channel(16);
        let peer_state = self.peer_state.clone();
        let session_established_rx = self.session_established_rx.resubscribe();
        let session_ready_rx = self.session_ready_rx.resubscribe();
        let start_handle = tokio::spawn(self.run(cancel_rx, error_tx));
        PeerHandle {
            peer,
            start_handle,
            cancel_tx,
            error_rx,
            peer_state,
            session_established_rx,
            session_ready_rx,
        }
    }

    async fn run(
        self,
        mut cancel_rx: broadcast::Receiver<()>,
        error_tx: broadcast::Sender<ChannelTransmittableError>,
    ) {
        loop {
            match self.peer_loop_with_errors(&mut cancel_rx).await {
                Ok(done) => {
                    if done {
                        break;
                    }
                }
                Err(err) => {
                    if let Err(err) = error_tx.send(err.into()) {
                        error!("Failed to send peer error over channel for external communication: {err}");
                    }
                }
            }
        }
    }

    async fn peer_loop_with_errors(&self, cancel_rx: &mut broadcast::Receiver<()>) -> Result<bool> {
        self.reconnect_and_restore().await?;
        let mut session_finished_rx = self.peer.session_finished_rx();
        loop {
            tokio::select! {
                _ = session_finished_rx.recv() => {
                    break
                }
                _ = cancel_rx.recv() => {
                    self.peer.leave_realm().await?;
                    self.peer.disconnect().await?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    async fn reconnect_and_restore(&self) -> Result<()> {
        self.reconnect_loop().await?;
        self.restore_session_state().await?;
        Ok(())
    }

    async fn reconnect_loop(&self) -> Result<()> {
        let mut failures = 0;
        while let Err(err) = self.connect().await {
            error!(
                "Failed to connect to router (connection_type = {:?}): {err}",
                self.connection_config.connection_type
            );
            failures += 1;
            if failures > self.connection_config.max_consecutive_failures {
                return Err(PeerConnectionError::new(format!(
                    "failed to connect to router after {failures} attempts"
                ))
                .into());
            }
            tokio::time::sleep(self.connection_config.reconnect_delay).await;
        }
        Ok(())
    }

    async fn connect(&self) -> Result<()> {
        *self.peer_state.write().await = PeerState::Disconnected;

        match &self.connection_config.connection_type {
            PeerConnectionType::Remote(uri) => self.peer.connect(&uri).await,
            PeerConnectionType::Direct(router_handle) => {
                self.peer
                    .direct_connect(router_handle.direct_connect().stream())
                    .await
            }
        }
    }

    async fn invocation_loop(
        uri: Uri,
        procedure: Arc<Box<dyn Procedure>>,
        mut invocation_rx: broadcast::Receiver<Invocation>,
    ) {
        while let Ok(invocation) = invocation_rx.recv().await {
            let id = invocation.id();
            if let Err(err) = procedure.invoke(invocation).await {
                error!("Procedure invocation {id} of {uri} failed: {err}");
            }
        }
    }

    async fn restore_session_state(&self) -> Result<()> {
        // Rejoin the realm.
        self.peer.join_realm(self.realm.as_ref()).await?;

        *self.peer_state.write().await = PeerState::Established;
        self.session_established_tx.send(()).ok();

        // TODO: Resubscribe.

        // Restart all procedure handlers.
        for (uri, procedure) in &self.procedures {
            let invocation_rx = match self.peer.register(uri.clone()).await {
                Ok(procedure) => procedure.invocation_rx,
                Err(err) => {
                    if procedure.ignore_registration_error {
                        continue;
                    } else {
                        return Err(err.context(ProcedureRegistrationError::new(format!(
                            "failed to register procedure {uri}"
                        ))));
                    }
                }
            };
            tokio::spawn(Self::invocation_loop(
                uri.clone(),
                procedure.procedure.clone(),
                invocation_rx,
            ));
        }

        *self.peer_state.write().await = PeerState::Ready;
        self.session_ready_tx.send(()).ok();

        Ok(())
    }
}
