use std::time::Duration;

use anyhow::{
    Error,
    Result,
};
use battler_wamp::{
    core::{
        error::{
            BasicError,
            InteractionError,
        },
        hash::HashSet,
        id::Id,
        roles::RouterRole,
        uri::Uri,
    },
    peer::{
        new_web_socket_peer,
        Event,
        PeerConfig,
        Subscription,
        WebSocketPeer,
    },
    router::{
        new_web_socket_router,
        EmptyPubSubPolicies,
        EmptyRpcPolicies,
        RealmConfig,
        RouterConfig,
        RouterHandle,
    },
};
use battler_wamp_values::{
    Dictionary,
    List,
    Value,
};
use tokio::{
    sync::broadcast::error::RecvError,
    task::JoinHandle,
};

const REALM: &str = "com.battler.test";

async fn start_router_with_config(
    mut config: RouterConfig,
) -> Result<(RouterHandle, JoinHandle<()>)> {
    config.realms.push(RealmConfig {
        name: "test".to_owned(),
        uri: Uri::try_from(REALM)?,
    });
    let router = new_web_socket_router(
        config,
        Box::new(EmptyPubSubPolicies::default()),
        Box::new(EmptyRpcPolicies::default()),
    )?;
    router.start().await
}

async fn start_router() -> Result<(RouterHandle, JoinHandle<()>)> {
    start_router_with_config(RouterConfig::default()).await
}

fn create_peer(name: &str) -> Result<WebSocketPeer> {
    let mut config = PeerConfig::default();
    config.name = name.to_owned();
    new_web_socket_peer(config)
}

#[tokio::test]
async fn peer_receives_published_messages_for_topic() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let publisher = create_peer("publisher").unwrap();
    let subscriber = create_peer("subscriber").unwrap();

    assert_matches::assert_matches!(
        publisher
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(publisher.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        subscriber
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(subscriber.join_realm(REALM).await, Ok(()));

    let mut subscription = subscriber
        .subscribe(Uri::try_from("com.battler.topic1").unwrap())
        .await
        .unwrap();

    // Publish 10 messages.
    for i in 0..10 {
        assert_matches::assert_matches!(
            publisher
                .publish(
                    Uri::try_from("com.battler.topic1").unwrap(),
                    Event {
                        arguments: List::from_iter([Value::Integer(i)]),
                        arguments_keyword: Dictionary::from_iter([(
                            "index".to_owned(),
                            Value::Integer(i)
                        )]),
                    }
                )
                .await,
            Ok(())
        );
    }

    // Subscriber should only receive 5 messages.
    let mut seen = HashSet::default();
    while let Ok(event) = subscription.event_rx.recv().await {
        assert_matches::assert_matches!(event.arguments.get(0), Some(Value::Integer(i)) => {
            seen.insert(*i);
            pretty_assertions::assert_eq!(event, Event {
                arguments: List::from_iter([Value::Integer(*i)]),
                arguments_keyword: Dictionary::from_iter([(
                    "index".to_owned(),
                    Value::Integer(*i)
                )])
            });
        });

        // Unsubscribe on the 5th message.
        if seen.len() >= 5 {
            assert_matches::assert_matches!(subscriber.unsubscribe(subscription.id).await, Ok(()));
            break;
        }
    }

    pretty_assertions::assert_eq!(seen, HashSet::from_iter([0, 1, 2, 3, 4]));
}

#[tokio::test]
async fn event_channel_closes_automatically_when_unsubscribing() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let subscriber = create_peer("subscriber").unwrap();

    assert_matches::assert_matches!(
        subscriber
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(subscriber.join_realm(REALM).await, Ok(()));

    let mut subscription = subscriber
        .subscribe(Uri::try_from("com.battler.topic1").unwrap())
        .await
        .unwrap();
    let subscription_id = subscription.id;

    assert_matches::assert_matches!(subscriber.unsubscribe(subscription_id).await, Ok(()));

    assert_matches::assert_matches!(subscription.event_rx.recv().await, Err(RecvError::Closed));
}

#[tokio::test]
async fn event_channel_closes_automatically_when_leaving_realm() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let subscriber = create_peer("subscriber").unwrap();

    assert_matches::assert_matches!(
        subscriber
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(subscriber.join_realm(REALM).await, Ok(()));

    let mut subscription = subscriber
        .subscribe(Uri::try_from("com.battler.topic1").unwrap())
        .await
        .unwrap();

    assert_matches::assert_matches!(subscriber.leave_realm().await, Ok(()));

    assert_matches::assert_matches!(subscription.event_rx.recv().await, Err(RecvError::Closed));
}

#[tokio::test]
async fn event_channel_closes_automatically_when_disconnecting() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let subscriber = create_peer("subscriber").unwrap();

    assert_matches::assert_matches!(
        subscriber
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(subscriber.join_realm(REALM).await, Ok(()));

    let mut subscription = subscriber
        .subscribe(Uri::try_from("com.battler.topic1").unwrap())
        .await
        .unwrap();

    assert_matches::assert_matches!(subscriber.disconnect().await, Ok(()));

    assert_matches::assert_matches!(subscription.event_rx.recv().await, Err(RecvError::Closed));
}

#[tokio::test]
async fn peer_does_not_receive_events_for_different_topic() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let publisher = create_peer("publisher").unwrap();
    let subscriber = create_peer("subscriber").unwrap();

    assert_matches::assert_matches!(
        publisher
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(publisher.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        subscriber
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(subscriber.join_realm(REALM).await, Ok(()));

    let mut subscription = subscriber
        .subscribe(Uri::try_from("com.battler.topic1").unwrap())
        .await
        .unwrap();

    assert_matches::assert_matches!(
        publisher
            .publish(
                Uri::try_from("com.battler.topic2").unwrap(),
                Event {
                    arguments: List::from_iter([Value::Bool(false)]),
                    arguments_keyword: Dictionary::default(),
                }
            )
            .await,
        Ok(())
    );

    async fn wait_for_event(subscription: &mut Subscription) -> Result<Event> {
        tokio::select! {
            event = subscription.event_rx.recv() => {
                event.map_err(Error::new)
            }
            _ = tokio::time::sleep(Duration::from_secs(5)) => {
                Err(Error::msg("timed out"))
            }
        }
    }

    assert_matches::assert_matches!(wait_for_event(&mut subscription).await, Err(err) => {
        assert_eq!(err.to_string(), "timed out");
    });
}

#[tokio::test]
async fn cannot_unsubscribe_from_non_existent_subscription() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let subscriber = create_peer("subscriber").unwrap();

    assert_matches::assert_matches!(
        subscriber
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(subscriber.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(subscriber.unsubscribe(Id::MAX).await, Err(err) =>{
        assert_matches::assert_matches!(err.downcast::<InteractionError>(), Ok(InteractionError::NoSuchSubscription));
    });

    let subscription = subscriber
        .subscribe(Uri::try_from("com.battler.topic1").unwrap())
        .await
        .unwrap();

    assert_matches::assert_matches!(subscriber.unsubscribe(subscription.id).await, Ok(()));
    assert_matches::assert_matches!(subscriber.unsubscribe(subscription.id).await, Err(err) =>{
        assert_matches::assert_matches!(err.downcast::<InteractionError>(), Ok(InteractionError::NoSuchSubscription));
    });
}

#[tokio::test]
async fn pub_sub_not_allowed_without_broker_role() {
    test_utils::setup::setup_test_environment();

    let mut config = RouterConfig::default();
    config.roles.remove(&RouterRole::Broker);
    let (router_handle, _) = start_router_with_config(config).await.unwrap();
    let peer = create_peer("peer").unwrap();

    assert_matches::assert_matches!(
        peer.connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(peer.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        peer.subscribe(Uri::try_from("com.battler.topic1").unwrap())
            .await,
        Err(err) => {
            assert_matches::assert_matches!(err.downcast::<BasicError>(), Ok(BasicError::NotAllowed(_)));
        }
    );
    assert_matches::assert_matches!(
        peer.publish(
            Uri::try_from("com.battler.topic1").unwrap(),
            Event {
                arguments: List::default(),
                arguments_keyword: Dictionary::default(),
            }
        )
        .await,
        Err(err) => {
            assert_matches::assert_matches!(err.downcast::<BasicError>(), Ok(BasicError::NotAllowed(_)));
        }
    );
}
