use anyhow::Result;
use battler_wamp::{
    core::uri::Uri,
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

async fn start_router() -> Result<(RouterHandle, JoinHandle<()>)> {
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
    router.start().await
}

fn create_peer() -> Result<WebSocketPeer> {
    let config = PeerConfig::default();
    new_web_socket_peer(config)
}

#[tokio::test]
async fn peer_directly_connects_to_router() {
    test_utils::setup::setup_test_environment();

    let (router_handle, router_join_handle) = start_router().await.unwrap();
    let peer = create_peer().unwrap();

    let connection = router_handle.direct_connect();
    assert_matches::assert_matches!(peer.direct_connect(connection.stream()).await, Ok(()));

    assert_matches::assert_matches!(peer.join_realm(REALM).await, Ok(()));
    assert_matches::assert_matches!(peer.leave_realm().await, Ok(()));
    assert_matches::assert_matches!(peer.join_realm(REALM).await, Ok(()));

    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}
