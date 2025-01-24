use std::time::Duration;

use anyhow::Result;
use async_trait::async_trait;
use battler_wamp::{
    core::{
        error::{
            InteractionError,
            WampError,
        },
        invocation_policy::InvocationPolicy,
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
use battler_wamp_values::{
    Integer,
    WampList,
};
use battler_wamprat::{
    peer::{
        CallOptions,
        PeerBuilder,
        PeerConnectionType,
        PeerHandle,
    },
    procedure::{
        ProcedureOptions,
        ProgressReporter,
        TypedPatternMatchedProcedure,
        TypedPatternMatchedProgressiveProcedure,
        TypedProcedure,
        TypedProgressiveProcedure,
    },
};
use battler_wamprat_error::WampError;
use battler_wamprat_message::WampApplicationMessage;
use battler_wamprat_uri::WampUriMatcher;
use thiserror::Error;
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
    peer_builder.add_procedure(Uri::try_from("com.battler.add2").unwrap(), AddHandler);
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
        CallOptions::default(),
    ).await, Ok(output) => {
        pretty_assertions::assert_eq!(output, AddOutput { args: SumArgs { sum: 4 }});
    });
    assert_matches::assert_matches!(caller_handle.call_and_wait::<AddInput, AddOutput>(
        Uri::try_from("com.battler.add2").unwrap(),
        AddInput { args: AddArgs { a: 12, b: 34 }},
        CallOptions::default(),
    ).await, Ok(output) => {
        pretty_assertions::assert_eq!(output, AddOutput { args: SumArgs { sum: 46 }});
    });
    assert_matches::assert_matches!(caller_handle.call_and_wait::<AddInput, AddOutput>(
        Uri::try_from("com.battler.add2").unwrap(),
        AddInput { args: AddArgs { a: 2024, b: 1000 }},
        CallOptions::default(),
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

    let mut peer_builder = PeerBuilder::new(PeerConnectionType::Remote(format!(
        "ws://{}",
        router_handle.local_addr()
    )));
    peer_builder.add_procedure(Uri::try_from("com.battler.add2").unwrap(), AddHandler);
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
        CallOptions::default(),
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

    let mut peer_builder = PeerBuilder::new(PeerConnectionType::Remote(format!(
        "ws://{}",
        router_handle.local_addr()
    )));
    peer_builder.add_procedure(Uri::try_from("com.battler.add2").unwrap(), AddHandler);
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
                    CallOptions::default(),
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
    peer_builder.add_procedure(Uri::try_from("com.battler.fn").unwrap(), Handler);
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
        CallOptions::default(),
    ).await, Err(err) => {
        assert_matches::assert_matches!(err.downcast::<WampError>(), Ok(err) => {
            assert_eq!(err.reason().as_ref(), "com.battler.error.forced_error");
            assert_eq!(err.message(), "foo bar");
        });
    });
}

#[derive(WampApplicationMessage)]
struct UploadInput;

#[derive(Debug, PartialEq, WampList)]
struct UploadOutputArgs {
    percentage: u64,
}

#[derive(Debug, PartialEq, WampApplicationMessage)]
struct UploadOutput(#[arguments] UploadOutputArgs);

struct UploadHandler;

#[async_trait]
impl TypedProgressiveProcedure for UploadHandler {
    type Input = UploadInput;
    type Output = UploadOutput;
    type Error = anyhow::Error;
    async fn invoke<'rpc>(
        &self,
        _: Self::Input,
        progress: ProgressReporter<'rpc, Self::Output>,
    ) -> Result<Self::Output> {
        progress(UploadOutput(UploadOutputArgs { percentage: 33 }))?;
        progress(UploadOutput(UploadOutputArgs { percentage: 67 }))?;
        Ok(UploadOutput(UploadOutputArgs { percentage: 100 }))
    }
}

#[tokio::test]
async fn calls_with_progressive_results() {
    test_utils::setup::setup_test_environment();

    // Start a router.
    let (router_handle, router_join_handle) = start_router(0).await.unwrap();

    let mut peer_builder = PeerBuilder::new(PeerConnectionType::Remote(format!(
        "ws://{}",
        router_handle.local_addr()
    )));
    peer_builder
        .add_procedure_progressive(Uri::try_from("com.battler.upload").unwrap(), UploadHandler);
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

    let mut rpc = caller_handle
        .call_with_progress::<UploadInput, UploadOutput>(
            Uri::try_from("com.battler.upload").unwrap(),
            UploadInput,
            CallOptions::default(),
        )
        .await
        .unwrap();

    let mut results = Vec::new();
    while let Ok(Some(result)) = rpc.next_result().await {
        results.push(result);
    }

    pretty_assertions::assert_eq!(
        results,
        Vec::from_iter([
            UploadOutput(UploadOutputArgs { percentage: 33 }),
            UploadOutput(UploadOutputArgs { percentage: 67 }),
            UploadOutput(UploadOutputArgs { percentage: 100 }),
        ])
    );

    caller_handle.cancel().unwrap();
    caller_join_handle.await.unwrap();

    callee_handle.cancel().unwrap();
    callee_join_handle.await.unwrap();

    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}

#[derive(WampUriMatcher)]
#[uri("com.battler.upload.{file_type}.v1")]
struct UploadPattern {
    file_type: String,
}

#[derive(Debug, Error, WampError)]
enum UploadError {
    #[error("unsupported file type")]
    #[uri("com.battler_wamprat.test.error.unsupported_file_type")]
    UnsupportedFileType,
}

#[async_trait]
impl TypedPatternMatchedProcedure for UploadHandler {
    type Pattern = UploadPattern;
    type Input = UploadInput;
    type Output = UploadOutput;
    type Error = UploadError;
    async fn invoke(
        &self,
        _: Self::Input,
        procedure: Self::Pattern,
    ) -> Result<Self::Output, Self::Error> {
        if procedure.file_type != "png" {
            return Err(UploadError::UnsupportedFileType);
        }
        Ok(UploadOutput(UploadOutputArgs { percentage: 100 }))
    }
}

#[tokio::test]
async fn calls_pattern_matched_procedure() {
    test_utils::setup::setup_test_environment();

    // Start a router.
    let (router_handle, router_join_handle) = start_router(0).await.unwrap();

    let mut peer_builder = PeerBuilder::new(PeerConnectionType::Remote(format!(
        "ws://{}",
        router_handle.local_addr()
    )));
    peer_builder.add_procedure_pattern_matched(UploadHandler);
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

    assert_matches::assert_matches!(caller_handle.call_and_wait::<UploadInput, UploadOutput>(
        Uri::try_from("com.battler.upload.png.v1").unwrap(),
        UploadInput,
        CallOptions::default(),
    ).await, Ok(result) => {
        pretty_assertions::assert_eq!(result, UploadOutput(UploadOutputArgs { percentage: 100 }));
    });

    assert_matches::assert_matches!(caller_handle.call_and_wait::<UploadInput, UploadOutput>(
        Uri::try_from("com.battler.upload.gif.v1").unwrap(),
        UploadInput,
        CallOptions::default(),
    ).await, Err(err) => {
        assert_matches::assert_matches!(err.downcast::<WampError>(), Ok(err) => {
            assert_eq!(err.reason().as_ref(), "com.battler_wamprat.test.error.unsupported_file_type");
            assert_eq!(err.to_string(), "unsupported file type");
        });
    });

    caller_handle.cancel().unwrap();
    caller_join_handle.await.unwrap();

    callee_handle.cancel().unwrap();
    callee_join_handle.await.unwrap();

    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}

#[async_trait]
impl TypedPatternMatchedProgressiveProcedure for UploadHandler {
    type Pattern = UploadPattern;
    type Input = UploadInput;
    type Output = UploadOutput;
    type Error = UploadError;
    async fn invoke<'rpc>(
        &self,
        _: Self::Input,
        procedure: Self::Pattern,
        progress: ProgressReporter<'rpc, Self::Output>,
    ) -> Result<Self::Output, Self::Error> {
        if procedure.file_type != "png" {
            return Err(UploadError::UnsupportedFileType);
        }
        progress(UploadOutput(UploadOutputArgs { percentage: 25 })).ok();
        progress(UploadOutput(UploadOutputArgs { percentage: 50 })).ok();
        progress(UploadOutput(UploadOutputArgs { percentage: 75 })).ok();
        Ok(UploadOutput(UploadOutputArgs { percentage: 100 }))
    }
}

#[tokio::test]
async fn calls_pattern_matched_procedure_with_progressive_results() {
    test_utils::setup::setup_test_environment();

    // Start a router.
    let (router_handle, router_join_handle) = start_router(0).await.unwrap();

    let mut peer_builder = PeerBuilder::new(PeerConnectionType::Remote(format!(
        "ws://{}",
        router_handle.local_addr()
    )));
    peer_builder.add_procedure_pattern_matched_progressive(UploadHandler);
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

    let mut rpc = caller_handle
        .call_with_progress::<UploadInput, UploadOutput>(
            UploadPattern {
                file_type: "png".to_owned(),
            }
            .wamp_generate_uri()
            .unwrap(),
            UploadInput,
            CallOptions::default(),
        )
        .await
        .unwrap();

    let mut results = Vec::new();
    while let Ok(Some(result)) = rpc.next_result().await {
        results.push(result);
    }

    pretty_assertions::assert_eq!(
        results,
        Vec::from_iter([
            UploadOutput(UploadOutputArgs { percentage: 25 }),
            UploadOutput(UploadOutputArgs { percentage: 50 }),
            UploadOutput(UploadOutputArgs { percentage: 75 }),
            UploadOutput(UploadOutputArgs { percentage: 100 }),
        ])
    );

    let mut rpc = caller_handle
        .call_with_progress::<UploadInput, UploadOutput>(
            UploadPattern {
                file_type: "gif".to_owned(),
            }
            .wamp_generate_uri()
            .unwrap(),
            UploadInput,
            CallOptions::default(),
        )
        .await
        .unwrap();

    assert_matches::assert_matches!(rpc.next_result().await, Err(err) => {
        assert_matches::assert_matches!(err.downcast::<WampError>(), Ok(err) => {
            assert_eq!(err.reason().as_ref(), "com.battler_wamprat.test.error.unsupported_file_type");
            assert_eq!(err.to_string(), "unsupported file type");
        });
    });
    assert_matches::assert_matches!(rpc.next_result().await, Ok(None));

    caller_handle.cancel().unwrap();
    caller_join_handle.await.unwrap();

    callee_handle.cancel().unwrap();
    callee_join_handle.await.unwrap();

    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}

#[tokio::test]
async fn calls_procedure_with_timeout() {
    test_utils::setup::setup_test_environment();

    // Start a router.
    let (router_handle, router_join_handle) = start_router(0).await.unwrap();

    #[derive(WampApplicationMessage)]
    struct Input;

    #[derive(Debug, PartialEq, WampApplicationMessage)]
    struct Output;

    struct StallingHandler;

    #[async_trait]
    impl TypedProgressiveProcedure for StallingHandler {
        type Input = Input;
        type Output = Output;
        type Error = anyhow::Error;
        async fn invoke<'rpc>(
            &self,
            _: Self::Input,
            progress: ProgressReporter<'rpc, Self::Output>,
        ) -> Result<Self::Output> {
            let mut timeout = Duration::from_secs(1);
            loop {
                timeout = timeout + Duration::from_secs(1);
                tokio::time::sleep(timeout).await;
                progress(Output)?;
            }
        }
    }

    let mut peer_builder = PeerBuilder::new(PeerConnectionType::Remote(format!(
        "ws://{}",
        router_handle.local_addr()
    )));
    peer_builder.add_procedure_progressive(
        Uri::try_from("com.battler.upload").unwrap(),
        StallingHandler,
    );
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

    let mut rpc = caller_handle
        .call_with_progress::<Input, Output>(
            Uri::try_from("com.battler.upload").unwrap(),
            Input,
            CallOptions {
                timeout: Some(Duration::from_secs(4)),
            },
        )
        .await
        .unwrap();

    let mut results = Vec::new();
    loop {
        match rpc.next_result().await {
            Ok(Some(result)) => results.push(Ok(result)),
            Ok(None) => break,
            Err(err) => results.push(Err(err)),
        }
    }

    assert_matches::assert_matches!(results.last(), Some(Err(err)) => {
        assert_matches::assert_matches!(err.downcast_ref::<InteractionError>(), Some(InteractionError::Canceled));
    });

    caller_handle.cancel().unwrap();
    caller_join_handle.await.unwrap();

    callee_handle.cancel().unwrap();
    callee_join_handle.await.unwrap();

    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}

#[tokio::test]
async fn call_cancellation_cancels_invocation() {
    test_utils::setup::setup_test_environment();

    // Start a router.
    let (router_handle, router_join_handle) = start_router(0).await.unwrap();

    #[derive(WampApplicationMessage)]
    struct Input;

    #[derive(Debug, PartialEq, WampApplicationMessage)]
    struct Output;

    struct StallingHandler;

    #[async_trait]
    impl TypedProgressiveProcedure for StallingHandler {
        type Input = Input;
        type Output = Output;
        type Error = anyhow::Error;
        async fn invoke<'rpc>(
            &self,
            _: Self::Input,
            progress: ProgressReporter<'rpc, Self::Output>,
        ) -> Result<Self::Output> {
            loop {
                tokio::time::sleep(Duration::ZERO).await;
                progress(Output)?;
            }
        }
    }

    let mut peer_builder = PeerBuilder::new(PeerConnectionType::Remote(format!(
        "ws://{}",
        router_handle.local_addr()
    )));
    peer_builder.add_procedure_progressive(
        Uri::try_from("com.battler.upload").unwrap(),
        StallingHandler,
    );
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

    let mut rpc = caller_handle
        .call_with_progress::<Input, Output>(
            Uri::try_from("com.battler.upload").unwrap(),
            Input,
            CallOptions::default(),
        )
        .await
        .unwrap();

    assert_matches::assert_matches!(rpc.cancel().await, Ok(()));

    let mut results = Vec::new();
    loop {
        match rpc.next_result().await {
            Ok(Some(result)) => results.push(Ok(result)),
            Ok(None) => break,
            Err(err) => results.push(Err(err)),
        }
    }

    assert_matches::assert_matches!(results.last(), Some(Err(err)) => {
        assert_matches::assert_matches!(err.downcast_ref::<InteractionError>(), Some(InteractionError::Canceled));
    });

    caller_handle.cancel().unwrap();
    caller_join_handle.await.unwrap();

    callee_handle.cancel().unwrap();
    callee_join_handle.await.unwrap();

    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}

#[tokio::test]
async fn shared_registration_persists_across_reconnects() {
    test_utils::setup::setup_test_environment();

    // Start a router.
    let (router_handle, router_join_handle) = start_router(0).await.unwrap();

    #[derive(WampApplicationMessage)]
    struct Input;

    #[derive(Debug, PartialEq, WampApplicationMessage)]
    struct Output;

    struct AvailableHandler;

    #[async_trait]
    impl TypedProcedure for AvailableHandler {
        type Input = Input;
        type Output = Output;
        type Error = anyhow::Error;
        async fn invoke(&self, _: Self::Input) -> Result<Self::Output> {
            Ok(Output)
        }

        fn options() -> ProcedureOptions {
            ProcedureOptions {
                invocation_policy: InvocationPolicy::First,
            }
        }
    }

    struct UnavailableHandler;

    #[async_trait]
    impl TypedProcedure for UnavailableHandler {
        type Input = Input;
        type Output = Output;
        type Error = anyhow::Error;
        async fn invoke(&self, _: Self::Input) -> Result<Self::Output> {
            Err(InteractionError::Unavailable.into())
        }

        fn options() -> ProcedureOptions {
            ProcedureOptions {
                invocation_policy: InvocationPolicy::First,
            }
        }
    }

    let mut peer_builder = PeerBuilder::new(PeerConnectionType::Remote(format!(
        "ws://{}",
        router_handle.local_addr()
    )));
    peer_builder.add_procedure(
        Uri::try_from("com.battler.test").unwrap(),
        UnavailableHandler,
    );
    let (callee_1_handle, callee_1_join_handle) = peer_builder.start(
        create_peer("callee").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );
    callee_1_handle.wait_until_ready().await.unwrap();

    let mut peer_builder = PeerBuilder::new(PeerConnectionType::Remote(format!(
        "ws://{}",
        router_handle.local_addr()
    )));
    peer_builder.add_procedure(Uri::try_from("com.battler.test").unwrap(), AvailableHandler);
    let (callee_2_handle, callee_2_join_handle) = peer_builder.start(
        create_peer("callee").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );
    callee_2_handle.wait_until_ready().await.unwrap();

    let (caller_handle, caller_join_handle) = PeerBuilder::new(PeerConnectionType::Remote(
        format!("ws://{}", router_handle.local_addr()),
    ))
    .start(
        create_peer("caller").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );
    caller_handle.wait_until_ready().await.unwrap();

    assert_matches::assert_matches!(
        caller_handle
            .call_and_wait::<Input, Output>(
                Uri::try_from("com.battler.test").unwrap(),
                Input,
                CallOptions::default(),
            )
            .await,
        Ok(_)
    );

    // End the unavailable callee, and force the available callee to reconnect.
    callee_1_handle.cancel().unwrap();
    callee_1_join_handle.await.unwrap();
    router_handle
        .end_session(
            Uri::try_from(REALM).unwrap(),
            callee_2_handle.current_session_id().await.unwrap(),
        )
        .unwrap();
    callee_2_handle.wait_until_ready().await.unwrap();

    assert_matches::assert_matches!(
        caller_handle
            .call_and_wait::<Input, Output>(
                Uri::try_from("com.battler.test").unwrap(),
                Input,
                CallOptions::default(),
            )
            .await,
        Ok(_)
    );

    caller_handle.cancel().unwrap();
    caller_join_handle.await.unwrap();

    callee_2_handle.cancel().unwrap();
    callee_2_join_handle.await.unwrap();

    router_handle.cancel().unwrap();
    router_join_handle.await.unwrap();
}
