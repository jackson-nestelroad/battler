use std::{
    fmt::Debug,
    marker::PhantomData,
    sync::Arc,
    time::Duration,
};

use ahash::HashMap;
use anyhow::{
    Error,
    Result,
};
use battler_wamp::{
    core::{
        error::{
            ChannelTransmittableError,
            WampError,
        },
        id::Id,
        invocation_policy::InvocationPolicy,
        match_style::MatchStyle,
        uri::{
            Uri,
            WildcardUri,
        },
    },
    peer::{
        Invocation,
        ProcedureMessage,
        ProcedureOptions,
        ProgressivePendingRpc,
        PublishedEvent,
        RpcCall,
        RpcResult,
        SimplePendingRpc,
        SupportedAuthMethod,
    },
    router::RouterHandle,
};
use battler_wamprat_message::WampApplicationMessage;
use futures_util::lock::Mutex;
use log::{
    error,
    warn,
};
use tokio::{
    sync::{
        RwLock,
        broadcast,
        mpsc,
    },
    task::JoinHandle,
};

use crate::{
    peer::{
        error::{
            PeerConnectionError,
            ProcedureRegistrationError,
        },
        subscriber::Subscriber,
    },
    procedure::Procedure,
    subscription::{
        TypedPatternMatchedSubscription,
        TypedSubscription,
    },
};

/// A preregistered procedure that will be registered on every new connection to a router.
pub(crate) struct PreregisteredProcedure {
    pub procedure: Arc<Box<dyn Procedure>>,
    pub ignore_registration_error: bool,
    pub match_style: Option<MatchStyle>,
    pub invocation_policy: InvocationPolicy,
}

/// The type of connection a [`Peer`] should continually establish with a router.
pub enum PeerConnectionType {
    /// A remote connection to some URI.
    Remote(String),
    /// A direct connection with a [`Router`][`battler_wamp::router::Router`] running in the same
    /// process.
    Direct(RouterHandle),
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
#[derive(Debug)]
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
            max_consecutive_failures: 3,
            reconnect_delay: Duration::from_secs(5),
        }
    }
}

fn retryable_error(err: &Error) -> bool {
    const RETRYABLE_URIS: [&str; 4] = [
        "wamp.error.canceled",
        "com.battler_wamp.peer_not_connected",
        "com.battler_wamp.send_error",
        "com.battler_wamp.recv_error",
    ];
    RETRYABLE_URIS.contains(&Into::<WampError>::into(err).reason().as_ref())
}

async fn repeat_while_retryable<F, T>(f: F) -> Result<T>
where
    F: AsyncFn() -> Result<T>,
{
    loop {
        let result = f().await;
        if let Err(err) = &result {
            if retryable_error(err) {
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        }
        return result;
    }
}

/// Options for calling a procedure.
#[derive(Debug, Default)]
pub struct CallOptions {
    pub timeout: Option<Duration>,
}

/// A wrapper around [`SimplePendingRpc`] for strongly-typed procedure calls.
#[derive(Debug)]
pub struct TypedSimplePendingRpc<T> {
    rpc: SimplePendingRpc,
    _t: PhantomData<T>,
}

impl<T> TypedSimplePendingRpc<T>
where
    T: WampApplicationMessage,
{
    fn new(rpc: SimplePendingRpc) -> Self {
        Self {
            rpc,
            _t: PhantomData,
        }
    }

    /// Waits for the result of the procedure call.
    pub async fn result(self) -> Result<T> {
        let result = self.rpc.result().await?;
        T::wamp_deserialize_application_message(result.arguments, result.arguments_keyword)
            .map_err(Error::new)
    }

    /// Cancels the pending call.
    pub async fn cancel(&self) -> Result<()> {
        self.rpc.cancel().await
    }

    /// Kills the pending call.
    ///
    /// The end error, or result, can still be read from [`Self::result`].
    pub async fn kill(&self) -> Result<()> {
        self.rpc.kill().await
    }
}

/// A wrapper around [`ProgressivePendingRpc`] for strongly-typed procedure calls.
#[derive(Debug)]
pub struct TypedProgressivePendingRpc<T> {
    rpc: ProgressivePendingRpc,
    _t: PhantomData<T>,
}

impl<T> TypedProgressivePendingRpc<T>
where
    T: WampApplicationMessage,
{
    fn new(rpc: ProgressivePendingRpc) -> Self {
        Self {
            rpc,
            _t: PhantomData,
        }
    }

    /// Waits for the result of the procedure call.
    pub async fn next_result(&mut self) -> Result<Option<T>> {
        let result = self.rpc.next_result().await?;
        match result {
            Some(result) => {
                T::wamp_deserialize_application_message(result.arguments, result.arguments_keyword)
                    .map(|val| Some(val))
                    .map_err(Error::new)
            }
            None => Ok(None),
        }
    }

    /// Cancels the pending call.
    pub async fn cancel(&mut self) -> Result<()> {
        self.rpc.cancel().await
    }

    /// Kills the pending call.
    ///
    /// The end error, or result, can still be read from [`Self::next_result`].
    pub async fn kill(&mut self) -> Result<()> {
        self.rpc.kill().await
    }
}

/// A handle to an asynchronously-running [`Peer`].
///
/// The peer's ownership is transferred away when it starts. This handle allows interaction with the
/// peer as it is running asynchronously.
pub struct PeerHandle<S> {
    peer: Arc<battler_wamp::peer::Peer<S>>,
    subscriber: Arc<Mutex<Subscriber<S>>>,

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
    /// Cancels the peer.
    ///
    /// Cancellation is asynchronous. Use the [`JoinHandle`] returned from [`Peer::start`] to wait
    /// for the peer to stop.
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

    /// The current session ID, as given by the router.
    ///
    /// Since a peer is reused across multiple router sessions, this ID is subject to change at any
    /// point.
    pub async fn current_session_id(&self) -> Option<Id> {
        self.peer.current_session_id().await
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
                Ok(err) => Err(err.into()),
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
                Ok(err) => Err(err.into()),
                Err(err) => Err(err.into()),
            },
        }
    }

    /// Publishes an event to a topic, without type checking.
    pub async fn publish_unchecked(&self, topic: Uri, event: PublishedEvent) -> Result<()> {
        self.wait_until_session_established().await?;
        let f = (|peer: Arc<battler_wamp::peer::Peer<S>>, topic: Uri, event: PublishedEvent| {
            async move || peer.publish(topic.clone(), event.clone()).await
        })(self.peer.clone(), topic, event);
        repeat_while_retryable(f).await
    }

    /// Publishes an event to a topic.
    pub async fn publish<Payload>(&self, topic: Uri, payload: Payload) -> Result<()>
    where
        Payload: battler_wamprat_message::WampApplicationMessage + 'static,
    {
        let (arguments, arguments_keyword) = payload.wamp_serialize_application_message()?;
        self.publish_unchecked(
            topic,
            PublishedEvent {
                arguments,
                arguments_keyword,
            },
        )
        .await
    }

    /// Subscribes to a topic.
    pub async fn subscribe<T, Event>(&self, topic: Uri, subscription: T) -> Result<()>
    where
        T: TypedSubscription<Event = Event> + 'static,
        Event: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
    {
        self.subscriber
            .lock()
            .await
            .subscribe(topic, subscription)
            .await
    }

    /// Subscribes to a topic.
    pub async fn subscribe_pattern_matched<T, Pattern, Event>(&self, subscription: T) -> Result<()>
    where
        T: TypedPatternMatchedSubscription<Pattern = Pattern, Event = Event> + 'static,
        Pattern: battler_wamprat_uri::WampUriMatcher + Send + Sync + 'static,
        Event: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
    {
        self.subscriber
            .lock()
            .await
            .subscribe_pattern_matched::<T, Pattern, Event>(subscription)
            .await
    }

    /// Unsubscribes from a topic.
    pub async fn unsubscribe(&self, topic: &WildcardUri) -> Result<()> {
        self.subscriber.lock().await.unsubscribe(topic).await
    }

    /// Calls a procedure, without type checking, and waits for its result.
    pub async fn call_and_wait_unchecked(
        &self,
        procedure: Uri,
        rpc_call: RpcCall,
    ) -> Result<RpcResult> {
        self.wait_until_session_established().await?;
        let f = (|peer: Arc<battler_wamp::peer::Peer<S>>, procedure: Uri, rpc_call: RpcCall| {
            async move || {
                peer.call_and_wait(procedure.clone(), rpc_call.clone())
                    .await
            }
        })(self.peer.clone(), procedure, rpc_call);
        repeat_while_retryable(f).await
    }

    /// Calls a procedure and waits for its result.
    pub async fn call_and_wait<Input, Output>(
        &self,
        procedure: Uri,
        input: Input,
        options: CallOptions,
    ) -> Result<Output>
    where
        Input: battler_wamprat_message::WampApplicationMessage + 'static,
        Output: battler_wamprat_message::WampApplicationMessage + 'static,
    {
        let (arguments, arguments_keyword) = input.wamp_serialize_application_message()?;
        let result = self
            .call_and_wait_unchecked(
                procedure,
                RpcCall {
                    arguments,
                    arguments_keyword,
                    timeout: options.timeout,
                },
            )
            .await?;
        let output = Output::wamp_deserialize_application_message(
            result.arguments,
            result.arguments_keyword,
        )?;
        Ok(output)
    }

    /// Calls a procedure, without type checking.
    pub async fn call_unchecked(
        &self,
        procedure: Uri,
        rpc_call: RpcCall,
    ) -> Result<SimplePendingRpc> {
        self.wait_until_session_established().await?;
        let f = (|peer: Arc<battler_wamp::peer::Peer<S>>, procedure: Uri, rpc_call: RpcCall| {
            async move || peer.call(procedure.clone(), rpc_call.clone()).await
        })(self.peer.clone(), procedure, rpc_call);
        repeat_while_retryable(f).await
    }

    /// Calls a procedure.
    pub async fn call<Input, Output>(
        &self,
        procedure: Uri,
        input: Input,
        options: CallOptions,
    ) -> Result<TypedSimplePendingRpc<Output>>
    where
        Input: battler_wamprat_message::WampApplicationMessage + 'static,
        Output: battler_wamprat_message::WampApplicationMessage + 'static,
    {
        let (arguments, arguments_keyword) = input.wamp_serialize_application_message()?;
        let rpc = self
            .call_unchecked(
                procedure,
                RpcCall {
                    arguments,
                    arguments_keyword,
                    timeout: options.timeout,
                },
            )
            .await?;
        Ok(TypedSimplePendingRpc::new(rpc))
    }

    /// Calls a procedure, expecting one or more progressive results, without type checking.
    pub async fn call_with_progress_unchecked(
        &self,
        procedure: Uri,
        rpc_call: RpcCall,
    ) -> Result<ProgressivePendingRpc> {
        self.wait_until_session_established().await?;
        let f = (|peer: Arc<battler_wamp::peer::Peer<S>>, procedure: Uri, rpc_call: RpcCall| {
            async move || {
                peer.call_with_progress(procedure.clone(), rpc_call.clone())
                    .await
            }
        })(self.peer.clone(), procedure, rpc_call);
        repeat_while_retryable(f).await
    }

    /// Calls a procedure, expecting one or more progressive results.
    pub async fn call_with_progress<Input, Output>(
        &self,
        procedure: Uri,
        input: Input,
        options: CallOptions,
    ) -> Result<TypedProgressivePendingRpc<Output>>
    where
        Input: battler_wamprat_message::WampApplicationMessage + 'static,
        Output: battler_wamprat_message::WampApplicationMessage + 'static,
    {
        let (arguments, arguments_keyword) = input.wamp_serialize_application_message()?;
        let rpc = self
            .call_with_progress_unchecked(
                procedure,
                RpcCall {
                    arguments,
                    arguments_keyword,
                    timeout: options.timeout,
                },
            )
            .await?;
        Ok(TypedProgressivePendingRpc::new(rpc))
    }
}

impl<S> Clone for PeerHandle<S>
where
    S: Send + 'static,
{
    fn clone(&self) -> Self {
        Self {
            peer: self.peer.clone(),
            subscriber: self.subscriber.clone(),
            cancel_tx: self.cancel_tx.clone(),
            error_rx: self.error_rx.resubscribe(),
            peer_state: self.peer_state.clone(),
            session_established_rx: self.session_established_rx.resubscribe(),
            session_ready_rx: self.session_ready_rx.resubscribe(),
        }
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
/// owned by a background task, so users can only operate on the peer using the returned
/// [`PeerHandle`].
pub struct Peer<S> {
    peer: Arc<battler_wamp::peer::Peer<S>>,
    connection_config: PeerConnectionConfig,
    realm: Uri,
    auth_methods: Vec<SupportedAuthMethod>,

    subscriber: Arc<Mutex<Subscriber<S>>>,
    procedures: ahash::HashMap<WildcardUri, PreregisteredProcedure>,

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
        auth_methods: impl Iterator<Item = SupportedAuthMethod>,
        procedures: impl Iterator<Item = (WildcardUri, PreregisteredProcedure)>,
    ) -> Self {
        let peer = Arc::new(peer);
        let (session_established_tx, session_established_rx) = broadcast::channel(16);
        let (session_ready_tx, session_ready_rx) = broadcast::channel(16);

        Self {
            peer: peer.clone(),
            connection_config,
            realm,
            auth_methods: auth_methods.collect(),
            subscriber: Arc::new(Mutex::new(Subscriber::new(peer))),
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
    pub fn start(self) -> (PeerHandle<S>, JoinHandle<()>) {
        let peer = self.peer.clone();
        let subscriber = self.subscriber.clone();
        let (cancel_tx, cancel_rx) = broadcast::channel(16);
        let (error_tx, error_rx) = broadcast::channel(16);
        let peer_state = self.peer_state.clone();
        let session_established_rx = self.session_established_rx.resubscribe();
        let session_ready_rx = self.session_ready_rx.resubscribe();
        let start_handle = tokio::spawn(self.run(cancel_rx, error_tx));
        (
            PeerHandle {
                peer,
                subscriber,
                cancel_tx,
                error_rx,
                peer_state,
                session_established_rx,
                session_ready_rx,
            },
            start_handle,
        )
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
                        error!(
                            "Failed to send peer error over channel for external communication: {err}"
                        );
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
                    if let Err(err) = self.peer.leave_realm().await {
                        warn!("Failed to leave realm when canceling peer: {err}");
                    }
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
            if failures >= self.connection_config.max_consecutive_failures {
                return Err(PeerConnectionError::new(format!(
                    "failed to connect to router after {failures} attempt(s)"
                ))
                .into());
            }
            tokio::time::sleep(self.connection_config.reconnect_delay).await;
        }
        Ok(())
    }

    async fn connect(&self) -> Result<()> {
        // Stop any ongoing connection, if it has not stopped automatically due to the stream being
        // closed.
        self.peer.disconnect().await?;
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

    async fn invocation(
        procedure: Arc<Box<dyn Procedure>>,
        uri: WildcardUri,
        invocation: Invocation,
        invocation_done_rx: mpsc::Sender<Id>,
    ) {
        let id = invocation.id();
        if let Err(err) = procedure.invoke(invocation).await {
            error!("Procedure invocation {id} of {uri} failed: {err}");
        }
        invocation_done_rx.send(id).await.ok();
    }

    async fn invocation_loop(
        uri: WildcardUri,
        procedure: Arc<Box<dyn Procedure>>,
        mut procedure_message_rx: broadcast::Receiver<ProcedureMessage>,
    ) {
        let (invocation_done_tx, mut invocation_done_rx) = mpsc::channel(16);
        let mut invocations = HashMap::default();
        loop {
            tokio::select! {
                message = procedure_message_rx.recv() => {
                    match message {
                        Ok(ProcedureMessage::Invocation(invocation)) => {
                            let id = invocation.id();
                            invocations.insert(id, tokio::spawn(Self::invocation(
                                procedure.clone(),
                                uri.clone(),
                                invocation,
                                invocation_done_tx.clone(),
                            )));
                        }
                        Ok(ProcedureMessage::Interrupt(interrupt)) => {
                            if let Some(handle) = invocations.remove(&interrupt.id()) {
                                handle.abort();
                            }
                        }
                        Err(_) => {
                            break;
                        }
                    }
                }
                id = invocation_done_rx.recv() => {
                    if let Some(id) = id {
                        invocations.remove(&id);
                    }
                }
            }
        }
    }

    async fn restore_session_state(&self) -> Result<()> {
        // Rejoin the realm.
        self.peer
            .join_realm_with_authentication(self.realm.as_ref(), &self.auth_methods)
            .await?;

        *self.peer_state.write().await = PeerState::Established;
        self.session_established_tx.send(()).ok();

        // Restore all subscriptions.
        self.subscriber.lock().await.restore_subscriptions().await?;

        // Restart all procedure handlers.
        for (uri, procedure) in &self.procedures {
            let procedure_message_rx = match self
                .peer
                .register_with_options(
                    uri.clone(),
                    ProcedureOptions {
                        match_style: procedure.match_style,
                        invocation_policy: procedure.invocation_policy,
                    },
                )
                .await
            {
                Ok(procedure) => procedure.procedure_message_rx,
                Err(err) => {
                    error!("Failed to register procedure {uri}: {err}");
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
                procedure_message_rx,
            ));
        }

        *self.peer_state.write().await = PeerState::Ready;
        self.session_ready_tx.send(()).ok();

        Ok(())
    }
}
