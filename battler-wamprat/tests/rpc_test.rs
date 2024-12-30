use anyhow::Result;
use async_trait::async_trait;
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
use battler_wamprat::{
    peer::{
        PeerBuilder,
        PeerConnectionType,
    },
    procedure::TypedProcedure,
};
use battler_wamprat_schema::{
    Integer,
    WampApplicationMessage,
    WampList,
};

const REALM: &str = "com.battler.test";

async fn start_router(port: u16) -> Result<RouterHandle> {
    let mut config = RouterConfig::default();
    // Must use a stable port for reconnection.
    config.port = port;
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

fn create_peer(name: &str) -> Result<WebSocketPeer> {
    let mut config = PeerConfig::default();
    config.name = name.to_owned();
    new_web_socket_peer(config)
}

#[derive(WampList)]
struct AddArgs {
    a: Integer,
    b: Integer,
}

#[derive(WampApplicationMessage)]
struct AddInput {
    #[arguments]
    args: AddArgs,
}

#[derive(Debug, PartialEq, WampList)]
struct SumArgs {
    sum: Integer,
}

#[derive(Debug, PartialEq, WampApplicationMessage)]
struct AddOutput {
    #[arguments]
    args: SumArgs,
}

struct AddHandler;

#[async_trait]
impl TypedProcedure for AddHandler {
    type Input = AddInput;
    type Output = AddOutput;
    async fn invoke(&self, input: Self::Input) -> Result<Self::Output> {
        let sum = input.args.a + input.args.b;
        Ok(AddOutput {
            args: SumArgs { sum },
        })
    }
}

#[tokio::test]
async fn registers_methods_on_start() {
    test_utils::setup::setup_test_environment();

    // Start a router.
    let router_handle = start_router(0).await.unwrap();

    // Create a callee with that exposes an addition RPC.
    let mut peer_builder = PeerBuilder::new(PeerConnectionType::Remote(format!(
        "ws://{}",
        router_handle.local_addr()
    )));
    peer_builder.add_typed_procedure(Uri::try_from("com.battler.add2").unwrap(), AddHandler);
    let callee_handle = peer_builder.start(
        create_peer("callee").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );

    // Must wait until the procedure is registered, since it happens asynchronously.
    callee_handle.wait_until_ready().await.unwrap();

    // Create a caller.
    let caller_handle = PeerBuilder::new(PeerConnectionType::Remote(format!(
        "ws://{}",
        router_handle.local_addr()
    )))
    .start(
        create_peer("caller").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );

    // Call the addition RPC.
    assert_matches::assert_matches!(caller_handle.call::<AddInput, AddOutput>(
        Uri::try_from("com.battler.add2").unwrap(),
        AddInput { args: AddArgs { a: 2, b: 2 }},
    ).await, Ok(output) => {
        pretty_assertions::assert_eq!(output, AddOutput { args: SumArgs { sum: 4 }});
    });
    assert_matches::assert_matches!(caller_handle.call::<AddInput, AddOutput>(
        Uri::try_from("com.battler.add2").unwrap(),
        AddInput { args: AddArgs { a: 12, b: 34 }},
    ).await, Ok(output) => {
        pretty_assertions::assert_eq!(output, AddOutput { args: SumArgs { sum: 46 }});
    });
    assert_matches::assert_matches!(caller_handle.call::<AddInput, AddOutput>(
        Uri::try_from("com.battler.add2").unwrap(),
        AddInput { args: AddArgs { a: 2024, b: 1000 }},
    ).await, Ok(output) => {
        pretty_assertions::assert_eq!(output, AddOutput { args: SumArgs { sum: 3024 }});
    });

    // Clean up everything.

    caller_handle.cancel().unwrap();
    caller_handle.join().await.unwrap();

    callee_handle.cancel().unwrap();
    callee_handle.join().await.unwrap();

    router_handle.cancel().unwrap();
    router_handle.join().await.unwrap();
}

#[tokio::test]
async fn registers_methods_on_reconnect() {
    test_utils::setup::setup_test_environment();

    // Start a router.
    let router_handle = start_router(8888).await.unwrap();

    // Create a callee with that exposes an addition RPC.
    let mut peer_builder = PeerBuilder::new(PeerConnectionType::Remote(format!(
        "ws://{}",
        router_handle.local_addr()
    )));
    peer_builder.add_typed_procedure(Uri::try_from("com.battler.add2").unwrap(), AddHandler);
    let callee_handle = peer_builder.start(
        create_peer("callee").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );
    callee_handle.wait_until_ready().await.unwrap();

    // Stop the router to disconnect the peer.
    router_handle.cancel().unwrap();
    router_handle.join().await.unwrap();

    let router_handle = start_router(8888).await.unwrap();

    // Wait again.
    callee_handle.wait_until_ready().await.unwrap();

    let caller_handle = PeerBuilder::new(PeerConnectionType::Remote(format!(
        "ws://{}",
        router_handle.local_addr()
    )))
    .start(
        create_peer("caller").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );

    assert_matches::assert_matches!(caller_handle.call::<AddInput, AddOutput>(
        Uri::try_from("com.battler.add2").unwrap(),
        AddInput { args: AddArgs { a: 12, b: 34 }},
    ).await, Ok(output) => {
        pretty_assertions::assert_eq!(output, AddOutput { args: SumArgs { sum: 46 }});
    });

    caller_handle.cancel().unwrap();
    caller_handle.join().await.unwrap();

    callee_handle.cancel().unwrap();
    callee_handle.join().await.unwrap();

    router_handle.cancel().unwrap();
    router_handle.join().await.unwrap();
}
