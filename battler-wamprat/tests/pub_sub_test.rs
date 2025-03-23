use anyhow::{
    Error,
    Result,
};
use battler_wamp::{
    core::uri::{
        Uri,
        WildcardUri,
    },
    peer::{
        PeerConfig,
        PublishedEvent,
        ReceivedEvent,
        WebSocketPeer,
        new_web_socket_peer,
    },
    router::{
        EmptyPubSubPolicies,
        EmptyRpcPolicies,
        RealmAuthenticationConfig,
        RealmConfig,
        RouterConfig,
        RouterHandle,
        new_web_socket_router,
    },
};
use battler_wamp_values::{
    List,
    Value,
    WampList,
};
use battler_wamprat::{
    peer::{
        PeerBuilder,
        PeerConnectionType,
        PeerHandle,
    },
    subscription::{
        TypedPatternMatchedSubscription,
        TypedSubscription,
    },
};
use battler_wamprat_message::WampApplicationMessage;
use battler_wamprat_uri::WampUriMatcher;
use tokio::{
    sync::broadcast,
    task::JoinHandle,
};

const REALM: &str = "com.battler.test";

async fn start_router(port: u16) -> Result<(RouterHandle, JoinHandle<()>)> {
    let mut config = RouterConfig::default();
    // Must use a stable port for reconnection.
    config.port = port;
    config.realms.push(RealmConfig {
        name: "test".to_owned(),
        uri: Uri::try_from(REALM)?,
        authentication: RealmAuthenticationConfig::default(),
    });
    let router = new_web_socket_router(
        config,
        Box::new(EmptyPubSubPolicies::default()),
        Box::new(EmptyRpcPolicies::default()),
    )?;
    router.start().await
}

fn create_peer(name: &str) -> Result<WebSocketPeer> {
    let mut config = PeerConfig::default();
    config.name = name.to_owned();
    new_web_socket_peer(config)
}

#[derive(Debug, Clone, WampList)]
struct MessageEventArgs(String);

#[derive(Debug, Clone, WampApplicationMessage)]
struct MessageEvent(#[arguments] MessageEventArgs);

#[derive(Debug, Clone, PartialEq, Eq)]
enum ReceivedMessageEvent {
    Valid(String),
    Invalid { event: ReceivedEvent, error: String },
}

struct MessageEventHandler {
    events_tx: broadcast::Sender<ReceivedMessageEvent>,
}

impl TypedSubscription for MessageEventHandler {
    type Event = MessageEvent;

    async fn handle_event(&self, event: Self::Event) {
        self.events_tx
            .send(ReceivedMessageEvent::Valid(event.0.0))
            .unwrap();
    }

    async fn handle_invalid_event(&self, event: ReceivedEvent, error: Error) {
        self.events_tx
            .send(ReceivedMessageEvent::Invalid {
                event,
                error: error.to_string(),
            })
            .unwrap();
    }
}

#[derive(WampUriMatcher)]
#[uri("com.battler.event.{version}.{name}")]
struct EventPattern {
    version: u64,
    name: String,
}

impl TypedPatternMatchedSubscription for MessageEventHandler {
    type Pattern = EventPattern;
    type Event = MessageEvent;

    async fn handle_event(&self, event: Self::Event, _: Self::Pattern) {
        self.events_tx
            .send(ReceivedMessageEvent::Valid(event.0.0))
            .unwrap();
    }

    async fn handle_invalid_event(&self, event: ReceivedEvent, error: Error) {
        self.events_tx
            .send(ReceivedMessageEvent::Invalid {
                event,
                error: error.to_string(),
            })
            .unwrap();
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn receives_events() {
    test_utils::setup::setup_test_environment();

    // Start a router.
    let (router_handle, router_join_handle) = start_router(0).await.unwrap();

    // Create a publisher and subscriber.
    let (publisher_handle, publisher_join_handle) = PeerBuilder::new(PeerConnectionType::Remote(
        format!("ws://{}", router_handle.local_addr()),
    ))
    .start(
        create_peer("publisher").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );
    publisher_handle.wait_until_ready().await.unwrap();

    let (subscriber_handle, subscriber_handle_join_handle) = PeerBuilder::new(
        PeerConnectionType::Remote(format!("ws://{}", router_handle.local_addr())),
    )
    .start(
        create_peer("publisher").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );
    subscriber_handle.wait_until_ready().await.unwrap();

    // Create a subscription that writes events to a channel.
    let (events_tx, mut events_rx) = broadcast::channel(16);
    assert_matches::assert_matches!(
        subscriber_handle
            .subscribe(
                Uri::try_from("com.battler.message").unwrap(),
                MessageEventHandler { events_tx }
            )
            .await,
        Ok(())
    );

    // Publish one valid and one invalid event.
    assert_matches::assert_matches!(
        publisher_handle
            .publish(
                Uri::try_from("com.battler.message").unwrap(),
                MessageEvent(MessageEventArgs("Hello, world!".to_owned())),
            )
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        publisher_handle
            .publish_unchecked(
                Uri::try_from("com.battler.message").unwrap(),
                PublishedEvent {
                    arguments: List::from_iter([Value::Integer(123)]),
                    ..Default::default()
                },
            )
            .await,
        Ok(())
    );

    // Receive the two events.
    let mut events = Vec::new();
    while let Ok(event) = events_rx.recv().await {
        events.push(event);
        if events.len() >= 2 {
            break;
        }
    }

    // Validate the two events were received correctly.
    pretty_assertions::assert_eq!(
        events,
        Vec::from_iter([
            ReceivedMessageEvent::Valid("Hello, world!".to_owned()),
            ReceivedMessageEvent::Invalid {
                event: ReceivedEvent {
                    arguments: List::from_iter([Value::Integer(123)]),
                    topic: Some(Uri::try_from("com.battler.message").unwrap()),
                    ..Default::default()
                },
                error: "value must be a string; failed to deserialize list member field_0 of MessageEventArgs; failed to deserialize arguments of MessageEvent".to_owned()
            }
        ])
    );

    // Unsubscribe and show the next message is not received.
    assert_matches::assert_matches!(
        subscriber_handle
            .unsubscribe(&WildcardUri::try_from("com.battler.message").unwrap())
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        publisher_handle
            .publish(
                Uri::try_from("com.battler.message").unwrap(),
                MessageEvent(MessageEventArgs("another message".to_owned())),
            )
            .await,
        Ok(())
    );

    assert_matches::assert_matches!(events_rx.recv().await, Err(err) => {
        assert_eq!(err.to_string(), "channel closed");
    });

    subscriber_handle.cancel().unwrap();
    subscriber_handle_join_handle.await.unwrap();

    publisher_handle.cancel().unwrap();
    publisher_join_handle.await.unwrap();

    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn resubscribes_on_reconnect() {
    test_utils::setup::setup_test_environment();

    // Start a router.
    let (router_handle, router_join_handle) = start_router(8889).await.unwrap();

    // Create a publisher and subscriber.
    let (publisher_handle, publisher_join_handle) = PeerBuilder::new(PeerConnectionType::Remote(
        format!("ws://{}", router_handle.local_addr()),
    ))
    .start(
        create_peer("publisher").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );
    publisher_handle.wait_until_ready().await.unwrap();

    let (subscriber_handle, subscriber_handle_join_handle) = PeerBuilder::new(
        PeerConnectionType::Remote(format!("ws://{}", router_handle.local_addr())),
    )
    .start(
        create_peer("publisher").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );
    subscriber_handle.wait_until_ready().await.unwrap();

    // Create a subscription that writes events to a channel.
    let (events_tx, mut events_rx) = broadcast::channel(16);
    assert_matches::assert_matches!(
        subscriber_handle
            .subscribe(
                Uri::try_from("com.battler.message").unwrap(),
                MessageEventHandler { events_tx }
            )
            .await,
        Ok(())
    );

    // Publish one event.
    assert_matches::assert_matches!(
        publisher_handle
            .publish(
                Uri::try_from("com.battler.message").unwrap(),
                MessageEvent(MessageEventArgs("first".to_owned())),
            )
            .await,
        Ok(())
    );

    // Stop the router to disconnect the peer.
    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();

    // Restart the router.
    let (router_handle, router_join_handle) = start_router(8889).await.unwrap();

    // Wait again, to ensure we are properly subscribed before publishing the message.
    subscriber_handle.wait_until_ready().await.unwrap();

    // Publish another event.
    assert_matches::assert_matches!(
        publisher_handle
            .publish(
                Uri::try_from("com.battler.message").unwrap(),
                MessageEvent(MessageEventArgs("second".to_owned())),
            )
            .await,
        Ok(())
    );

    // Receive the two events.
    let mut events = Vec::new();
    while let Ok(event) = events_rx.recv().await {
        events.push(event);
        if events.len() >= 2 {
            break;
        }
    }

    // Validate the two events were received correctly.
    pretty_assertions::assert_eq!(
        events,
        Vec::from_iter([
            ReceivedMessageEvent::Valid("first".to_owned()),
            ReceivedMessageEvent::Valid("second".to_owned()),
        ])
    );

    // Unsubscribe and show the subscription is not restored on the next reconnect.
    assert_matches::assert_matches!(
        subscriber_handle
            .unsubscribe(&WildcardUri::try_from("com.battler.message").unwrap())
            .await,
        Ok(())
    );

    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
    let (router_handle, router_join_handle) = start_router(8889).await.unwrap();
    subscriber_handle.wait_until_ready().await.unwrap();

    assert_matches::assert_matches!(
        publisher_handle
            .publish(
                Uri::try_from("com.battler.message").unwrap(),
                MessageEvent(MessageEventArgs("third".to_owned())),
            )
            .await,
        Ok(())
    );

    assert_matches::assert_matches!(events_rx.recv().await, Err(err) => {
        assert_eq!(err.to_string(), "channel closed");
    });

    subscriber_handle.cancel().unwrap();
    subscriber_handle_join_handle.await.unwrap();

    publisher_handle.cancel().unwrap();
    publisher_join_handle.await.unwrap();

    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn retries_publish_during_reconnect() {
    test_utils::setup::setup_test_environment();

    // Start a router.
    let (router_handle, router_join_handle) = start_router(0).await.unwrap();

    // Create a publisher and subscriber.
    let (publisher_handle, publisher_join_handle) = PeerBuilder::new(PeerConnectionType::Remote(
        format!("ws://{}", router_handle.local_addr()),
    ))
    .start(
        create_peer("publisher").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );
    publisher_handle.wait_until_ready().await.unwrap();

    let (subscriber_handle, subscriber_handle_join_handle) = PeerBuilder::new(
        PeerConnectionType::Remote(format!("ws://{}", router_handle.local_addr())),
    )
    .start(
        create_peer("publisher").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );
    subscriber_handle.wait_until_ready().await.unwrap();

    // Create a subscription that writes events to a channel.
    let (events_tx, mut events_rx) = broadcast::channel(16);
    assert_matches::assert_matches!(
        subscriber_handle
            .subscribe(
                Uri::try_from("com.battler.message").unwrap(),
                MessageEventHandler { events_tx }
            )
            .await,
        Ok(())
    );

    async fn publish<S>(publisher_handle: PeerHandle<S>)
    where
        S: Send + Sync + 'static,
    {
        assert_matches::assert_matches!(
            publisher_handle
                .publish(
                    Uri::try_from("com.battler.message").unwrap(),
                    MessageEvent(MessageEventArgs("Hello, world!".to_owned())),
                )
                .await,
            Ok(())
        );
    }

    let publish_handle = tokio::spawn(publish(publisher_handle.clone()));

    // Kick the publisher immediately.
    router_handle
        .end_session(
            Uri::try_from(REALM).unwrap(),
            publisher_handle.current_session_id().await.unwrap(),
        )
        .await
        .unwrap();

    // Receive the event.
    assert_matches::assert_matches!(
        events_rx.recv().await,
        Ok(ReceivedMessageEvent::Valid(msg)) => {
            assert_eq!(msg, "Hello, world!".to_owned());
        }
    );

    publish_handle.await.unwrap();

    subscriber_handle.cancel().unwrap();
    subscriber_handle_join_handle.await.unwrap();

    publisher_handle.cancel().unwrap();
    publisher_join_handle.await.unwrap();

    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn receives_pattern_based_events() {
    test_utils::setup::setup_test_environment();

    // Start a router.
    let (router_handle, router_join_handle) = start_router(0).await.unwrap();

    // Create a publisher and subscriber.
    let (publisher_handle, publisher_join_handle) = PeerBuilder::new(PeerConnectionType::Remote(
        format!("ws://{}", router_handle.local_addr()),
    ))
    .start(
        create_peer("publisher").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );
    publisher_handle.wait_until_ready().await.unwrap();

    let (subscriber_handle, subscriber_handle_join_handle) = PeerBuilder::new(
        PeerConnectionType::Remote(format!("ws://{}", router_handle.local_addr())),
    )
    .start(
        create_peer("publisher").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );
    subscriber_handle.wait_until_ready().await.unwrap();

    // Create a subscription that writes events to a channel.
    let (events_tx, mut events_rx) = broadcast::channel(16);
    assert_matches::assert_matches!(
        subscriber_handle
            .subscribe_pattern_matched(MessageEventHandler { events_tx })
            .await,
        Ok(())
    );

    assert_matches::assert_matches!(
        publisher_handle
            .publish(
                Uri::try_from("com.battler.event.1.abc").unwrap(),
                MessageEvent(MessageEventArgs("foo".to_owned())),
            )
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        publisher_handle
            .publish(
                Uri::try_from("com.battler.event.2.def").unwrap(),
                MessageEvent(MessageEventArgs("bar".to_owned())),
            )
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        publisher_handle
            .publish_unchecked(
                Uri::try_from("com.battler.event.3.ghi").unwrap(),
                PublishedEvent {
                    arguments: List::from_iter([Value::Integer(123)]),
                    ..Default::default()
                },
            )
            .await,
        Ok(())
    );

    // Receive the three events.
    let mut events = Vec::new();
    while let Ok(event) = events_rx.recv().await {
        log::warn!("{event:?}");
        events.push(event);
        if events.len() >= 3 {
            break;
        }
    }

    // Validate the three events were received correctly.
    pretty_assertions::assert_eq!(
        events,
        Vec::from_iter([
            ReceivedMessageEvent::Valid("foo".to_owned()),
            ReceivedMessageEvent::Valid("bar".to_owned()),
            ReceivedMessageEvent::Invalid {
                event: ReceivedEvent {
                    arguments: List::from_iter([Value::Integer(123)]),
                    topic: Some(Uri::try_from("com.battler.event.3.ghi").unwrap()),
                    ..Default::default()
                },
                error: "value must be a string; failed to deserialize list member field_0 of MessageEventArgs; failed to deserialize arguments of MessageEvent".to_owned()
            }
        ])
    );

    // Unsubscribe and show the next message is not received.
    assert_matches::assert_matches!(
        subscriber_handle
            .unsubscribe(&EventPattern::uri_for_router())
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        publisher_handle
            .publish(
                Uri::try_from("com.battler.event.4.jkl").unwrap(),
                MessageEvent(MessageEventArgs("another message".to_owned())),
            )
            .await,
        Ok(())
    );

    assert_matches::assert_matches!(events_rx.recv().await, Err(err) => {
        assert_eq!(err.to_string(), "channel closed");
    });

    subscriber_handle.cancel().unwrap();
    subscriber_handle_join_handle.await.unwrap();

    publisher_handle.cancel().unwrap();
    publisher_join_handle.await.unwrap();

    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}
