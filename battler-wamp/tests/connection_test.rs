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
        EmptyPubSubPolicies,
        EmptyRpcPolicies,
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
    let router = new_web_socket_router(
        config,
        Box::new(EmptyPubSubPolicies::default()),
        Box::new(EmptyRpcPolicies::default()),
    )?;
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
    let peer = create_peer().unwrap();

    // Connect to the router.
    assert_matches::assert_matches!(
        peer.connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );

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
async fn peer_reconnects_to_router() {
    test_utils::setup::setup_test_environment();

    let router_handle = start_router().await.unwrap();
    let peer = create_peer().unwrap();

    // Connect to the router.
    assert_matches::assert_matches!(
        peer.connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    // Reconnect.
    assert_matches::assert_matches!(
        peer.connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );

    router_handle.cancel().unwrap();
    router_handle.join().await.unwrap();
}

#[tokio::test]
async fn peer_disconnects_from_router() {
    test_utils::setup::setup_test_environment();

    let router_handle = start_router().await.unwrap();
    let peer = create_peer().unwrap();

    assert_matches::assert_matches!(peer.disconnect().await, Ok(()));

    assert_matches::assert_matches!(
        peer.connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );

    assert_matches::assert_matches!(peer.disconnect().await, Ok(()));
    assert_matches::assert_matches!(
        peer.connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );

    assert_matches::assert_matches!(peer.disconnect().await, Ok(()));

    router_handle.cancel().unwrap();
    router_handle.join().await.unwrap();
}
