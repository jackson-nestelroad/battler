use anyhow::Result;
use battler_wamp::{
    peer::{
        PeerConfig,
        PeerNotConnectedError,
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
use battler_wamp_uri::Uri;
use tokio::task::JoinHandle;

const REALM: &str = "com.battler.test";

async fn start_router() -> Result<(RouterHandle, JoinHandle<()>)> {
    let mut config = RouterConfig::default();
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

fn create_peer() -> Result<WebSocketPeer> {
    let config = PeerConfig::default();
    new_web_socket_peer(config)
}

#[tokio::test(flavor = "multi_thread")]
async fn peer_connects_to_router() {
    test_utils::setup::setup_test_environment();

    let (router_handle, router_join_handle) = start_router().await.unwrap();
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
    router_join_handle.await.unwrap();

    // The peer is not connected.
    assert_matches::assert_matches!(peer.join_realm(REALM).await, Err(err) => {
         assert_matches::assert_matches!(err.downcast::<PeerNotConnectedError>(), Ok(_));
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn peer_reconnects_to_router() {
    test_utils::setup::setup_test_environment();

    let (router_handle, router_join_handle) = start_router().await.unwrap();
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
    router_join_handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn peer_disconnects_from_router() {
    test_utils::setup::setup_test_environment();

    let (router_handle, router_join_handle) = start_router().await.unwrap();
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
    router_join_handle.await.unwrap();
}
