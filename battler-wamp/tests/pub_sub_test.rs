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
        match_style::MatchStyle,
        roles::RouterRole,
        uri::{
            Uri,
            WildcardUri,
        },
    },
    peer::{
        new_web_socket_peer,
        PeerConfig,
        PublishedEvent,
        ReceivedEvent,
        Subscription,
        SubscriptionOptions,
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
                    PublishedEvent {
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

        // Unsubscribe on the 5th message.
        if i == 4 {
            assert_matches::assert_matches!(subscriber.unsubscribe(subscription.id).await, Ok(()));
        }
    }

    // Subscriber should only receive 5 messages.
    let mut seen = HashSet::default();
    while let Ok(event) = subscription.event_rx.recv().await {
        assert_matches::assert_matches!(event.arguments.get(0), Some(Value::Integer(i)) => {
            seen.insert(*i);
            pretty_assertions::assert_eq!(event, ReceivedEvent {
                arguments: List::from_iter([Value::Integer(*i)]),
                arguments_keyword: Dictionary::from_iter([(
                    "index".to_owned(),
                    Value::Integer(*i)
                )]),
                topic: Some(Uri::try_from("com.battler.topic1").unwrap()),
            });
        });
    }

    // When we unsubscribe, we close out the event receiver channel. There is a race condition
    // between some of the last events being received and processed by the peer and the unsubscribe
    // actuating.
    //
    // Thus, we check that we received at most 5 messages.
    assert!(seen.len() <= 5);
    pretty_assertions::assert_eq!(seen, (0..seen.len() as u64).collect());
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
                PublishedEvent {
                    arguments: List::from_iter([Value::Bool(false)]),
                    arguments_keyword: Dictionary::default(),
                }
            )
            .await,
        Ok(())
    );

    async fn wait_for_event(subscription: &mut Subscription) -> Result<ReceivedEvent> {
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
            PublishedEvent {
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

#[tokio::test]
async fn publisher_does_not_receive_event() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let peer = create_peer("peer").unwrap();

    assert_matches::assert_matches!(
        peer.connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(peer.join_realm(REALM).await, Ok(()));

    let mut subscription = peer
        .subscribe(Uri::try_from("com.battler.topic1").unwrap())
        .await
        .unwrap();

    assert_matches::assert_matches!(
        peer.publish(
            Uri::try_from("com.battler.topic1").unwrap(),
            PublishedEvent::default(),
        )
        .await,
        Ok(())
    );

    tokio::select! {
        _ = subscription.event_rx.recv() => {
            assert!(false, "publisher received event for its own subscription");
        }
        _ = tokio::time::sleep(Duration::from_secs(5)) => (),
    }
}

#[tokio::test]
async fn publish_matches_subscription_by_prefix() {
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
        .subscribe_with_options(
            WildcardUri::try_from("com.battler.battle.abcd").unwrap(),
            SubscriptionOptions {
                match_style: Some(MatchStyle::Prefix),
            },
        )
        .await
        .unwrap();

    assert_matches::assert_matches!(
        publisher
            .publish(
                Uri::try_from("com.battler.battle.abcd.start").unwrap(),
                PublishedEvent::default(),
            )
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        publisher
            .publish(
                Uri::try_from("com.battler.battle.abcd.update").unwrap(),
                PublishedEvent::default(),
            )
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        publisher
            .publish(
                Uri::try_from("com.battler.battle.abcd.update").unwrap(),
                PublishedEvent::default(),
            )
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        publisher
            .publish(
                Uri::try_from("com.battler.battle.abcd.update").unwrap(),
                PublishedEvent::default(),
            )
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        publisher
            .publish(
                Uri::try_from("com.battler.battle.abcd.end").unwrap(),
                PublishedEvent::default(),
            )
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        publisher
            .publish(
                Uri::try_from("com.battler.battle.another.update").unwrap(),
                PublishedEvent::default(),
            )
            .await,
        Ok(())
    );

    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_matches::assert_matches!(subscriber.unsubscribe(subscription.id).await, Ok(()));

    let mut topics_seen = Vec::default();
    while let Ok(event) = subscription.event_rx.recv().await {
        topics_seen.push(event.topic);
    }

    pretty_assertions::assert_eq!(
        topics_seen,
        Vec::from_iter([
            Some(Uri::try_from("com.battler.battle.abcd.start").unwrap()),
            Some(Uri::try_from("com.battler.battle.abcd.update").unwrap()),
            Some(Uri::try_from("com.battler.battle.abcd.update").unwrap()),
            Some(Uri::try_from("com.battler.battle.abcd.update").unwrap()),
            Some(Uri::try_from("com.battler.battle.abcd.end").unwrap()),
        ])
    );
}

#[tokio::test]
async fn publish_matches_subscription_by_wildcard() {
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
        .subscribe_with_options(
            WildcardUri::try_from("com.battler.battle..start").unwrap(),
            SubscriptionOptions {
                match_style: Some(MatchStyle::Wildcard),
            },
        )
        .await
        .unwrap();

    assert_matches::assert_matches!(
        publisher
            .publish(
                Uri::try_from("com.battler.battle.battle1.start").unwrap(),
                PublishedEvent::default(),
            )
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        publisher
            .publish(
                Uri::try_from("com.battler.battle.battle1.update").unwrap(),
                PublishedEvent::default(),
            )
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        publisher
            .publish(
                Uri::try_from("com.battler.battle.battle2.start").unwrap(),
                PublishedEvent::default(),
            )
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        publisher
            .publish(
                Uri::try_from("com.battler.battle.battle3.start").unwrap(),
                PublishedEvent::default(),
            )
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        publisher
            .publish(
                Uri::try_from("com.battler.battle.battle2.update").unwrap(),
                PublishedEvent::default(),
            )
            .await,
        Ok(())
    );

    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_matches::assert_matches!(subscriber.unsubscribe(subscription.id).await, Ok(()));

    let mut topics_seen = Vec::default();
    while let Ok(event) = subscription.event_rx.recv().await {
        topics_seen.push(event.topic);
    }

    pretty_assertions::assert_eq!(
        topics_seen,
        Vec::from_iter([
            Some(Uri::try_from("com.battler.battle.battle1.start").unwrap()),
            Some(Uri::try_from("com.battler.battle.battle2.start").unwrap()),
            Some(Uri::try_from("com.battler.battle.battle3.start").unwrap()),
        ])
    );
}

#[tokio::test]
async fn publish_matches_subscription_by_wildcard_prefix() {
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
        .subscribe_with_options(
            WildcardUri::try_from("com.battler.battle..").unwrap(),
            SubscriptionOptions {
                match_style: Some(MatchStyle::Wildcard),
            },
        )
        .await
        .unwrap();

    assert_matches::assert_matches!(
        publisher
            .publish(
                Uri::try_from("com.battler.battle.battle1.start").unwrap(),
                PublishedEvent::default(),
            )
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        publisher
            .publish(
                Uri::try_from("com.battler.battle.battle2.update").unwrap(),
                PublishedEvent::default(),
            )
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        publisher
            .publish(
                Uri::try_from("com.battler.battle.battle3.end").unwrap(),
                PublishedEvent::default(),
            )
            .await,
        Ok(())
    );

    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_matches::assert_matches!(subscriber.unsubscribe(subscription.id).await, Ok(()));

    let mut topics_seen = Vec::default();
    while let Ok(event) = subscription.event_rx.recv().await {
        topics_seen.push(event.topic);
    }

    pretty_assertions::assert_eq!(
        topics_seen,
        Vec::from_iter([
            Some(Uri::try_from("com.battler.battle.battle1.start").unwrap()),
            Some(Uri::try_from("com.battler.battle.battle2.update").unwrap()),
            Some(Uri::try_from("com.battler.battle.battle3.end").unwrap()),
        ])
    );
}

mod subscription_wildcard_match_test {
    use std::time::Duration;

    use battler_wamp::{
        core::{
            match_style::MatchStyle,
            uri::{
                Uri,
                WildcardUri,
            },
        },
        peer::{
            Peer,
            PublishedEvent,
            Subscription,
            SubscriptionOptions,
        },
    };
    use tokio::{
        sync::broadcast,
        task::JoinHandle,
    };

    use crate::{
        create_peer,
        start_router,
        REALM,
    };

    async fn subscribe_that_expects_event<S>(
        peer: &Peer<S>,
        uri: WildcardUri,
        match_style: Option<MatchStyle>,
        cancel_rx: broadcast::Receiver<()>,
    ) -> JoinHandle<()>
    where
        S: Send + 'static,
    {
        let subscription = peer
            .subscribe_with_options(uri.clone(), SubscriptionOptions { match_style })
            .await
            .unwrap();

        async fn handler(
            mut subscription: Subscription,
            uri: WildcardUri,
            mut cancel_rx: broadcast::Receiver<()>,
        ) {
            loop {
                tokio::select! {
                    event = subscription.event_rx.recv() => {
                        match event {
                            Ok(_) => {
                                return;
                            }
                            _ => (),
                        }
                    }
                    _ = cancel_rx.recv() => {
                        panic!("no event received for {uri}");
                    }
                }
            }
        }

        tokio::spawn(handler(subscription, uri, cancel_rx))
    }

    async fn subscribe_that_expects_no_event<S>(
        peer: &Peer<S>,
        uri: WildcardUri,
        match_style: Option<MatchStyle>,
        cancel_rx: broadcast::Receiver<()>,
    ) -> JoinHandle<()>
    where
        S: Send + 'static,
    {
        let subscription = peer
            .subscribe_with_options(uri.clone(), SubscriptionOptions { match_style })
            .await
            .unwrap();

        async fn handler(mut subscription: Subscription, mut cancel_rx: broadcast::Receiver<()>) {
            loop {
                tokio::select! {
                    event = subscription.event_rx.recv() => {
                        match event {
                            Ok(_) => {
                                panic!("unexpected event {event:?}");
                            }
                            _ => (),
                        }
                    }
                    _ = cancel_rx.recv() => {
                        return;
                    }
                }
            }
        }

        tokio::spawn(handler(subscription, cancel_rx))
    }

    #[tokio::test]
    async fn uses_exact_match() {
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

        let (cancel_tx, cancel_rx) = broadcast::channel(16);
        let handles = Vec::from_iter([subscribe_that_expects_event(
            &subscriber,
            WildcardUri::try_from("a1.b2.c3.d4.e55").unwrap(),
            None,
            cancel_rx.resubscribe(),
        )
        .await]);

        assert_matches::assert_matches!(
            publisher
                .publish(
                    Uri::try_from("a1.b2.c3.d4.e55").unwrap(),
                    PublishedEvent::default()
                )
                .await,
            Ok(_)
        );

        // A small delay to ensure published events are received by the peer.
        tokio::time::sleep(Duration::from_millis(200)).await;
        cancel_tx.send(()).unwrap();
        for result in futures_util::future::join_all(handles).await {
            assert_matches::assert_matches!(result, Ok(()));
        }
    }

    #[tokio::test]
    async fn uses_single_prefix_match() {
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

        let (cancel_tx, cancel_rx) = broadcast::channel(16);
        let handles = Vec::from_iter([
            subscribe_that_expects_no_event(
                &subscriber,
                WildcardUri::try_from("a1.b2.c3.d4.e55").unwrap(),
                None,
                cancel_rx.resubscribe(),
            )
            .await,
            subscribe_that_expects_event(
                &subscriber,
                WildcardUri::try_from("a1.b2.c3").unwrap(),
                Some(MatchStyle::Prefix),
                cancel_rx.resubscribe(),
            )
            .await,
        ]);

        assert_matches::assert_matches!(
            publisher
                .publish(
                    Uri::try_from("a1.b2.c3.d98.e74").unwrap(),
                    PublishedEvent::default()
                )
                .await,
            Ok(_)
        );

        // A small delay to ensure published events are received by the peer.
        tokio::time::sleep(Duration::from_millis(200)).await;
        cancel_tx.send(()).unwrap();
        for result in futures_util::future::join_all(handles).await {
            assert_matches::assert_matches!(result, Ok(()));
        }
    }

    #[tokio::test]
    async fn uses_multiple_prefix_matches() {
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

        let (cancel_tx, cancel_rx) = broadcast::channel(16);
        let handles = Vec::from_iter([
            subscribe_that_expects_event(
                &subscriber,
                WildcardUri::try_from("a1.b2.c3").unwrap(),
                Some(MatchStyle::Prefix),
                cancel_rx.resubscribe(),
            )
            .await,
            subscribe_that_expects_event(
                &subscriber,
                WildcardUri::try_from("a1.b2.c3.d4").unwrap(),
                Some(MatchStyle::Prefix),
                cancel_rx.resubscribe(),
            )
            .await,
        ]);

        assert_matches::assert_matches!(
            publisher
                .publish(
                    Uri::try_from("a1.b2.c3.d4.325").unwrap(),
                    PublishedEvent::default()
                )
                .await,
            Ok(_)
        );

        // A small delay to ensure published events are received by the peer.
        tokio::time::sleep(Duration::from_millis(200)).await;
        cancel_tx.send(()).unwrap();
        for result in futures_util::future::join_all(handles).await {
            assert_matches::assert_matches!(result, Ok(()));
        }
    }

    #[tokio::test]
    async fn uses_wildcard_match() {
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

        let (cancel_tx, cancel_rx) = broadcast::channel(16);
        let handles = Vec::from_iter([
            subscribe_that_expects_no_event(
                &subscriber,
                WildcardUri::try_from("a1.b2.c3.d4").unwrap(),
                Some(MatchStyle::Prefix),
                cancel_rx.resubscribe(),
            )
            .await,
            subscribe_that_expects_event(
                &subscriber,
                WildcardUri::try_from("a1.b2..d4.e5").unwrap(),
                Some(MatchStyle::Wildcard),
                cancel_rx.resubscribe(),
            )
            .await,
        ]);

        assert_matches::assert_matches!(
            publisher
                .publish(
                    Uri::try_from("a1.b2.c55.d4.e5").unwrap(),
                    PublishedEvent::default()
                )
                .await,
            Ok(_)
        );

        // A small delay to ensure published events are received by the peer.
        tokio::time::sleep(Duration::from_millis(200)).await;
        cancel_tx.send(()).unwrap();
        for result in futures_util::future::join_all(handles).await {
            assert_matches::assert_matches!(result, Ok(()));
        }
    }

    #[tokio::test]
    async fn uses_all_match_types_at_the_same_time() {
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

        let (cancel_tx, cancel_rx) = broadcast::channel(16);
        let handles = Vec::from_iter([
            subscribe_that_expects_event(
                &subscriber,
                WildcardUri::try_from("a1.b2.c3.d4.e5").unwrap(),
                None,
                cancel_rx.resubscribe(),
            )
            .await,
            subscribe_that_expects_event(
                &subscriber,
                WildcardUri::try_from("a1.b2.c3.d4").unwrap(),
                Some(MatchStyle::Prefix),
                cancel_rx.resubscribe(),
            )
            .await,
            subscribe_that_expects_event(
                &subscriber,
                WildcardUri::try_from("a1.b2..d4.e5").unwrap(),
                Some(MatchStyle::Wildcard),
                cancel_rx.resubscribe(),
            )
            .await,
            subscribe_that_expects_event(
                &subscriber,
                WildcardUri::try_from("a1....e5").unwrap(),
                Some(MatchStyle::Wildcard),
                cancel_rx.resubscribe(),
            )
            .await,
        ]);

        assert_matches::assert_matches!(
            publisher
                .publish(
                    Uri::try_from("a1.b2.c3.d4.e5").unwrap(),
                    PublishedEvent::default()
                )
                .await,
            Ok(_)
        );

        // A small delay to ensure published events are received by the peer.
        tokio::time::sleep(Duration::from_millis(200)).await;
        cancel_tx.send(()).unwrap();
        for result in futures_util::future::join_all(handles).await {
            assert_matches::assert_matches!(result, Ok(()));
        }
    }
}
