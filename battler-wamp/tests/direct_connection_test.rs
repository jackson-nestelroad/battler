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

fn create_peer() -> Result<WebSocketPeer, Error> {
    let config = PeerConfig::default();
    new_web_socket_peer(config)
}

#[tokio::test]
async fn peer_directly_connects_to_router() {
    test_utils::setup::setup_test_environment();

    let router_handle = start_router().await.unwrap();
    let mut peer = create_peer().unwrap();

    let connection = router_handle.direct_connect();
    assert_matches::assert_matches!(peer.direct_connect(connection.stream()).await, Ok(()));

    assert_matches::assert_matches!(peer.join_realm(REALM).await, Ok(()));
    assert_matches::assert_matches!(peer.leave_realm().await, Ok(()));
    assert_matches::assert_matches!(peer.join_realm(REALM).await, Ok(()));

    router_handle.cancel().unwrap();
    router_handle.join().await.unwrap();
}
