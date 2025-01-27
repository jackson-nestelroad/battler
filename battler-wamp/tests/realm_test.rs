use anyhow::Result;
use battler_wamp::{
    core::{
        error::InteractionError,
        uri::Uri,
    },
    peer::{
        PeerConfig,
        WebSocketPeer,
        new_web_socket_peer,
    },
    router::{
        EmptyPubSubPolicies,
        EmptyRpcPolicies,
        RealmConfig,
        RouterConfig,
        RouterHandle,
        new_web_socket_router,
    },
};
use tokio::task::JoinHandle;

const REALM: &str = "com.battler.test";
const OTHER_REALM: &str = "com.battler.other";

async fn start_router() -> Result<(RouterHandle, JoinHandle<()>)> {
    let mut config = RouterConfig::default();
    config.realms.push(RealmConfig {
        name: "test".to_owned(),
        uri: Uri::try_from(REALM)?,
    });
    config.realms.push(RealmConfig {
        name: "other".to_owned(),
        uri: Uri::try_from(OTHER_REALM)?,
    });
    let router = new_web_socket_router(
        config,
        Box::new(EmptyPubSubPolicies::default()),
        Box::new(EmptyRpcPolicies::default()),
    )?;
    router.start().await
}

fn create_peer() -> Result<WebSocketPeer> {
    let config = PeerConfig::default();
    new_web_socket_peer(config)
}

#[tokio::test(flavor = "multi_thread")]
async fn peer_joins_realm() {
    test_utils::setup::setup_test_environment();

    let (router_handle, router_join_handle) = start_router().await.unwrap();
    let peer = create_peer().unwrap();

    // Connect to the router.
    assert_matches::assert_matches!(
        peer.connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );

    // Join realm.
    assert_matches::assert_matches!(peer.join_realm(REALM).await, Ok(()));

    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn peer_reconnects_and_rejoins_realm() {
    test_utils::setup::setup_test_environment();

    let (router_handle, router_join_handle) = start_router().await.unwrap();
    let peer = create_peer().unwrap();

    assert_matches::assert_matches!(
        peer.connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(peer.join_realm(REALM).await, Ok(()));

    // Clean up the first router.
    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();

    // Recreate the router.
    let (router_handle, router_join_handle) = start_router().await.unwrap();

    // The router transitions peers to the closed state, but the message may not be received if the
    // channel closes too soon.
    match peer.join_realm(REALM).await {
        Ok(()) => (),
        Err(err) => {
            // The channel is closed.
            assert_eq!(err.to_string(), "channel closed");

            // Second attempt shows the peer is not connected.
            assert_matches::assert_matches!(peer.join_realm(REALM).await, Err(err) => {
                assert_eq!(err.to_string(), "peer is not connected");
            });

            // Reconnect and rejoin the realm.
            assert_matches::assert_matches!(
                peer.connect(&format!("ws://{}", router_handle.local_addr()))
                    .await,
                Ok(())
            );
            assert_matches::assert_matches!(peer.join_realm(REALM).await, Ok(()));
        }
    }

    // Clean up the second router.
    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn peer_joins_and_leaves_realm() {
    test_utils::setup::setup_test_environment();

    let (router_handle, router_join_handle) = start_router().await.unwrap();
    let peer = create_peer().unwrap();

    assert_matches::assert_matches!(
        peer.connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );

    assert_matches::assert_matches!(peer.join_realm(REALM).await, Ok(()));
    assert_matches::assert_matches!(peer.leave_realm().await, Ok(()));
    assert_matches::assert_matches!(peer.join_realm(REALM).await, Ok(()));
    assert_matches::assert_matches!(peer.leave_realm().await, Ok(()));
    assert_matches::assert_matches!(peer.join_realm(REALM).await, Ok(()));

    // Invalid state transition, so the channel closes.
    assert_matches::assert_matches!(peer.join_realm(REALM).await, Err(err) => {
        assert!(err.to_string().contains("invalid state transition"), "{err}");
    });

    // Second attempt shows the peer is not connected.
    assert_matches::assert_matches!(peer.join_realm(REALM).await, Err(err) => {
        assert!(err.to_string() == "peer is not connected" || err.to_string() == "channel closed", "{err}");
    });

    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn peer_joins_another_realm_after_leaving() {
    test_utils::setup::setup_test_environment();

    let (router_handle, router_join_handle) = start_router().await.unwrap();
    let peer = create_peer().unwrap();

    assert_matches::assert_matches!(
        peer.connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );

    assert_matches::assert_matches!(peer.join_realm(REALM).await, Ok(()));
    assert_matches::assert_matches!(peer.leave_realm().await, Ok(()));
    assert_matches::assert_matches!(peer.join_realm(OTHER_REALM).await, Ok(()));
    assert_matches::assert_matches!(peer.disconnect().await, Ok(()));

    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn peer_cannot_join_missing_realm() {
    test_utils::setup::setup_test_environment();

    let (router_handle, router_join_handle) = start_router().await.unwrap();
    let peer = create_peer().unwrap();

    assert_matches::assert_matches!(
        peer.connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );

    // There is a race condition between the router aborting the connection due to the error and the
    // peer processing the ABORT message.
    //
    // There are two ways to fix this:
    //  1. The router should not end connections after ABORTED messages.
    //  2. The peer should keep session transmission channels alive while using them.
    //
    // The latter option makes the peer more resilient to abrupt connection loss in general.
    assert_matches::assert_matches!(peer.join_realm("unknown").await, Err(err) => {
        match err.downcast::<InteractionError>() {
            Ok(err) => assert_matches::assert_matches!(err, InteractionError::NoSuchRealm),
            Err(err) => assert_eq!(err.to_string(), "channel closed"),
        }
    });

    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}
