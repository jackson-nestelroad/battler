//! # battler-wamp
//!
//! **battler-wamp** is an implementation of the **Web Application Message Protocol** (WAMP) for
//! Rust.
//!
//! The library implements the WAMP protocol for both routers and peers (a.k.a., servers and
//! clients).
//!
//! The library uses [`tokio`](https://tokio.rs) as its asynchronous runtime, and is ready for
//! use on top of WebSocket streams.
//!
//! For writing peers that desire strongly-typed messaging (including procedure calls and pub/sub
//! events), use [`battler-wamprat`](https://crates.io/crates/battler-wamprat).
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
//! ## Routers
//!
//! WAMP peers talk to one another by establishing a session on a shared realm through a shared
//! router.
//!
//! Spinning up a router with `battler-wamp` is incredibly easy. Configure the router through a
//! [`RouterConfig`][`crate::router::RouterConfig`] and construct a
//! [`Router`][`crate::router::Router`] object directly.
//!
//! If you are working with WebSocket connections, the
//! [`new_web_socket_router`][`crate::router::new_web_socket_router`] utility function sets up
//! the proper modules for convenience
//!
//! A router is a full-fledged server that manages resources and interactions between peers. Thus,
//! the router can function mostly autonomously after it is set up. The router runs in a background
//! task transparent to the caller. It can be interacted with through the returned
//! [`RouterHandle`][`crate::router::RouterHandle`]. The caller also receives a
//! [`tokio::task::JoinHandle`] that can be used to wait for the router to be fully destroyed.
//!
//! ### Router Example
//!
//! ```
//! use battler_wamp::{
//!     core::uri::Uri,
//!     router::{
//!         EmptyPubSubPolicies,
//!         EmptyRpcPolicies,
//!         RealmAuthenticationConfig,
//!         RealmConfig,
//!         RouterConfig,
//!         new_web_socket_router,
//!     },
//! };
//!
//! #[tokio::main]
//! async fn main() {
//!     let mut config = RouterConfig::default();
//!     config.port = 8080;
//!     config.realms.push(RealmConfig {
//!         name: "Test Realm".to_owned(),
//!         uri: Uri::try_from("com.battler_wamp.realm.test").unwrap(),
//!         authentication: RealmAuthenticationConfig::default(),
//!     });
//!
//!     // Create the router.
//!     //
//!     // Policy modules can be used to inject custom policies for resources created on the router.
//!     let router = new_web_socket_router(
//!         config,
//!         Box::new(EmptyPubSubPolicies::default()),
//!         Box::new(EmptyRpcPolicies::default()),
//!     )
//!     .unwrap();
//!
//!     // Start the router in a background task.
//!     let (router_handle, router_join_handle) = router.start().await.unwrap();
//!
//!     // Let the router run for as long as desired...
//!
//!     // Cancel and wait for the router to terminate.
//!     router_handle.cancel().unwrap();
//!     router_join_handle.await;
//! }
//! ```
//!
//! ## Peers
//!
//! WAMP peers are simply clients that interact with a WAMP router. Unlike routers, they are
//! directly controlled by callers, so peers are constructed and intended to be owned by
//! higher-level application code.
//!
//! Configure a peer using a [`PeerConfig`][`crate::peer::PeerConfig`] and construct a
//! [`Peer`][`crate::peer::Peer`] directly.
//!
//! If you are working with WebSocket connections, the
//! [`new_web_socket_peer`][`crate::peer::new_web_socket_peer`] utility function sets up the
//! proper modules for convenience.
//!
//! ### Connecting to a Realm
//!
//! ```
//! use battler_wamp::{
//!     core::{
//!         hash::HashMap,
//!         uri::Uri,
//!     },
//!     peer::{
//!         Peer,
//!         PeerConfig,
//!         WebSocketConfig,
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
//! use tokio::task::JoinHandle;
//!
//! async fn start_router() -> anyhow::Result<(RouterHandle, JoinHandle<()>)> {
//!     let mut config = RouterConfig::default();
//!     config.realms.push(RealmConfig {
//!         name: "Realm A".to_owned(),
//!         uri: Uri::try_from("com.battler_wamp.realm.a").unwrap(),
//!         authentication: RealmAuthenticationConfig::default(),
//!     });
//!     config.realms.push(RealmConfig {
//!         name: "Realm B".to_owned(),
//!         uri: Uri::try_from("com.battler_wamp.realm.b").unwrap(),
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
//! #[tokio::main]
//! async fn main() {
//!     let (router_handle, _) = start_router().await.unwrap();
//!
//!     let mut config = PeerConfig::default();
//!     config.web_socket = Some(WebSocketConfig {
//!         headers: HashMap::from_iter([(
//!             "X-WAMP-Framework".to_owned(),
//!             "battler-wamp".to_owned(),
//!         )]),
//!     });
//!
//!     // Create peer, connect to a router, and join a realm.
//!     let peer = new_web_socket_peer(config).unwrap();
//!     peer.connect(&format!("ws://{}", router_handle.local_addr()))
//!         .await
//!         .unwrap();
//!     peer.join_realm("com.battler_wamp.realm.a").await.unwrap();
//!
//!     // Leave the realm, and join a different one.
//!     peer.leave_realm().await.unwrap();
//!     peer.join_realm("com.battler_wamp.realm.b").await.unwrap();
//!
//!     // Disconnect from the router altogether.
//!     peer.disconnect().await.unwrap();
//! }
//! ```
//!
//! ### Pub/Sub
//!
//! Peers can subscribe to topics that other peers can publish events to.
//!
//! #### Simple Example
//!
//! ```
//! use battler_wamp::{
//!     core::{
//!         hash::HashMap,
//!         uri::Uri,
//!     },
//!     peer::{
//!         Peer,
//!         PeerConfig,
//!         PublishedEvent,
//!         ReceivedEvent,
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
//!     Dictionary,
//!     List,
//!     Value,
//! };
//! use tokio::task::JoinHandle;
//!
//! async fn start_router() -> anyhow::Result<(RouterHandle, JoinHandle<()>)> {
//!     let mut config = RouterConfig::default();
//!     config.realms.push(RealmConfig {
//!         name: "Realm".to_owned(),
//!         uri: Uri::try_from("com.battler_wamp.realm").unwrap(),
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
//! async fn publisher(router_handle: RouterHandle) {
//!     let publisher = new_web_socket_peer(PeerConfig::default()).unwrap();
//!     publisher
//!         .connect(&format!("ws://{}", router_handle.local_addr()))
//!         .await
//!         .unwrap();
//!     publisher
//!         .join_realm("com.battler_wamp.realm")
//!         .await
//!         .unwrap();
//!
//!     // Publish one event to a topic.
//!     publisher
//!         .publish(
//!             Uri::try_from("com.battler_wamp.topic1").unwrap(),
//!             PublishedEvent {
//!                 arguments: List::from_iter([Value::Integer(123)]),
//!                 arguments_keyword: Dictionary::from_iter([(
//!                     "foo".to_owned(),
//!                     Value::String("bar".to_owned()),
//!                 )]),
//!                 ..Default::default()
//!             },
//!         )
//!         .await
//!         .unwrap();
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let (router_handle, _) = start_router().await.unwrap();
//!
//!     let subscriber = new_web_socket_peer(PeerConfig::default()).unwrap();
//!     subscriber
//!         .connect(&format!("ws://{}", router_handle.local_addr()))
//!         .await
//!         .unwrap();
//!     subscriber
//!         .join_realm("com.battler_wamp.realm")
//!         .await
//!         .unwrap();
//!
//!     // Subscribe to a topic.
//!     let mut subscription = subscriber
//!         .subscribe(Uri::try_from("com.battler_wamp.topic1").unwrap())
//!         .await
//!         .unwrap();
//!
//!     tokio::spawn(publisher(router_handle.clone()));
//!
//!     // The subscription contains a channel for receiving events.
//!     while let Ok(event) = subscription.event_rx.recv().await {
//!         assert_eq!(
//!             event,
//!             ReceivedEvent {
//!                 arguments: List::from_iter([Value::Integer(123)]),
//!                 arguments_keyword: Dictionary::from_iter([(
//!                     "foo".to_owned(),
//!                     Value::String("bar".to_owned())
//!                 )]),
//!                 topic: Some(Uri::try_from("com.battler_wamp.topic1").unwrap()),
//!             }
//!         );
//!
//!         // Unsubscribe to close the event loop.
//!         subscriber.unsubscribe(subscription.id).await.unwrap();
//!     }
//! }
//! ```
//!
//! #### Pattern-Based Subscription
//!
//! ```
//! use battler_wamp::{
//!     core::{
//!         hash::HashMap,
//!         match_style::MatchStyle,
//!         uri::{
//!             Uri,
//!             WildcardUri,
//!         },
//!     },
//!     peer::{
//!         Peer,
//!         PeerConfig,
//!         PublishedEvent,
//!         ReceivedEvent,
//!         SubscriptionOptions,
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
//!     Dictionary,
//!     List,
//!     Value,
//! };
//! use tokio::task::JoinHandle;
//!
//! async fn start_router() -> anyhow::Result<(RouterHandle, JoinHandle<()>)> {
//!     let mut config = RouterConfig::default();
//!     config.realms.push(RealmConfig {
//!         name: "Realm".to_owned(),
//!         uri: Uri::try_from("com.battler_wamp.realm").unwrap(),
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
//! async fn publisher(router_handle: RouterHandle) {
//!     let publisher = new_web_socket_peer(PeerConfig::default()).unwrap();
//!     publisher
//!         .connect(&format!("ws://{}", router_handle.local_addr()))
//!         .await
//!         .unwrap();
//!     publisher
//!         .join_realm("com.battler_wamp.realm")
//!         .await
//!         .unwrap();
//!
//!     publisher
//!         .publish(
//!             Uri::try_from("com.battler_wamp.topics.1").unwrap(),
//!             PublishedEvent {
//!                 arguments: List::from_iter([Value::Integer(123)]),
//!                 ..Default::default()
//!             },
//!         )
//!         .await
//!         .unwrap();
//!     publisher
//!         .publish(
//!             Uri::try_from("com.battler_wamp.topics.2").unwrap(),
//!             PublishedEvent {
//!                 arguments: List::from_iter([Value::Integer(456)]),
//!                 ..Default::default()
//!             },
//!         )
//!         .await
//!         .unwrap();
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let (router_handle, _) = start_router().await.unwrap();
//!
//!     let subscriber = new_web_socket_peer(PeerConfig::default()).unwrap();
//!     subscriber
//!         .connect(&format!("ws://{}", router_handle.local_addr()))
//!         .await
//!         .unwrap();
//!     subscriber
//!         .join_realm("com.battler_wamp.realm")
//!         .await
//!         .unwrap();
//!
//!     // Subscribe to a topic.
//!     let mut subscription = subscriber
//!         .subscribe_with_options(
//!             WildcardUri::try_from("com.battler_wamp.topics").unwrap(),
//!             SubscriptionOptions {
//!                 match_style: Some(MatchStyle::Prefix),
//!             },
//!         )
//!         .await
//!         .unwrap();
//!
//!     tokio::spawn(publisher(router_handle.clone()));
//!
//!     assert_eq!(
//!         subscription.event_rx.recv().await.unwrap(),
//!         ReceivedEvent {
//!             arguments: List::from_iter([Value::Integer(123)]),
//!             topic: Some(Uri::try_from("com.battler_wamp.topics.1").unwrap()),
//!             ..Default::default()
//!         }
//!     );
//!     assert_eq!(
//!         subscription.event_rx.recv().await.unwrap(),
//!         ReceivedEvent {
//!             arguments: List::from_iter([Value::Integer(456)]),
//!             topic: Some(Uri::try_from("com.battler_wamp.topics.2").unwrap()),
//!             ..Default::default()
//!         }
//!     );
//! }
//! ```
//!
//! ### RPC
//!
//! Peers can register procedures that other peers can call.
//!
//! #### Simple Example
//!
//! ```
//! use battler_wamp::{
//!     core::{
//!         hash::HashMap,
//!         uri::Uri,
//!     },
//!     peer::{
//!         Peer,
//!         PeerConfig,
//!         Procedure,
//!         ProcedureMessage,
//!         RpcCall,
//!         RpcResult,
//!         RpcYield,
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
//!     Dictionary,
//!     List,
//!     Value,
//! };
//! use tokio::task::JoinHandle;
//!
//! async fn start_router() -> anyhow::Result<(RouterHandle, JoinHandle<()>)> {
//!     let mut config = RouterConfig::default();
//!     config.realms.push(RealmConfig {
//!         name: "Realm".to_owned(),
//!         uri: Uri::try_from("com.battler_wamp.realm").unwrap(),
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
//! async fn start_callee(router_handle: RouterHandle) -> WebSocketPeer {
//!     let callee = new_web_socket_peer(PeerConfig::default()).unwrap();
//!     callee
//!         .connect(&format!("ws://{}", router_handle.local_addr()))
//!         .await
//!         .unwrap();
//!     callee.join_realm("com.battler_wamp.realm").await.unwrap();
//!
//!     // Register a procedure that echoes the caller's input.
//!     let mut procedure = callee
//!         .register(Uri::try_from("com.battler_wamp.echo").unwrap())
//!         .await
//!         .unwrap();
//!
//!     // Handle the procedure in a separate task.
//!     async fn handler(mut procedure: Procedure) {
//!         while let Ok(message) = procedure.procedure_message_rx.recv().await {
//!             match message {
//!                 ProcedureMessage::Invocation(invocation) => {
//!                     let result = RpcYield {
//!                         arguments: invocation.arguments.clone(),
//!                         arguments_keyword: invocation.arguments_keyword.clone(),
//!                     };
//!                     invocation.respond_ok(result).await.unwrap();
//!                 }
//!                 _ => (),
//!             }
//!         }
//!     }
//!
//!     tokio::spawn(handler(procedure));
//!     callee
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let (router_handle, _) = start_router().await.unwrap();
//!
//!     let callee = start_callee(router_handle.clone()).await;
//!
//!     let caller = new_web_socket_peer(PeerConfig::default()).unwrap();
//!     caller
//!         .connect(&format!("ws://{}", router_handle.local_addr()))
//!         .await
//!         .unwrap();
//!     caller.join_realm("com.battler_wamp.realm").await.unwrap();
//!
//!     let rpc = caller
//!         .call(
//!             Uri::try_from("com.battler_wamp.echo").unwrap(),
//!             RpcCall {
//!                 arguments: List::from_iter([Value::Integer(1), Value::Integer(2)]),
//!                 arguments_keyword: Dictionary::from_iter([(
//!                     "foo".to_owned(),
//!                     Value::String("bar".to_owned()),
//!                 )]),
//!                 ..Default::default()
//!             },
//!         )
//!         .await
//!         .unwrap();
//!     assert_eq!(
//!         rpc.result().await.unwrap(),
//!         RpcResult {
//!             arguments: List::from_iter([Value::Integer(1), Value::Integer(2)]),
//!             arguments_keyword: Dictionary::from_iter([(
//!                 "foo".to_owned(),
//!                 Value::String("bar".to_owned()),
//!             )]),
//!             ..Default::default()
//!         }
//!     );
//! }
//! ```
//!
//! #### Custom Error Reporting
//!
//! ```
//! use battler_wamp::{
//!     core::{
//!         error::WampError,
//!         hash::HashMap,
//!         uri::Uri,
//!     },
//!     peer::{
//!         Peer,
//!         PeerConfig,
//!         Procedure,
//!         ProcedureMessage,
//!         RpcCall,
//!         RpcResult,
//!         RpcYield,
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
//!     Dictionary,
//!     List,
//!     Value,
//! };
//! use tokio::task::JoinHandle;
//!
//! async fn start_router() -> anyhow::Result<(RouterHandle, JoinHandle<()>)> {
//!     let mut config = RouterConfig::default();
//!     config.realms.push(RealmConfig {
//!         name: "Realm".to_owned(),
//!         uri: Uri::try_from("com.battler_wamp.realm").unwrap(),
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
//! async fn start_callee(router_handle: RouterHandle) -> WebSocketPeer {
//!     let callee = new_web_socket_peer(PeerConfig::default()).unwrap();
//!     callee
//!         .connect(&format!("ws://{}", router_handle.local_addr()))
//!         .await
//!         .unwrap();
//!     callee.join_realm("com.battler_wamp.realm").await.unwrap();
//!
//!     let mut procedure = callee
//!         .register(Uri::try_from("com.battler_wamp.add2").unwrap())
//!         .await
//!         .unwrap();
//!
//!     // Handle the procedure in a separate task.
//!     async fn handler(mut procedure: Procedure) {
//!         while let Ok(message) = procedure.procedure_message_rx.recv().await {
//!             match message {
//!                 ProcedureMessage::Invocation(invocation) => {
//!                     let result = if invocation.arguments.len() != 2 {
//!                         Err(WampError::new(
//!                             Uri::try_from("com.battler_wamp.error.add_error").unwrap(),
//!                             "2 arguments required".to_owned(),
//!                         ))
//!                     } else {
//!                         match (&invocation.arguments[0], &invocation.arguments[1]) {
//!                             (Value::Integer(a), Value::Integer(b)) => Ok(RpcYield {
//!                                 arguments: List::from_iter([Value::Integer(a + b)]),
//!                                 ..Default::default()
//!                             }),
//!                             _ => Err(WampError::new(
//!                                 Uri::try_from("com.battler_wamp.error.add_error").unwrap(),
//!                                 "integers required",
//!                             )),
//!                         }
//!                     };
//!                     invocation.respond(result).await.unwrap();
//!                 }
//!                 _ => (),
//!             }
//!         }
//!     }
//!
//!     tokio::spawn(handler(procedure));
//!     callee
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let (router_handle, _) = start_router().await.unwrap();
//!
//!     let callee = start_callee(router_handle.clone()).await;
//!
//!     let caller = new_web_socket_peer(PeerConfig::default()).unwrap();
//!     caller
//!         .connect(&format!("ws://{}", router_handle.local_addr()))
//!         .await
//!         .unwrap();
//!     caller.join_realm("com.battler_wamp.realm").await.unwrap();
//!
//!     assert_eq!(
//!         caller
//!             .call_and_wait(
//!                 Uri::try_from("com.battler_wamp.add2").unwrap(),
//!                 RpcCall::default()
//!             )
//!             .await
//!             .unwrap_err()
//!             .downcast::<WampError>()
//!             .unwrap(),
//!         WampError::new(
//!             Uri::try_from("com.battler_wamp.error.add_error").unwrap(),
//!             "2 arguments required"
//!         ),
//!     );
//!
//!     assert_eq!(
//!         caller
//!             .call_and_wait(
//!                 Uri::try_from("com.battler_wamp.add2").unwrap(),
//!                 RpcCall {
//!                     arguments: List::from_iter([Value::Integer(1), Value::Integer(2)]),
//!                     ..Default::default()
//!                 }
//!             )
//!             .await
//!             .unwrap(),
//!         RpcResult {
//!             arguments: List::from_iter([Value::Integer(3)]),
//!             ..Default::default()
//!         }
//!     );
//! }
//! ```
//!
//! #### Pattern-Based Registration
//!
//! ```
//! use battler_wamp::{
//!     core::{
//!         hash::HashMap,
//!         match_style::MatchStyle,
//!         uri::{
//!             Uri,
//!             WildcardUri,
//!         },
//!     },
//!     peer::{
//!         Peer,
//!         PeerConfig,
//!         Procedure,
//!         ProcedureMessage,
//!         ProcedureOptions,
//!         RpcCall,
//!         RpcResult,
//!         RpcYield,
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
//!     Dictionary,
//!     List,
//!     Value,
//! };
//! use tokio::task::JoinHandle;
//!
//! async fn start_router() -> anyhow::Result<(RouterHandle, JoinHandle<()>)> {
//!     let mut config = RouterConfig::default();
//!     config.realms.push(RealmConfig {
//!         name: "Realm".to_owned(),
//!         uri: Uri::try_from("com.battler_wamp.realm").unwrap(),
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
//! async fn start_callee(router_handle: RouterHandle) -> WebSocketPeer {
//!     let callee = new_web_socket_peer(PeerConfig::default()).unwrap();
//!     callee
//!         .connect(&format!("ws://{}", router_handle.local_addr()))
//!         .await
//!         .unwrap();
//!     callee.join_realm("com.battler_wamp.realm").await.unwrap();
//!
//!     let mut procedure = callee
//!         .register_with_options(
//!             WildcardUri::try_from("com.battler_wamp..echo").unwrap(),
//!             ProcedureOptions {
//!                 match_style: Some(MatchStyle::Wildcard),
//!                 ..Default::default()
//!             },
//!         )
//!         .await
//!         .unwrap();
//!
//!     // Handle the procedure in a separate task.
//!     async fn handler(mut procedure: Procedure) {
//!         while let Ok(message) = procedure.procedure_message_rx.recv().await {
//!             match message {
//!                 ProcedureMessage::Invocation(invocation) => {
//!                     let result = RpcYield {
//!                         arguments: invocation.arguments.clone(),
//!                         arguments_keyword: invocation.arguments_keyword.clone(),
//!                     };
//!                     invocation.respond_ok(result).await.unwrap();
//!                 }
//!                 _ => (),
//!             }
//!         }
//!     }
//!
//!     tokio::spawn(handler(procedure));
//!     callee
//! }
//!
//! #[tokio::main]
//! async fn main() {
//!     let (router_handle, _) = start_router().await.unwrap();
//!
//!     let callee = start_callee(router_handle.clone()).await;
//!
//!     let caller = new_web_socket_peer(PeerConfig::default()).unwrap();
//!     caller
//!         .connect(&format!("ws://{}", router_handle.local_addr()))
//!         .await
//!         .unwrap();
//!     caller.join_realm("com.battler_wamp.realm").await.unwrap();
//!
//!     assert_eq!(
//!         caller
//!             .call_and_wait(
//!                 Uri::try_from("com.battler_wamp.v1.echo").unwrap(),
//!                 RpcCall {
//!                     arguments: List::from_iter([Value::Integer(1), Value::Integer(2)]),
//!                     arguments_keyword: Dictionary::from_iter([(
//!                         "foo".to_owned(),
//!                         Value::String("bar".to_owned()),
//!                     )]),
//!                     ..Default::default()
//!                 }
//!             )
//!             .await
//!             .unwrap(),
//!         RpcResult {
//!             arguments: List::from_iter([Value::Integer(1), Value::Integer(2)]),
//!             arguments_keyword: Dictionary::from_iter([(
//!                 "foo".to_owned(),
//!                 Value::String("bar".to_owned()),
//!             )]),
//!             ..Default::default()
//!         }
//!     );
//!
//!     assert_eq!(
//!         caller
//!             .call_and_wait(
//!                 Uri::try_from("com.battler_wamp.v2.echo").unwrap(),
//!                 RpcCall {
//!                     arguments: List::from_iter([Value::String("abc".to_owned())]),
//!                     ..Default::default()
//!                 }
//!             )
//!             .await
//!             .unwrap(),
//!         RpcResult {
//!             arguments: List::from_iter([Value::String("abc".to_owned())]),
//!             ..Default::default()
//!         }
//!     );
//! }
//! ```
pub mod auth;
pub mod core;
pub mod message;
pub mod peer;
pub mod router;
pub mod serializer;
pub mod transport;
