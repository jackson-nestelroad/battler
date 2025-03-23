//! # battler-wamprat-schema
//! **battler-wamprat-schema** is a supplemental crate for [`battler-wamprat`](https://crates.io/crates/battler-wamprat). It provides a procedural macro for generating consumer and producer peer objects for strongly-typed procedures and pub/sub topics.
//!
//! ## What is WAMP?
//!
//! **WAMP** is an open standard, routed protocol that provides two messaging patterns: Publish &
//! Subscribe and routed Remote Procedure Calls. It is intended to connect application components in
//! distributed applications. WAMP uses WebSocket as its default transport, but it can be
//! transmitted via any other protocol that allows for ordered, reliable, bi-directional, and
//! message-oriented communications.
//!
//! ## Background
//!
//! **battler-wamprat** is a Rust library and framework for peers communicating over the **Web
//! Application Message Protocol** (WAMP).
//!
//! The library is built on [`battler-wamp`](https://crates.io/crates/battler-wamp) to provide more complex functionality:
//!
//! 1. Automatic reconnection and re-registration of procedures and subscriptions when a session is
//!    dropped.
//! 1. Strongly-typed procedure handling, procedure calls, event publication, and subscription event
//!    handling using built-in serialization and deserialization.
//!
//! The library uses [`tokio`](https://tokio.rs) as its asynchronous runtime, and is ready for use on top of WebSocket streams.
//!
//! ## Schemas
//!
//! The `battler-wamprat-schema` crate works by generating code around
//! [`battler_wamprat::peer::Peer`] objects based on a schema.
//!
//! A **schema** is a collection of procedures and pub/sub topics that are logically connected
//! by application logic. A schema can be consumed by a **consumer** (a.k.a., a caller and
//! subscriber) and produced by a **producer** (a.k.a., a callee and publisher).
//!
//! Both consumers and producers are peers communicating via a WAMP router. When defining a schema,
//! the code for producer and consumer peers are automatically generated around the
//! [`battler_wamprat::peer::Peer`] object. Thus, peer objects can be entirely constructed by
//! `battler_wamprat_schema`, while all underlying functionality is provided by `battler_wamprat`.
//!
//! ## Usage
//!
//! A schema is defined with an enum type using the [`WampSchema`] procedural macro. You simply
//! attach different types (e.g., input, output, error, event, etc.) to each enum variant to
//! generate the strongly-typed peer methods.
//!
//! Note that schemas are attached to a **single realm**, so the connection logic is simplified.
//! Each peer will stay connected to the peer until is is manually canceled.
//!
//! After defining the schema, producers and consumers can be created from the schema enum.
//! Consumers generate a wrapper around [`battler_wamprat::peer::Peer`] directly, while producers
//! generate a wrapper around [`battler_wamprat::peer::PeerBuilder`] for registering procedure
//! handlers.
//!
//! To start a peer, you only need to provide preliminary information in the [`PeerConfig`], such as
//! which router to connect to and supported authentication methods.
//!
//! Below is a detailed example of a consumer and producer peer interacting through a router.
//!
//! ```
//! use std::time::Duration;
//!
//! use anyhow::Result;
//! use battler_wamp::{
//!     core::uri::Uri,
//!     peer::{
//!         WebSocketPeer,
//!         new_web_socket_peer,
//!     },
//!     router::{
//!         EmptyPubSubPolicies,
//!         EmptyRpcPolicies,
//!         RealmAuthenticationConfig,
//!         RealmConfig,
//!         RouterConfig,
//!         RouterHandle,
//!         new_web_socket_router,
//!     },
//! };
//! use battler_wamp_values::{
//!     WampDictionary,
//!     WampList,
//! };
//! use battler_wamprat::{
//!     peer::{
//!         CallOptions,
//!         PeerConnectionConfig,
//!         PeerConnectionType,
//!     },
//!     procedure::{
//!         Invocation,
//!         TypedProcedure,
//!     },
//!     subscription::{
//!         TypedPatternMatchedSubscription,
//!         TypedSubscription,
//!     },
//! };
//! use battler_wamprat_error::WampError;
//! use battler_wamprat_message::WampApplicationMessage;
//! use battler_wamprat_schema::{
//!     PeerConfig,
//!     WampSchema,
//!     WampSchemaError,
//! };
//! use battler_wamprat_uri::WampUriMatcher;
//! use thiserror::Error;
//! use tokio::{
//!     select,
//!     sync::broadcast::{
//!         self,
//!         error::{
//!             RecvError,
//!             TryRecvError,
//!         },
//!     },
//!     task::JoinHandle,
//! };
//!
//! #[derive(Debug, WampList)]
//! struct OneNumber(u64);
//!
//! #[derive(Debug, WampList)]
//! struct TwoNumbers(u64, u64);
//!
//! #[derive(Debug, WampApplicationMessage)]
//! struct Input(#[arguments] TwoNumbers);
//!
//! #[derive(Debug, WampApplicationMessage)]
//! struct Output(#[arguments] OneNumber);
//!
//! #[derive(Debug, Error, WampError)]
//! enum DivideError {
//!     #[error("cannot divide by 0")]
//!     #[uri("com.battler.error.divide_by_zero")]
//!     DivideByZero,
//! }
//!
//! #[derive(Debug, Clone, WampApplicationMessage)]
//! struct Ping;
//!
//! #[derive(Debug, Clone, WampDictionary)]
//! struct Message {
//!     author: String,
//!     content: String,
//! }
//!
//! #[derive(Debug, WampApplicationMessage)]
//! struct MessageEvent(#[arguments_keyword] Message);
//!
//! #[derive(Debug, WampUriMatcher)]
//! #[uri("com.battler.message.{version}.{channel}")]
//! struct MessagePattern {
//!     version: u64,
//!     channel: String,
//! }
//!
//! #[derive(WampSchema)]
//! #[realm("com.battler_wamprat_schema.realm.example")]
//! #[allow(unused)]
//! enum Example {
//!     #[rpc(uri = "com.battler.add", input = Input, output = Output)]
//!     Add,
//!     #[rpc(uri = "com.battler.divide", input = Input, output = Output, error = DivideError)]
//!     Divide,
//!     #[pubsub(uri = "com.battler.ping", event = Ping)]
//!     Ping,
//!     #[pubsub(pattern = MessagePattern, event = MessageEvent)]
//!     Message,
//! }
//!
//! async fn start_router() -> Result<(RouterHandle, JoinHandle<()>)> {
//!     let mut config = RouterConfig::default();
//!     config.realms.push(RealmConfig {
//!         name: "example".to_owned(),
//!         uri: Uri::try_from("com.battler_wamprat_schema.realm.example")?,
//!         authentication: RealmAuthenticationConfig::default(),
//!     });
//!     let router = new_web_socket_router(
//!         config,
//!         Box::new(EmptyPubSubPolicies::default()),
//!         Box::new(EmptyRpcPolicies::default()),
//!     )?;
//!     router.start().await
//! }
//!
//! fn create_peer(name: &str) -> Result<WebSocketPeer> {
//!     let mut config = battler_wamp::peer::PeerConfig::default();
//!     config.name = name.to_owned();
//!     new_web_socket_peer(config)
//! }
//!
//! async fn run_producer(
//!     router_handle: RouterHandle,
//!     producer_ready_tx: broadcast::Sender<()>,
//!     mut done_rx: broadcast::Receiver<()>,
//! ) {
//!     struct Adder;
//!     impl AddProcedure for Adder {}
//!
//!     impl TypedProcedure for Adder {
//!         type Input = Input;
//!         type Output = Output;
//!         type Error = anyhow::Error;
//!
//!         async fn invoke(
//!             &self,
//!             _: Invocation,
//!             input: Self::Input,
//!         ) -> Result<Self::Output, Self::Error> {
//!             Ok(Output(OneNumber(input.0.0 + input.0.1)))
//!         }
//!     }
//!
//!     struct Divider;
//!     impl DivideProcedure for Divider {}
//!
//!     impl TypedProcedure for Divider {
//!         type Input = Input;
//!         type Output = Output;
//!         type Error = DivideError;
//!
//!         async fn invoke(
//!             &self,
//!             _: Invocation,
//!             input: Self::Input,
//!         ) -> Result<Self::Output, Self::Error> {
//!             if input.0.1 == 0 {
//!                 Err(DivideError::DivideByZero)
//!             } else {
//!                 Ok(Output(OneNumber(input.0.0 / input.0.1)))
//!             }
//!         }
//!     }
//!
//!     let mut producer_builder = Example::producer_builder(PeerConfig {
//!         connection: PeerConnectionConfig::new(PeerConnectionType::Remote(format!(
//!             "ws://{}",
//!             router_handle.local_addr()
//!         ))),
//!         auth_methods: Vec::default(),
//!     });
//!     producer_builder.register_add(Adder).unwrap();
//!     producer_builder.register_divide(Divider).unwrap();
//!     let producer = producer_builder
//!         .start(create_peer("producer").unwrap())
//!         .unwrap();
//!     producer.wait_until_ready().await.unwrap();
//!
//!     producer_ready_tx.send(()).unwrap();
//!
//!     loop {
//!         select! {
//!             _ = done_rx.recv() => break,
//!             _ = tokio::time::sleep(Duration::from_secs(1)) => {
//!                 producer.publish_ping(Ping).await.unwrap();
//!                 producer.publish_message(
//!                     MessagePattern {
//!                         version: 1,
//!                         channel: "main".to_owned(),
//!                     },
//!                     MessageEvent(Message {
//!                         author: "user".to_owned(),
//!                         content: "foo".to_owned(),
//!                     })
//!                 )
//!                 .await.unwrap();
//!             }
//!         }
//!     }
//!
//!     producer.stop().await.unwrap();
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let (router_handle, _) = start_router().await.unwrap();
//!
//!     let (producer_ready_tx, mut producer_ready_rx) = broadcast::channel(1);
//!     let (done_tx, done_rx) = broadcast::channel(1);
//!
//!     let producer_join_handle = tokio::spawn(run_producer(
//!         router_handle.clone(),
//!         producer_ready_tx,
//!         done_rx,
//!     ));
//!
//!     // Wait for producer to be ready to serve procedure calls.
//!     producer_ready_rx.recv().await.unwrap();
//!
//!     let consumer = Example::consumer(
//!         PeerConfig {
//!             connection: PeerConnectionConfig::new(PeerConnectionType::Remote(format!(
//!                 "ws://{}",
//!                 router_handle.local_addr()
//!             ))),
//!             auth_methods: Vec::default(),
//!         },
//!         create_peer("consumer").unwrap(),
//!     )
//!     .unwrap();
//!     consumer.wait_until_ready().await.unwrap();
//!
//!     assert_matches::assert_matches!(
//!         consumer
//!             .add(Input(TwoNumbers(36345, 88818)), CallOptions::default())
//!             .await,
//!         Ok(rpc) => {
//!             assert_matches::assert_matches!(rpc.result().await, Ok(Output(OneNumber(125163))));
//!         }
//!     );
//!
//!     assert_matches::assert_matches!(consumer.divide(Input(TwoNumbers(25, 2)), CallOptions::default()).await, Ok(rpc) => {
//!         assert_matches::assert_matches!(rpc.result_observing_error().await, Ok(Output(OneNumber(12))));
//!     });
//!
//!     assert_matches::assert_matches!(consumer.divide(Input(TwoNumbers(1, 0)), CallOptions::default()).await, Ok(rpc) => {
//!         assert_matches::assert_matches!(rpc.result_observing_error().await, Err(WampSchemaError::Known(DivideError::DivideByZero)));
//!     });
//!
//!     struct PingHandler {
//!         events_tx: broadcast::Sender<Ping>,
//!     }
//!     impl PingSubscription for PingHandler {}
//!
//!     impl TypedSubscription for PingHandler {
//!         type Event = Ping;
//!
//!         async fn handle_event(&self, event: Self::Event) {
//!             self.events_tx.send(event).unwrap();
//!         }
//!     }
//!
//!     let (events_tx, mut events_rx) = broadcast::channel(16);
//!     assert_matches::assert_matches!(
//!         consumer.subscribe_ping(PingHandler { events_tx }).await,
//!         Ok(())
//!     );
//!
//!     assert_matches::assert_matches!(events_rx.recv().await, Ok(Ping));
//!     assert_matches::assert_matches!(events_rx.recv().await, Ok(Ping));
//!     assert_matches::assert_matches!(events_rx.recv().await, Ok(Ping));
//!     assert_matches::assert_matches!(events_rx.try_recv(), Err(TryRecvError::Empty));
//!
//!     assert_matches::assert_matches!(consumer.unsubscribe_ping().await, Ok(()));
//!     assert_matches::assert_matches!(events_rx.recv().await, Err(RecvError::Closed));
//!
//!     struct MessageHandler {
//!         events_tx: broadcast::Sender<(Message, u64, String)>,
//!     }
//!     impl MessageSubscription for MessageHandler {}
//!
//!     impl TypedPatternMatchedSubscription for MessageHandler {
//!         type Event = MessageEvent;
//!         type Pattern = MessagePattern;
//!
//!         async fn handle_event(&self, event: Self::Event, topic: Self::Pattern) {
//!             self.events_tx
//!                 .send((event.0, topic.version, topic.channel))
//!                 .unwrap();
//!         }
//!     }
//!
//!     let (events_tx, mut events_rx) = broadcast::channel(16);
//!     assert_matches::assert_matches!(
//!         consumer
//!             .subscribe_message(MessageHandler { events_tx })
//!             .await,
//!         Ok(())
//!     );
//!     assert_matches::assert_matches!(events_rx.recv().await, Ok((message, version, channel)) => {
//!         assert_eq!(message.author, "user");
//!         assert_eq!(message.content, "foo");
//!         assert_eq!(version, 1);
//!         assert_eq!(channel, "main");
//!     });
//!
//!     // Clean up the consumer and producer.
//!     consumer.stop().await.unwrap();
//!     done_tx.send(()).unwrap();
//!     producer_join_handle.await.unwrap();
//! }
//! ```

use std::{
    fmt::{
        Debug,
        Display,
    },
    marker::PhantomData,
};

use battler_wamp::core::error::WampError;
pub use battler_wamprat_schema_proc_macro::WampSchema;

/// An error resulting from a call to a schema object.
///
/// Procedures may define error types that are expected to be generated by callees. However,
/// procedure calls can fail in many other ways not decided by the callee. In this case, several
/// other error types may be generated instead. This type reflects this error handling scenario;
/// some errors may be "known" and others may be "unknown."
///
/// Unknown errors are reported with [`anyhow::Error`] and can be further inspected by the client.
#[derive(Debug)]
pub enum WampSchemaError<E> {
    /// A known error occurred and is parsed ahead of time for the client.
    Known(E),
    /// An unknown error occurred.
    Unknown(anyhow::Error),
}

impl<E> WampSchemaError<E>
where
    E: Into<anyhow::Error>,
{
    /// Converts the error back into the generic form.
    pub fn any_err(self) -> anyhow::Error {
        match self {
            Self::Known(err) => err.into(),
            Self::Unknown(err) => err,
        }
    }
}

/// A wrapper around [`battler_wamprat::peer::TypedSimplePendingRpc`] for strongly-typed procedure
/// calls.
#[derive(Debug)]
pub struct SimplePendingRpc<T, E> {
    rpc: battler_wamprat::peer::TypedSimplePendingRpc<T>,
    _t: PhantomData<T>,
    _e: PhantomData<E>,
}

impl<T, E> SimplePendingRpc<T, E>
where
    T: battler_wamprat_message::WampApplicationMessage,
    E: TryFrom<WampError, Error = WampError> + Debug + Display + Send + Sync + 'static,
{
    /// Waits for the result of the procedure call, observing the error and attempting to parse it
    /// to the known type.
    pub async fn result_observing_error(self) -> Result<T, WampSchemaError<E>> {
        self.rpc
            .result()
            .await
            .map_err(|err| match err.downcast::<WampError>() {
                Ok(err) => match E::try_from(err) {
                    Ok(err) => WampSchemaError::Known(err),
                    Err(err) => WampSchemaError::Unknown(err.into()),
                },
                Err(err) => WampSchemaError::Unknown(err),
            })
    }
}

impl<T, E> SimplePendingRpc<T, E>
where
    T: battler_wamprat_message::WampApplicationMessage,
    E: Debug + Display + Send + Sync + 'static,
{
    /// Waits for the result of the procedure call.
    pub async fn result(self) -> Result<T, anyhow::Error> {
        self.rpc.result().await
    }

    /// Cancels the pending call.
    pub async fn cancel(&self) -> anyhow::Result<()> {
        self.rpc.cancel().await
    }

    /// Kills the pending call.
    ///
    /// The end error, or result, can still be read from [`Self::result`].
    pub async fn kill(&self) -> anyhow::Result<()> {
        self.rpc.kill().await
    }
}

impl<T, E> From<battler_wamprat::peer::TypedSimplePendingRpc<T>> for SimplePendingRpc<T, E> {
    fn from(value: battler_wamprat::peer::TypedSimplePendingRpc<T>) -> Self {
        Self {
            rpc: value,
            _t: PhantomData,
            _e: PhantomData,
        }
    }
}

/// A wrapper around [`battler_wamprat::peer::TypedProgressivePendingRpc`] for strongly-typed
/// procedure calls.
#[derive(Debug)]
pub struct ProgressivePendingRpc<T, E> {
    rpc: battler_wamprat::peer::TypedProgressivePendingRpc<T>,
    _t: PhantomData<T>,
    _e: PhantomData<E>,
}

impl<T, E> ProgressivePendingRpc<T, E>
where
    T: battler_wamprat_message::WampApplicationMessage,
    E: TryFrom<WampError, Error = WampError> + Debug + Display + Send + Sync + 'static,
{
    /// Waits for the result of the procedure call, observing the error and attempting to parse it
    /// to the known type.
    pub async fn next_result_observing_error(&mut self) -> Result<Option<T>, WampSchemaError<E>> {
        self.rpc
            .next_result()
            .await
            .map_err(|err| match err.downcast::<WampError>() {
                Ok(err) => match E::try_from(err) {
                    Ok(err) => WampSchemaError::Known(err),
                    Err(err) => WampSchemaError::Unknown(err.into()),
                },
                Err(err) => WampSchemaError::Unknown(err),
            })
    }
}

impl<T, E> ProgressivePendingRpc<T, E>
where
    T: battler_wamprat_message::WampApplicationMessage,
    E: Debug + Display + Send + Sync + 'static,
{
    /// Waits for the result of the procedure call.
    pub async fn next_result(&mut self) -> Result<Option<T>, anyhow::Error> {
        self.rpc.next_result().await
    }

    /// Cancels the pending call.
    pub async fn cancel(&mut self) -> anyhow::Result<()> {
        self.rpc.cancel().await
    }

    /// Kills the pending call.
    ///
    /// The end error, or result, can still be read from [`Self::next_result`].
    pub async fn kill(&mut self) -> anyhow::Result<()> {
        self.rpc.kill().await
    }
}

impl<T, E> From<battler_wamprat::peer::TypedProgressivePendingRpc<T>>
    for ProgressivePendingRpc<T, E>
{
    fn from(value: battler_wamprat::peer::TypedProgressivePendingRpc<T>) -> Self {
        Self {
            rpc: value,
            _t: PhantomData,
            _e: PhantomData,
        }
    }
}

/// Configuration for a peer connecting to a router.
pub struct PeerConfig {
    /// Connection configuration.
    pub connection: battler_wamprat::peer::PeerConnectionConfig,
    /// Supported authentication methods.
    pub auth_methods: Vec<battler_wamp::peer::SupportedAuthMethod>,
}
