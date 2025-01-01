use anyhow::Result;
use async_trait::async_trait;
use battler_wamp::{
    core::{
        error::WampError,
        uri::Uri,
    },
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
use battler_wamp_values::{
    Integer,
    WampList,
};
use battler_wamprat::{
    peer::{
        PeerBuilder,
        PeerConnectionType,
        PeerHandle,
    },
    procedure::TypedProcedure,
};
use battler_wamprat_message::WampApplicationMessage;
use tokio::task::JoinHandle;

const REALM: &str = "com.battler.test";

async fn start_router(port: u16) -> Result<(RouterHandle, JoinHandle<()>)> {
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
    router.start().await
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
    type Error = anyhow::Error;
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
    let (router_handle, router_join_handle) = start_router(0).await.unwrap();

    // Create a callee with that exposes an addition RPC.
    let mut peer_builder = PeerBuilder::new(PeerConnectionType::Remote(format!(
        "ws://{}",
        router_handle.local_addr()
    )));
    peer_builder.add_procedure_typed(Uri::try_from("com.battler.add2").unwrap(), AddHandler);
    let (callee_handle, callee_join_handle) = peer_builder.start(
        create_peer("callee").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );

    // Must wait until the procedure is registered, since it happens asynchronously.
    callee_handle.wait_until_ready().await.unwrap();

    // Create a caller.
    let (caller_handle, caller_join_handle) = PeerBuilder::new(PeerConnectionType::Remote(
        format!("ws://{}", router_handle.local_addr()),
    ))
    .start(
        create_peer("caller").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );

    // Call the addition RPC.
    assert_matches::assert_matches!(caller_handle.call_and_wait::<AddInput, AddOutput>(
        Uri::try_from("com.battler.add2").unwrap(),
        AddInput { args: AddArgs { a: 2, b: 2 }},
    ).await, Ok(output) => {
        pretty_assertions::assert_eq!(output, AddOutput { args: SumArgs { sum: 4 }});
    });
    assert_matches::assert_matches!(caller_handle.call_and_wait::<AddInput, AddOutput>(
        Uri::try_from("com.battler.add2").unwrap(),
        AddInput { args: AddArgs { a: 12, b: 34 }},
    ).await, Ok(output) => {
        pretty_assertions::assert_eq!(output, AddOutput { args: SumArgs { sum: 46 }});
    });
    assert_matches::assert_matches!(caller_handle.call_and_wait::<AddInput, AddOutput>(
        Uri::try_from("com.battler.add2").unwrap(),
        AddInput { args: AddArgs { a: 2024, b: 1000 }},
    ).await, Ok(output) => {
        pretty_assertions::assert_eq!(output, AddOutput { args: SumArgs { sum: 3024 }});
    });

    // Clean up everything.

    caller_handle.cancel().unwrap();
    caller_join_handle.await.unwrap();

    callee_handle.cancel().unwrap();
    callee_join_handle.await.unwrap();

    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}

#[tokio::test]
async fn registers_methods_on_reconnect() {
    test_utils::setup::setup_test_environment();

    // Start a router.
    let (router_handle, router_join_handle) = start_router(8888).await.unwrap();

    // Create a callee with that exposes an addition RPC.
    let mut peer_builder = PeerBuilder::new(PeerConnectionType::Remote(format!(
        "ws://{}",
        router_handle.local_addr()
    )));
    peer_builder.add_procedure_typed(Uri::try_from("com.battler.add2").unwrap(), AddHandler);
    let (callee_handle, callee_join_handle) = peer_builder.start(
        create_peer("callee").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );
    callee_handle.wait_until_ready().await.unwrap();

    // Stop the router to disconnect the peer.
    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();

    // Restart the router.
    let (router_handle, router_join_handle) = start_router(8888).await.unwrap();

    // Wait again, to ensure the method is registered before calling it.
    callee_handle.wait_until_ready().await.unwrap();

    let (caller_handle, caller_join_handle) = PeerBuilder::new(PeerConnectionType::Remote(
        format!("ws://{}", router_handle.local_addr()),
    ))
    .start(
        create_peer("caller").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );

    assert_matches::assert_matches!(caller_handle.call_and_wait::<AddInput, AddOutput>(
        Uri::try_from("com.battler.add2").unwrap(),
        AddInput { args: AddArgs { a: 12, b: 34 }},
    ).await, Ok(output) => {
        pretty_assertions::assert_eq!(output, AddOutput { args: SumArgs { sum: 46 }});
    });

    caller_handle.cancel().unwrap();
    caller_join_handle.await.unwrap();

    callee_handle.cancel().unwrap();
    callee_join_handle.await.unwrap();

    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}

#[tokio::test]
async fn retries_call_during_reconnect() {
    test_utils::setup::setup_test_environment();

    // Start a router.
    let (router_handle, router_join_handle) = start_router(0).await.unwrap();

    // Create a callee with that exposes an addition RPC.
    let mut peer_builder = PeerBuilder::new(PeerConnectionType::Remote(format!(
        "ws://{}",
        router_handle.local_addr()
    )));
    peer_builder.add_procedure_typed(Uri::try_from("com.battler.add2").unwrap(), AddHandler);
    let (callee_handle, callee_join_handle) = peer_builder.start(
        create_peer("callee").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );
    callee_handle.wait_until_ready().await.unwrap();

    let (caller_handle, caller_join_handle) = PeerBuilder::new(PeerConnectionType::Remote(
        format!("ws://{}", router_handle.local_addr()),
    ))
    .start(
        create_peer("caller").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );
    caller_handle.wait_until_ready().await.unwrap();

    async fn call<S>(caller_handle: PeerHandle<S>)
    where
        S: Send + Sync + 'static,
    {
        assert_matches::assert_matches!(
            caller_handle
                .call_and_wait::<AddInput, AddOutput>(
                    Uri::try_from("com.battler.add2").unwrap(),
                    AddInput {
                        args: AddArgs { a: 12, b: 34 },
                    },
                )
                .await,
            Ok(output) => {
                pretty_assertions::assert_eq!(output, AddOutput { args: SumArgs { sum: 46 }});
            }
        );
    }

    let call_handle = tokio::spawn(call(caller_handle.clone()));

    // We cannot restart the whole router because there may be a data race between the callee
    // re-registering its procedure and the caller invoking the procedure.
    router_handle
        .end_session(
            Uri::try_from(REALM).unwrap(),
            caller_handle.current_session_id().await.unwrap(),
        )
        .unwrap();

    call_handle.await.unwrap();

    caller_handle.cancel().unwrap();
    caller_join_handle.await.unwrap();

    callee_handle.cancel().unwrap();
    callee_join_handle.await.unwrap();

    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}

#[tokio::test]
async fn persists_error_data() {
    #[derive(Debug, WampApplicationMessage)]
    struct Input;

    #[derive(Debug, WampApplicationMessage)]
    struct Output;

    #[derive(Debug, PartialEq)]
    struct Error {
        msg: String,
    }

    impl Into<WampError> for Error {
        fn into(self) -> WampError {
            WampError::new(
                Uri::try_from("com.battler.error.forced_error").unwrap(),
                self.msg,
            )
        }
    }

    struct Handler;

    #[async_trait]
    impl TypedProcedure for Handler {
        type Input = Input;
        type Output = Output;
        type Error = Error;
        async fn invoke(&self, _: Self::Input) -> Result<Self::Output, Self::Error> {
            Err(Error {
                msg: "foo bar".to_owned(),
            })
        }
    }

    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router(0).await.unwrap();

    let mut peer_builder = PeerBuilder::new(PeerConnectionType::Remote(format!(
        "ws://{}",
        router_handle.local_addr()
    )));
    peer_builder.add_procedure_typed(Uri::try_from("com.battler.fn").unwrap(), Handler);
    let (callee_handle, _) = peer_builder.start(
        create_peer("callee").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );
    callee_handle.wait_until_ready().await.unwrap();

    let (caller_handle, _) = PeerBuilder::new(PeerConnectionType::Remote(format!(
        "ws://{}",
        router_handle.local_addr()
    )))
    .start(
        create_peer("caller").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );

    assert_matches::assert_matches!(caller_handle.call_and_wait::<Input, Output>(
        Uri::try_from("com.battler.fn").unwrap(),
        Input,
    ).await, Err(err) => {
        assert_matches::assert_matches!(err.downcast::<WampError>(), Ok(err) => {
            assert_eq!(err.reason().as_ref(), "com.battler.error.forced_error");
            assert_eq!(err.message(), "foo bar");
        });
    });
}
