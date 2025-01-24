//! # battler-wamprat
//! ## `battler-wamp` + **RaT (Reconnection and Typing)**
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
//! ## What is WAMP?
//!
//! **WAMP** is an open standard, routed protocol that provides two messaging patterns: Publish &
//! Subscribe and routed Remote Procedure Calls. It is intended to connect application components in
//! distributed applications. WAMP uses WebSocket as its default transport, but it can be
//! transmitted via any other protocol that allows for ordered, reliable, bi-directional, and
//! message-oriented communications.
//!
//! The WAMP protocol specification is described [here](https://wamp-proto.org/spec.html).
//!
//! ## Core Features
//!
//! ### Reconnection
//!
//! When a WAMP peer disconnects from a router, all of its owned resources are discarded. This
//! includes subscriptions and procedures. When the peer reconnects with the router, all resources
//! must be manually redefined with the router.
//!
//! The philosophy of `battler-wamprat` is that application logic should not need to worry about
//! this reregistration whatsoever. The peer keeps a record of all resources, so they are quickly
//! reestablished as soon as possible.
//!
//! ### Type Checking
//!
//! In general, user-provided types can be type checked using traits with derive macros for ease of
//! use.
//!
//! The [`battler_wamprat_message::WampApplicationMessage`] trait can be used to type check
//! application messages. This trait is used for pub/sub events, RPC calls, and RPC results.
//!
//! The [`battler_wamprat_uri::WampUriMatcher`] trait can be used to type check pattern-matched
//! URIs. This trait is only required when URI pattern matching is used.
//!
//! ## Usage
//!
//! A WAMP peer managed by `battler-wamprat` runs in an asynchronous task, which continually
//! establishes a connection to the configured WAMP router. On each new session, all known resources
//! (e.g., procedures and subscriptions) will be recreated, thereby resuming the previous session.
//!
//! A new peer can be built using a [`PeerBuilder`][`crate::peer::PeerBuilder`]. The
//! [`PeerConnectionConfig`][`crate::peer::PeerConnectionConfig`] describes what router to connect
//! to and how to handle reconnects. Procedures must be preregistered on the builder at this point,
//! so that they can be registered on the router as soon as a session is established.
//!
//! When it is time to build and start the peer, a [`battler_wamp::peer::Peer`] must be passed in.
//! This allows the underlying peer object to be configured however desired. Once the peer starts in
//! the background, it can be interacted with through the returned
//! [`PeerHandle`][`crate::peer::PeerHandle`]. The handle can be used for dynamic resources, (e.g.,
//! subscribing to a topic) and for one-off calls (e.g., publishing an event, calling a procedure).
//!
//! The [`PeerBuilder`][`crate::peer::PeerBuilder`] also returns a [`tokio::task::JoinHandle`] that
//! can be used to wait for the peer to be fully destroyed.
//!
//! See the examples below for all of these things in action.
//!
//! ## Examples
//!
//! ### Pub/Sub
//!
//! Peers can subscribe to topics that other peers can publish events to. When a peer reconnects to
//! a router, all of its previous subscriptions are restored.
//!
//! Subscriptions must be a type implementing one of the following traits:
//! * [`Subscription`][`crate::subscription::Subscription`] - For events without type checking.
//! * [`TypedSubscription`][`crate::subscription::TypedSubscription`] - For events with strict type
//!   checking.
//! * [`TypedPatternMatchedSubscription`][`crate::subscription::TypedPatternMatchedSubscription`] -
//!   For events with strict type checking and a pattern-matched URI.
//!
//! All of these traits provide methods for handling events matched by the subscription.
//!
//! #### Simple Example
//!
//! ```
//! use battler_wamp::{
//!     core::uri::{
//!         Uri,
//!         WildcardUri,
//!     },
//!     peer::{
//!         PeerConfig,
//!         ReceivedEvent,
//!         new_web_socket_peer,
//!     },
//!     router::{
//!         EmptyPubSubPolicies,
//!         EmptyRpcPolicies,
//!         RealmConfig,
//!         RouterConfig,
//!         RouterHandle,
//!         new_web_socket_router,
//!     },
//! };
//! use battler_wamp_values::WampList;
//! use battler_wamprat::{
//!     peer::{
//!         PeerBuilder,
//!         PeerConnectionType,
//!     },
//!     subscription::TypedSubscription,
//! };
//! use battler_wamprat_message::WampApplicationMessage;
//! use tokio::{
//!     sync::broadcast,
//!     task::JoinHandle,
//! };
//!
//! async fn start_router() -> anyhow::Result<(RouterHandle, JoinHandle<()>)> {
//!     let mut config = RouterConfig::default();
//!     config.realms.push(RealmConfig {
//!         name: "Realm".to_owned(),
//!         uri: Uri::try_from("com.battler_wamprat.realm").unwrap(),
//!     });
//!     let router = new_web_socket_router(
//!         config,
//!         Box::new(EmptyPubSubPolicies::default()),
//!         Box::new(EmptyRpcPolicies::default()),
//!     )?;
//!     router.start().await
//! }
//!
//! #[derive(WampList)]
//! struct PingEventArgs(String);
//!
//! #[derive(WampApplicationMessage)]
//! struct PingEvent(#[arguments] PingEventArgs);
//!
//! struct PingEventHandler {
//!     ping_tx: broadcast::Sender<String>,
//! }
//!
//! #[async_trait::async_trait]
//! impl TypedSubscription for PingEventHandler {
//!     type Event = PingEvent;
//!
//!     async fn handle_event(&self, event: Self::Event) {
//!         self.ping_tx.send(event.0.0).unwrap();
//!     }
//!
//!     async fn handle_invalid_event(&self, event: ReceivedEvent, error: anyhow::Error) {
//!         panic!("invalid event: {event:?}");
//!     }
//! }
//!
//! async fn publish_event(router_handle: RouterHandle) {
//!     let (publisher, _) = PeerBuilder::new(PeerConnectionType::Remote(format!(
//!         "ws://{}",
//!         router_handle.local_addr()
//!     )))
//!     .start(
//!         new_web_socket_peer(PeerConfig::default()).unwrap(),
//!         Uri::try_from("com.battler_wamprat.realm").unwrap(),
//!     );
//!     publisher.wait_until_ready().await.unwrap();
//!
//!     publisher
//!         .publish(
//!             Uri::try_from("com.battler_wamprat.ping").unwrap(),
//!             PingEvent(PingEventArgs("Hello, World!".to_owned())),
//!         )
//!         .await
//!         .unwrap();
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let (router_handle, _) = start_router().await.unwrap();
//!
//!     let (subscriber, _) = PeerBuilder::new(PeerConnectionType::Remote(format!(
//!         "ws://{}",
//!         router_handle.local_addr()
//!     )))
//!     .start(
//!         new_web_socket_peer(PeerConfig::default()).unwrap(),
//!         Uri::try_from("com.battler_wamprat.realm").unwrap(),
//!     );
//!     subscriber.wait_until_ready().await.unwrap();
//!
//!     // Subscribe.
//!     let (ping_tx, mut ping_rx) = broadcast::channel(16);
//!     subscriber
//!         .subscribe(
//!             Uri::try_from("com.battler_wamprat.ping").unwrap(),
//!             PingEventHandler { ping_tx },
//!         )
//!         .await
//!         .unwrap();
//!
//!     publish_event(router_handle.clone()).await;
//!
//!     // Wait for the event.
//!     assert_eq!(ping_rx.recv().await.unwrap(), "Hello, World!");
//!
//!     // Unsubscribe.
//!     subscriber
//!         .unsubscribe(&WildcardUri::try_from("com.battler_wamprat.ping").unwrap())
//!         .await
//!         .unwrap();
//! }
//! ```
//!
//! #### Pattern-Based Subscription
//!
//! ```
//! use battler_wamp::{
//!     core::uri::{
//!         Uri,
//!         WildcardUri,
//!     },
//!     peer::{
//!         PeerConfig,
//!         new_web_socket_peer,
//!     },
//!     router::{
//!         EmptyPubSubPolicies,
//!         EmptyRpcPolicies,
//!         RealmConfig,
//!         RouterConfig,
//!         RouterHandle,
//!         new_web_socket_router,
//!     },
//! };
//! use battler_wamp_values::WampList;
//! use battler_wamprat::{
//!     peer::{
//!         PeerBuilder,
//!         PeerConnectionType,
//!     },
//!     subscription::TypedPatternMatchedSubscription,
//! };
//! use battler_wamprat_message::WampApplicationMessage;
//! use battler_wamprat_uri::WampUriMatcher;
//! use tokio::{
//!     sync::broadcast,
//!     task::JoinHandle,
//! };
//!
//! async fn start_router() -> anyhow::Result<(RouterHandle, JoinHandle<()>)> {
//!     let mut config = RouterConfig::default();
//!     config.realms.push(RealmConfig {
//!         name: "Realm".to_owned(),
//!         uri: Uri::try_from("com.battler_wamprat.realm").unwrap(),
//!     });
//!     let router = new_web_socket_router(
//!         config,
//!         Box::new(EmptyPubSubPolicies::default()),
//!         Box::new(EmptyRpcPolicies::default()),
//!     )?;
//!     router.start().await
//! }
//!
//! #[derive(WampList)]
//! struct PingEventArgs(String);
//!
//! #[derive(WampApplicationMessage)]
//! struct PingEvent(#[arguments] PingEventArgs);
//!
//! #[derive(WampUriMatcher)]
//! #[uri("com.battler_wamprat.ping.v{version}")]
//! struct PingEventPattern {
//!     version: u64,
//! }
//!
//! struct PingEventHandler {
//!     ping_tx: broadcast::Sender<(String, u64)>,
//! }
//!
//! #[async_trait::async_trait]
//! impl TypedPatternMatchedSubscription for PingEventHandler {
//!     type Pattern = PingEventPattern;
//!     type Event = PingEvent;
//!
//!     async fn handle_event(&self, event: Self::Event, pattern: Self::Pattern) {
//!         self.ping_tx.send((event.0.0, pattern.version)).unwrap();
//!     }
//! }
//!
//! async fn publish_event(router_handle: RouterHandle) {
//!     let (publisher, _) = PeerBuilder::new(PeerConnectionType::Remote(format!(
//!         "ws://{}",
//!         router_handle.local_addr()
//!     )))
//!     .start(
//!         new_web_socket_peer(PeerConfig::default()).unwrap(),
//!         Uri::try_from("com.battler_wamprat.realm").unwrap(),
//!     );
//!     publisher.wait_until_ready().await.unwrap();
//!
//!     publisher
//!         .publish(
//!             Uri::try_from("com.battler_wamprat.ping.v1").unwrap(),
//!             PingEvent(PingEventArgs("foo".to_owned())),
//!         )
//!         .await
//!         .unwrap();
//!     publisher
//!         .publish(
//!             Uri::try_from("com.battler_wamprat.ping.invalid").unwrap(),
//!             PingEvent(PingEventArgs("bar".to_owned())),
//!         )
//!         .await
//!         .unwrap();
//!     publisher
//!         .publish(
//!             Uri::try_from("com.battler_wamprat.ping.v2").unwrap(),
//!             PingEvent(PingEventArgs("baz".to_owned())),
//!         )
//!         .await
//!         .unwrap();
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let (router_handle, _) = start_router().await.unwrap();
//!
//!     let (subscriber, _) = PeerBuilder::new(PeerConnectionType::Remote(format!(
//!         "ws://{}",
//!         router_handle.local_addr()
//!     )))
//!     .start(
//!         new_web_socket_peer(PeerConfig::default()).unwrap(),
//!         Uri::try_from("com.battler_wamprat.realm").unwrap(),
//!     );
//!     subscriber.wait_until_ready().await.unwrap();
//!
//!     // Subscribe.
//!     let (ping_tx, mut ping_rx) = broadcast::channel(16);
//!     subscriber
//!         .subscribe_pattern_matched(PingEventHandler { ping_tx })
//!         .await
//!         .unwrap();
//!
//!     publish_event(router_handle.clone()).await;
//!
//!     // Wait for events.
//!     assert_eq!(ping_rx.recv().await.unwrap(), ("foo".to_owned(), 1));
//!     assert_eq!(ping_rx.recv().await.unwrap(), ("baz".to_owned(), 2));
//!
//!     // Unsubscribe.
//!     subscriber
//!         .unsubscribe(&PingEventPattern::uri_for_router())
//!         .await
//!         .unwrap();
//! }
//! ```
//!
//! ### RPC
//!
//! #### Simple Example
//!
//! ```
//! use battler_wamp::{
//!     core::{
//!         error::WampError,
//!         uri::{
//!             Uri,
//!             WildcardUri,
//!         },
//!     },
//!     peer::{
//!         PeerConfig,
//!         new_web_socket_peer,
//!     },
//!     router::{
//!         EmptyPubSubPolicies,
//!         EmptyRpcPolicies,
//!         RealmConfig,
//!         RouterConfig,
//!         RouterHandle,
//!         new_web_socket_router,
//!     },
//! };
//! use battler_wamp_values::{
//!     Integer,
//!     WampList,
//! };
//! use battler_wamprat::{
//!     peer::{
//!         CallOptions,
//!         PeerBuilder,
//!         PeerConnectionType,
//!     },
//!     procedure::TypedProcedure,
//! };
//! use battler_wamprat_error::WampError;
//! use battler_wamprat_message::WampApplicationMessage;
//! use tokio::{
//!     sync::broadcast,
//!     task::JoinHandle,
//! };
//!
//! async fn start_router() -> anyhow::Result<(RouterHandle, JoinHandle<()>)> {
//!     let mut config = RouterConfig::default();
//!     config.realms.push(RealmConfig {
//!         name: "Realm".to_owned(),
//!         uri: Uri::try_from("com.battler_wamprat.realm").unwrap(),
//!     });
//!     let router = new_web_socket_router(
//!         config,
//!         Box::new(EmptyPubSubPolicies::default()),
//!         Box::new(EmptyRpcPolicies::default()),
//!     )?;
//!     router.start().await
//! }
//!
//! #[derive(WampList)]
//! struct DivideInputArgs(Integer, Integer);
//!
//! #[derive(WampApplicationMessage)]
//! struct DivideInput(#[arguments] DivideInputArgs);
//!
//! #[derive(Debug, PartialEq, WampList)]
//! struct DivideOutputArgs(Integer, Integer);
//!
//! #[derive(Debug, PartialEq, WampApplicationMessage)]
//! struct DivideOutput(#[arguments] DivideOutputArgs);
//!
//! #[derive(Debug, PartialEq, thiserror::Error, WampError)]
//! enum DivideError {
//!     #[error("cannot divide by 0")]
//!     #[uri("com.battler_wamprat.divide.error.divide_by_zero")]
//!     DivideByZero,
//! }
//!
//! struct DivideHandler;
//!
//! #[async_trait::async_trait]
//! impl TypedProcedure for DivideHandler {
//!     type Input = DivideInput;
//!     type Output = DivideOutput;
//!     type Error = DivideError;
//!
//!     async fn invoke(&self, input: Self::Input) -> Result<Self::Output, Self::Error> {
//!         if input.0.1 == 0 {
//!             return Err(DivideError::DivideByZero);
//!         }
//!         let q = input.0.0 / input.0.1;
//!         let r = input.0.0 % input.0.1;
//!         Ok(DivideOutput(DivideOutputArgs(q, r)))
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let (router_handle, _) = start_router().await.unwrap();
//!
//!     // Set up the peer that provides the procedure definition.
//!     let mut callee = PeerBuilder::new(PeerConnectionType::Remote(format!(
//!         "ws://{}",
//!         router_handle.local_addr()
//!     )));
//!     callee.add_procedure(
//!         Uri::try_from("com.battler_wamprat.divide").unwrap(),
//!         DivideHandler,
//!     );
//!     let (callee, _) = callee.start(
//!         new_web_socket_peer(PeerConfig::default()).unwrap(),
//!         Uri::try_from("com.battler_wamprat.realm").unwrap(),
//!     );
//!     callee.wait_until_ready().await.unwrap();
//!
//!     // Set up the caller.
//!     let (caller, _) = PeerBuilder::new(PeerConnectionType::Remote(format!(
//!         "ws://{}",
//!         router_handle.local_addr()
//!     )))
//!     .start(
//!         new_web_socket_peer(PeerConfig::default()).unwrap(),
//!         Uri::try_from("com.battler_wamprat.realm").unwrap(),
//!     );
//!     caller.wait_until_ready().await.unwrap();
//!
//!     // Call the procedure.
//!     assert_eq!(
//!         caller
//!             .call_and_wait::<DivideInput, DivideOutput>(
//!                 Uri::try_from("com.battler_wamprat.divide").unwrap(),
//!                 DivideInput(DivideInputArgs(65, 4)),
//!                 CallOptions::default(),
//!             )
//!             .await
//!             .unwrap(),
//!         DivideOutput(DivideOutputArgs(16, 1))
//!     );
//!     assert_eq!(
//!         TryInto::<DivideError>::try_into(
//!             caller
//!                 .call_and_wait::<DivideInput, DivideOutput>(
//!                     Uri::try_from("com.battler_wamprat.divide").unwrap(),
//!                     DivideInput(DivideInputArgs(2, 0)),
//!                     CallOptions::default(),
//!                 )
//!                 .await
//!                 .unwrap_err()
//!                 .downcast::<WampError>()
//!                 .unwrap()
//!         )
//!         .unwrap(),
//!         DivideError::DivideByZero
//!     );
//! }
//! ```
//!
//! #### Pattern-Based Registration
//!
//! TODO
pub mod error;
pub mod peer;
pub mod procedure;
pub mod subscription;
