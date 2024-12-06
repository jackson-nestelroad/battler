use anyhow::Error;
use battler_wamp::{
    core::uri::Uri,
    peer::{
        new_web_socket_peer,
        PeerConfig,
        WebSocketPeer,
    },
    router::{
        new_web_socket_router,
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
    let router = new_web_socket_router(config)?;
    let handle = router.start().await?;
    Ok(handle)
}

fn create_peer() -> Result<WebSocketPeer, Error> {
    let config = PeerConfig::default();
    new_web_socket_peer(config)
}

#[tokio::test]
async fn peer_connects_to_router() {
    test_utils::setup::setup_test_environment();

    let router_handle = start_router().await.unwrap();
    let mut peer = create_peer().unwrap();

    // Connect to the router.
    peer.connect(&format!("ws://{}", router_handle.local_addr()))
        .await
        .unwrap();

    // Close the router.
    //
    // We expect that the connection established by the peer ends abruptly, because it is an
    // untracked connection not attached to any realm.
    router_handle.cancel().unwrap();
    router_handle.join().await.unwrap();

    // The channel is closed.
    assert_matches::assert_matches!(peer.join_realm(REALM).await, Err(err) => {
        assert!(err.to_string().contains("channel closed"));
    });
}

#[tokio::test]
async fn peer_joins_realm() {
    test_utils::setup::setup_test_environment();

    let router_handle = start_router().await.unwrap();
    let mut peer = create_peer().unwrap();

    // Connect to the router.
    peer.connect(&format!("ws://{}", router_handle.local_addr()))
        .await
        .unwrap();

    // Join realm.
    assert_matches::assert_matches!(peer.join_realm(REALM).await, Ok(()));

    router_handle.cancel().unwrap();
    router_handle.join().await.unwrap();
}

#[tokio::test]
async fn peer_reconnects_and_rejoins_realm() {
    test_utils::setup::setup_test_environment();

    let router_handle = start_router().await.unwrap();
    let mut peer = create_peer().unwrap();

    peer.connect(&format!("ws://{}", router_handle.local_addr()))
        .await
        .unwrap();
    assert_matches::assert_matches!(peer.join_realm(REALM).await, Ok(()));

    // Clean up the first router.
    router_handle.cancel().unwrap();
    router_handle.join().await.unwrap();

    // Recreate the router.
    let router_handle = start_router().await.unwrap();

    // The channel is closed.
    assert_matches::assert_matches!(peer.join_realm(REALM).await, Err(err) => {
        assert!(err.to_string().contains("channel closed"));
    });

    // Second attempt shows the peer is not connected.
    assert_matches::assert_matches!(peer.join_realm(REALM).await, Err(err) => {
        assert!(err.to_string().contains("peer is not connected"));
    });

    // Reconnect and rejoin the realm.
    peer.connect(&format!("ws://{}", router_handle.local_addr()))
        .await
        .unwrap();
    assert_matches::assert_matches!(peer.join_realm(REALM).await, Ok(()));

    // Clean up the second router.
    router_handle.cancel().unwrap();
    router_handle.join().await.unwrap();
}
