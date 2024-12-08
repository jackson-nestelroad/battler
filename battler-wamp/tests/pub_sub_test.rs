use anyhow::Error;
use battler_wamp::{
    core::{
        hash::HashSet,
        types::{
            Dictionary,
            Integer,
            List,
            Value,
        },
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
        RealmConfig,
        RouterConfig,
        RouterHandle,
    },
};

const REALM: &str = "com.battler.test";

async fn start_router() -> Result<RouterHandle, Error> {
    let mut config = RouterConfig::default();
    config.realms.push(RealmConfig {
        name: "test".to_owned(),
        uri: Uri::try_from(REALM)?,
    });
    let router = new_web_socket_router(config, Box::new(EmptyPubSubPolicies::default()))?;
    let handle = router.start().await?;
    Ok(handle)
}

fn create_peer(agent: &str) -> Result<WebSocketPeer, Error> {
    let mut config = PeerConfig::default();
    config.agent = agent.to_owned();
    new_web_socket_peer(config)
}

#[tokio::test]
async fn peer_receives_published_messages_for_topic() {
    test_utils::setup::setup_test_environment();

    let router_handle = start_router().await.unwrap();
    let mut publisher = create_peer("publisher").unwrap();
    let mut subscriber = create_peer("subscriber").unwrap();

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

    let subscription = subscriber
        .subscribe(Uri::try_from("com.battler.topic1").unwrap())
        .await
        .unwrap();
    let subscription_id = subscription.id;

    async fn receive_events(mut subscription: Subscription) {
        let mut seen = HashSet::default();
        fn done(seen: &HashSet<Integer>) -> bool {
            (0..5).all(|i| seen.contains(&i))
        }
        while !done(&seen) {
            let event = subscription.event_rx.recv().await;
            assert_matches::assert_matches!(event, Ok(event) => {
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
            });
        }

        pretty_assertions::assert_eq!(seen, HashSet::from_iter([0, 1, 2, 3, 4]));
    }

    let subscribe_handle = tokio::spawn(receive_events(subscription));

    for i in 0..6 {
        assert_matches::assert_matches!(
            publisher
                .publish(
                    Uri::try_from("com.battler.topic1").unwrap(),
                    List::from_iter([Value::Integer(i)]),
                    Dictionary::from_iter([("index".to_owned(), Value::Integer(i))]),
                )
                .await,
            Ok(())
        );
    }

    assert_matches::assert_matches!(subscribe_handle.await, Ok(()));

    assert_matches::assert_matches!(subscriber.unsubscribe(subscription_id).await, Ok(()));

    router_handle.cancel().unwrap();
    router_handle.join().await.unwrap();
}
