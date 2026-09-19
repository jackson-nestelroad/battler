use std::time::Duration;

use anyhow::Result;
use battler_wamp::{
    peer::{
        PeerConfig,
        RpcCall,
        WebSocketPeer,
        new_web_socket_peer,
    },
    router::{
        EmptyConnectionPolicies,
        EmptyPubSubPolicies,
        EmptyRpcPolicies,
        RealmAuthenticationConfig,
        RealmConfig,
        RouterConfig,
        RouterHandle,
        RouterLimitsConfig,
        new_web_socket_router,
    },
};
use battler_wamp_uri::Uri;
use tokio::{
    io::{
        AsyncReadExt,
        AsyncWriteExt,
    },
    net::TcpStream,
    task::JoinHandle,
};

const REALM: &str = "com.battler.test";

async fn start_router_with_limits(
    limits: RouterLimitsConfig,
) -> Result<(RouterHandle, JoinHandle<()>)> {
    let mut config = RouterConfig {
        limits,
        ..Default::default()
    };
    config.realms.push(RealmConfig {
        name: "test".to_owned(),
        uri: Uri::try_from(REALM)?,
        authentication: RealmAuthenticationConfig::default(),
    });
    let router = new_web_socket_router(
        config,
        Box::new(EmptyConnectionPolicies),
        Box::new(EmptyPubSubPolicies),
        Box::new(EmptyRpcPolicies),
    )?;
    router.start().await
}

fn create_peer() -> Result<WebSocketPeer> {
    let config = PeerConfig::default();
    new_web_socket_peer(config)
}

#[tokio::test(flavor = "multi_thread")]
async fn global_connection_limit_enforces_and_reclaims() {
    test_utils::setup::setup_test_environment();

    let limits = RouterLimitsConfig {
        max_connections: Some(2),
        ..Default::default()
    };

    let (router_handle, router_join_handle) = start_router_with_limits(limits).await.unwrap();
    let addr = format!("ws://{}", router_handle.local_addr());

    let peer1 = create_peer().unwrap();
    let peer2 = create_peer().unwrap();
    let peer3 = create_peer().unwrap();

    // First 2 connect successfully
    assert!(peer1.connect(&addr).await.is_ok());
    assert!(peer2.connect(&addr).await.is_ok());

    // 3rd is rejected due to global limit
    let res3 = peer3.connect(&addr).await;
    assert!(
        res3.is_err(),
        "Expected 3rd connection to be rejected by global connection limit"
    );

    // Disconnecting peer1 frees a slot
    assert!(peer1.disconnect().await.is_ok());
    tokio::time::sleep(Duration::from_millis(50)).await;

    // Now peer3 can connect successfully
    assert!(
        peer3.connect(&addr).await.is_ok(),
        "Expected peer3 to connect after peer1 disconnected"
    );

    peer2.disconnect().await.ok();
    peer3.disconnect().await.ok();
    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn per_ip_connection_limit_enforces_when_not_exempt() {
    test_utils::setup::setup_test_environment();

    let limits = RouterLimitsConfig {
        max_connections_per_ip: Some(2),
        exempt_loopback: false, // Disable exemption to test loopback address enforcement
        ..Default::default()
    };

    let (router_handle, router_join_handle) = start_router_with_limits(limits).await.unwrap();
    let addr = format!("ws://{}", router_handle.local_addr());

    let peer1 = create_peer().unwrap();
    let peer2 = create_peer().unwrap();
    let peer3 = create_peer().unwrap();

    assert!(peer1.connect(&addr).await.is_ok());
    assert!(peer2.connect(&addr).await.is_ok());

    // 3rd from 127.0.0.1 is rejected because limit per IP is 2
    let res3 = peer3.connect(&addr).await;
    assert!(
        res3.is_err(),
        "Expected 3rd connection from same IP to be rejected"
    );

    // Disconnect peer1
    assert!(peer1.disconnect().await.is_ok());
    tokio::time::sleep(Duration::from_millis(50)).await;

    // peer3 can now connect
    assert!(peer3.connect(&addr).await.is_ok());

    peer2.disconnect().await.ok();
    peer3.disconnect().await.ok();
    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn loopback_exemption_allows_multiple_local_connections() {
    test_utils::setup::setup_test_environment();

    let limits = RouterLimitsConfig {
        max_connections_per_ip: Some(1),
        exempt_loopback: true, // Default behavior: loopback exempt
        ..Default::default()
    };

    let (router_handle, router_join_handle) = start_router_with_limits(limits).await.unwrap();
    let addr = format!("ws://{}", router_handle.local_addr());

    let peer1 = create_peer().unwrap();
    let peer2 = create_peer().unwrap();
    let peer3 = create_peer().unwrap();

    // All 3 succeed because loopback is exempt from the per-IP cap of 1
    assert!(peer1.connect(&addr).await.is_ok());
    assert!(peer2.connect(&addr).await.is_ok());
    assert!(peer3.connect(&addr).await.is_ok());

    peer1.disconnect().await.ok();
    peer2.disconnect().await.ok();
    peer3.disconnect().await.ok();
    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn message_rate_limiting_terminates_flooding_peer() {
    test_utils::setup::setup_test_environment();

    let limits = RouterLimitsConfig {
        rate_limit_burst: 3,
        rate_limit_refill_per_sec: 1,
        ..Default::default()
    };

    let (router_handle, router_join_handle) = start_router_with_limits(limits).await.unwrap();
    let addr = format!("ws://{}", router_handle.local_addr());

    let peer = create_peer().unwrap();
    peer.connect(&addr).await.unwrap();
    peer.join_realm(REALM).await.unwrap();

    let mut finished_rx = peer.connection_finished_rx();

    // Blast procedure calls rapidly to trigger rate limiting
    for _ in 0..20 {
        let _ = peer
            .call(
                Uri::try_from("com.battler.nonexistent").unwrap(),
                RpcCall::default(),
            )
            .await;
    }

    // Peer should receive disconnection/finish signal due to rate limit violation
    let finished_res = tokio::time::timeout(Duration::from_secs(2), finished_rx.recv()).await;
    assert!(
        finished_res.is_ok(),
        "Expected peer connection to finish after exceeding rate limit"
    );

    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn handshake_timeout_terminates_slowloris_connection() {
    test_utils::setup::setup_test_environment();

    let limits = RouterLimitsConfig {
        handshake_timeout: Duration::from_millis(200),
        ..Default::default()
    };

    let (router_handle, router_join_handle) = start_router_with_limits(limits).await.unwrap();

    // Connect raw TCP socket and do NOT send HTTP upgrade request
    let mut tcp_stream = TcpStream::connect(router_handle.local_addr())
        .await
        .unwrap();

    // Send 1 partial byte to simulate Slowloris
    tcp_stream.write_all(b"G").await.unwrap();

    // Wait for server handshake timeout to fire
    let mut buf = [0u8; 128];
    let read_res = tokio::time::timeout(Duration::from_secs(2), tcp_stream.read(&mut buf)).await;

    assert!(
        read_res.is_ok(),
        "Expected read to complete after server closes connection"
    );
    let bytes_read = read_res.unwrap().unwrap_or(0);
    assert_eq!(
        bytes_read, 0,
        "Expected EOF (0 bytes) indicating server severed the connection"
    );

    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn idle_timeout_terminates_inactive_connection() {
    test_utils::setup::setup_test_environment();

    let limits = RouterLimitsConfig {
        idle_timeout: Some(Duration::from_millis(250)),
        ..Default::default()
    };

    let (router_handle, router_join_handle) = start_router_with_limits(limits).await.unwrap();
    let addr = format!("ws://{}", router_handle.local_addr());

    let peer = create_peer().unwrap();
    peer.connect(&addr).await.unwrap();
    let mut finished_rx = peer.connection_finished_rx();

    // Sleep longer than idle timeout without sending anything
    let finished_res = tokio::time::timeout(Duration::from_secs(2), finished_rx.recv()).await;
    assert!(
        finished_res.is_ok(),
        "Expected peer connection to be closed by idle timeout"
    );

    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}
