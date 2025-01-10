use std::time::Duration;

use anyhow::{
    Error,
    Result,
};
use battler_wamp::{
    core::{
        error::{
            BasicError,
            InteractionError,
        },
        hash::HashMap,
        id::Id,
        invocation_policy::InvocationPolicy,
        match_style::MatchStyle,
        uri::{
            Uri,
            WildcardUri,
        },
    },
    peer::{
        new_web_socket_peer,
        Invocation,
        PeerConfig,
        PeerNotConnectedError,
        Procedure,
        ProcedureMessage,
        ProcedureOptions,
        RpcCall,
        RpcResult,
        RpcYield,
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
    List,
    Value,
};
use futures_util::{
    future::join_all,
    StreamExt,
};
use tokio::{
    sync::mpsc::{
        unbounded_channel,
        UnboundedReceiver,
        UnboundedSender,
    },
    task::JoinHandle,
};

const REALM: &str = "com.battler.test";

async fn start_router_with_config(
    mut config: RouterConfig,
) -> Result<(RouterHandle, JoinHandle<()>)> {
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

async fn start_router() -> Result<(RouterHandle, JoinHandle<()>)> {
    start_router_with_config(RouterConfig::default()).await
}

fn create_peer_with_config(name: &str, mut config: PeerConfig) -> Result<WebSocketPeer> {
    config.name = name.to_owned();
    new_web_socket_peer(config)
}

fn create_peer(name: &str) -> Result<WebSocketPeer> {
    create_peer_with_config(name, PeerConfig::default())
}

#[tokio::test]
async fn peer_invokes_procedure_from_another_peer() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let caller = create_peer("caller").unwrap();
    let callee = create_peer("callee").unwrap();

    assert_matches::assert_matches!(
        caller
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee.join_realm(REALM).await, Ok(()));

    let procedure = callee
        .register(Uri::try_from("com.battler.add2").unwrap())
        .await
        .unwrap();
    let procedure_id = procedure.id;

    async fn adder(mut procedure: Procedure) {
        fn handle_invocation(invocation: &Invocation) -> Result<Integer> {
            if invocation.arguments.len() != 2 {
                return Err(
                    BasicError::InvalidArgument("invalid number of arguments".to_string()).into(),
                );
            }
            match (&invocation.arguments[0], &invocation.arguments[1]) {
                (Value::Integer(a), Value::Integer(b)) => Ok(a + b),
                _ => Err(BasicError::InvalidArgument("invalid arguments".to_string()).into()),
            }
        }

        while let Ok(ProcedureMessage::Invocation(invocation)) =
            procedure.procedure_message_rx.recv().await
        {
            let result = handle_invocation(&invocation).map(|sum| RpcYield {
                arguments: List::from_iter([Value::Integer(sum)]),
                ..Default::default()
            });
            assert_matches::assert_matches!(invocation.respond(result), Ok(()));
        }
    }

    let adder_handle = tokio::spawn(adder(procedure));

    assert_matches::assert_matches!(
        caller
            .call_and_wait(
                Uri::try_from("com.battler.add2").unwrap(),
                RpcCall::default()
            )
            .await,
        Err(err) => {
            assert_matches::assert_matches!(err.downcast_ref::<BasicError>(), Some(BasicError::InvalidArgument(_)));
            assert_eq!(err.to_string(), "invalid number of arguments");
        }
    );
    assert_matches::assert_matches!(
        caller
            .call_and_wait(
                Uri::try_from("com.battler.add2").unwrap(),
                RpcCall {
                    arguments: List::from_iter([Value::Integer(12), Value::Bool(false)]),
                    ..Default::default()
                }
            )
            .await,
        Err(err) => {
            assert_matches::assert_matches!(err.downcast_ref::<BasicError>(), Some(BasicError::InvalidArgument(_)));
            assert_eq!(err.to_string(), "invalid arguments");
        }
    );
    assert_matches::assert_matches!(
        caller
            .call_and_wait(
                Uri::try_from("com.battler.add2").unwrap(),
                RpcCall {
                    arguments: List::from_iter([Value::Integer(12), Value::Integer(33)]),
                    ..Default::default()
                }
            )
            .await,
        Ok(result) => {
            pretty_assertions::assert_eq!(result, RpcResult {
                arguments: List::from_iter([Value::Integer(45)]),
                ..Default::default()
            });
        }
    );

    assert_matches::assert_matches!(callee.unregister(procedure_id).await, Ok(()));

    assert_matches::assert_matches!(
        caller
            .call_and_wait(
                Uri::try_from("com.battler.add2").unwrap(),
                RpcCall {
                    arguments: List::from_iter([Value::Integer(12), Value::Integer(33)]),
                    ..Default::default()
                }
            )
            .await,
        Err(err) => {
            assert_matches::assert_matches!(err.downcast::<InteractionError>(), Ok(InteractionError::NoSuchProcedure));
        }
    );

    adder_handle.await.unwrap();
}

#[tokio::test]
async fn caller_receives_cancelled_error_when_callee_leaves() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let caller = create_peer("caller").unwrap();
    let callee = create_peer("callee").unwrap();

    assert_matches::assert_matches!(
        caller
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee.join_realm(REALM).await, Ok(()));

    let procedure = callee
        .register(Uri::try_from("com.battler.add2").unwrap())
        .await
        .unwrap();

    async fn handler(callee: WebSocketPeer, mut procedure: Procedure) {
        while let Ok(_) = procedure.procedure_message_rx.recv().await {
            // Leave the realm when we receive an invocation.
            assert_matches::assert_matches!(callee.leave_realm().await, Ok(()));
            return;
        }
    }

    let handler_handle = tokio::spawn(handler(callee, procedure));

    assert_matches::assert_matches!(
        caller.call_and_wait(
            Uri::try_from("com.battler.add2").unwrap(),
            RpcCall::default(),
        ).await,
        Err(err) => {
            assert_matches::assert_matches!(err.downcast::<InteractionError>(), Ok(InteractionError::Canceled));
        }
    );

    handler_handle.await.unwrap();
}

#[tokio::test]
async fn calls_from_same_peer_processed_in_parallel() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let caller = create_peer("caller").unwrap();
    let callee = create_peer("callee").unwrap();

    assert_matches::assert_matches!(
        caller
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee.join_realm(REALM).await, Ok(()));

    let procedure = callee
        .register(Uri::try_from("com.battler.fn").unwrap())
        .await
        .unwrap();

    async fn handler(mut procedure: Procedure) {
        let mut invocations = Vec::new();
        while let Ok(ProcedureMessage::Invocation(invocation)) =
            procedure.procedure_message_rx.recv().await
        {
            invocations.push(invocation);

            // Wait for two invocations.
            if invocations.len() < 2 {
                continue;
            }

            // Respond to invocations at the same time.
            for invocation in invocations {
                let arguments = invocation.arguments.clone();
                assert_matches::assert_matches!(
                    invocation.respond_ok(RpcYield {
                        arguments,
                        ..Default::default()
                    }),
                    Ok(())
                );
            }
            break;
        }
    }

    let handler_handle = tokio::spawn(handler(procedure));

    // Two calls made in parallel.
    let call_1 = caller.call_and_wait(
        Uri::try_from("com.battler.fn").unwrap(),
        RpcCall {
            arguments: List::from_iter([Value::Integer(1)]),
            ..Default::default()
        },
    );
    let call_2 = caller.call_and_wait(
        Uri::try_from("com.battler.fn").unwrap(),
        RpcCall {
            arguments: List::from_iter([Value::Integer(2)]),
            ..Default::default()
        },
    );

    let results = join_all([call_1, call_2]).await;
    assert_eq!(results.len(), 2);
    assert_matches::assert_matches!(&results[0], Ok(result) => {
        pretty_assertions::assert_eq!(*result, RpcResult {
            arguments: List::from_iter([Value::Integer(1)]),
            ..Default::default()
        });
    });
    assert_matches::assert_matches!(&results[1], Ok(result) => {
        pretty_assertions::assert_eq!(*result, RpcResult {
            arguments: List::from_iter([Value::Integer(2)]),
            ..Default::default()
        });
    });
    handler_handle.await.unwrap();
}

#[tokio::test]
async fn peer_cancels_call_immediately() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let caller = create_peer("caller").unwrap();
    let callee = create_peer("callee").unwrap();

    assert_matches::assert_matches!(
        caller
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee.join_realm(REALM).await, Ok(()));

    let procedure = callee
        .register(Uri::try_from("com.battler.fn").unwrap())
        .await
        .unwrap();

    async fn handler(mut procedure: Procedure) {
        loop {
            if let Ok(ProcedureMessage::Interrupt(_)) = procedure.procedure_message_rx.recv().await
            {
                break;
            }
        }
    }

    let handler_handle = tokio::spawn(handler(procedure));

    let rpc = caller
        .call(Uri::try_from("com.battler.fn").unwrap(), RpcCall::default())
        .await
        .unwrap();

    assert_matches::assert_matches!(rpc.cancel().await, Ok(()));
    assert_matches::assert_matches!(rpc.result().await, Err(err) => {
        assert_matches::assert_matches!(err.downcast::<InteractionError>(), Ok(InteractionError::Canceled));
    });

    handler_handle.await.unwrap();
}

#[tokio::test]
async fn peer_cancels_call_after_invocation() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let caller = create_peer("caller").unwrap();
    let callee = create_peer("callee").unwrap();

    assert_matches::assert_matches!(
        caller
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee.join_realm(REALM).await, Ok(()));

    let procedure = callee
        .register(Uri::try_from("com.battler.fn").unwrap())
        .await
        .unwrap();

    let (invocation_received_tx, mut invocation_received_rx) = unbounded_channel();

    async fn handler(mut procedure: Procedure, invocation_received_tx: UnboundedSender<()>) {
        let mut invocation_id = Id::MAX;
        while let Ok(message) = procedure.procedure_message_rx.recv().await {
            match message {
                ProcedureMessage::Invocation(invocation) => {
                    invocation_id = invocation.id();
                    invocation_received_tx.send(()).unwrap();
                }
                ProcedureMessage::Interrupt(interrupt) => {
                    if interrupt.id() == invocation_id {
                        return;
                    }
                }
            }
        }
    }

    let handler_handle = tokio::spawn(handler(procedure, invocation_received_tx));

    let rpc = caller
        .call(Uri::try_from("com.battler.fn").unwrap(), RpcCall::default())
        .await
        .unwrap();

    // Wait for the invocation to be received.
    invocation_received_rx.recv().await;

    assert_matches::assert_matches!(rpc.cancel().await, Ok(()));
    assert_matches::assert_matches!(rpc.result().await, Err(err) => {
        assert_matches::assert_matches!(err.downcast::<InteractionError>(), Ok(InteractionError::Canceled));
    });

    handler_handle.await.unwrap();
}

#[tokio::test]
async fn peer_kills_call_and_gets_result() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let caller = create_peer("caller").unwrap();
    let callee = create_peer("callee").unwrap();

    assert_matches::assert_matches!(
        caller
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee.join_realm(REALM).await, Ok(()));

    let procedure = callee
        .register(Uri::try_from("com.battler.fn").unwrap())
        .await
        .unwrap();

    async fn handler(mut procedure: Procedure) {
        async fn handle_invocation(
            invocation: Invocation,
            mut interrupt_rx: UnboundedReceiver<()>,
        ) {
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(10)) => {
                        invocation.respond_error(Error::msg("timeout")).unwrap();
                        return;
                    }
                    _ = interrupt_rx.recv() => {
                        invocation.respond_ok(RpcYield::default()).unwrap();
                        return;
                    }
                }
            }
        }

        let mut interrupt_txs = HashMap::default();
        while let Ok(message) = procedure.procedure_message_rx.recv().await {
            match message {
                ProcedureMessage::Invocation(invocation) => {
                    let (interrupt_tx, interrupt_rx) = unbounded_channel();
                    interrupt_txs.insert(invocation.id(), interrupt_tx);
                    tokio::spawn(handle_invocation(invocation, interrupt_rx));
                }
                ProcedureMessage::Interrupt(interrupt) => {
                    if let Some(interrupt_tx) = interrupt_txs.get(&interrupt.id()) {
                        interrupt_tx.send(()).unwrap();
                    }
                }
            }
        }
    }

    tokio::spawn(handler(procedure));

    let rpc = caller
        .call(Uri::try_from("com.battler.fn").unwrap(), RpcCall::default())
        .await
        .unwrap();

    // Kill the call and wait for the result, which is successful.
    assert_matches::assert_matches!(rpc.kill().await, Ok(()));
    assert_matches::assert_matches!(rpc.result().await, Ok(_));
}

#[tokio::test]
async fn peer_receives_progressive_call_results() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let caller = create_peer("caller").unwrap();
    let callee = create_peer("callee").unwrap();

    assert_matches::assert_matches!(
        caller
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee.join_realm(REALM).await, Ok(()));

    let procedure = callee
        .register(Uri::try_from("com.battler.fn").unwrap())
        .await
        .unwrap();

    async fn handler(mut procedure: Procedure) {
        async fn handle_invocation(invocation: Invocation) {
            assert_matches::assert_matches!(
                invocation.progress(RpcYield {
                    arguments: List::from_iter([Value::Integer(1)]),
                    ..Default::default()
                }),
                Ok(())
            );
            assert_matches::assert_matches!(
                invocation.progress(RpcYield {
                    arguments: List::from_iter([Value::Integer(2)]),
                    ..Default::default()
                }),
                Ok(())
            );
            assert_matches::assert_matches!(
                invocation.progress(RpcYield {
                    arguments: List::from_iter([Value::Integer(3)]),
                    ..Default::default()
                }),
                Ok(())
            );
            assert_matches::assert_matches!(invocation.respond_ok(RpcYield::default()), Ok(()));
        }

        while let Ok(message) = procedure.procedure_message_rx.recv().await {
            match message {
                ProcedureMessage::Invocation(invocation) => {
                    tokio::spawn(handle_invocation(invocation));
                }
                _ => (),
            }
        }
    }

    tokio::spawn(handler(procedure));

    let rpc = caller
        .call_with_progress(Uri::try_from("com.battler.fn").unwrap(), RpcCall::default())
        .await
        .unwrap();

    let mut stream = rpc.into_stream();
    assert_matches::assert_matches!(stream.next().await, Some(Ok(result)) => {
        pretty_assertions::assert_eq!(result, RpcResult {
            arguments: List::from_iter([Value::Integer(1)]),
            progress: true,
            ..Default::default()
        });
    });
    assert_matches::assert_matches!(stream.next().await, Some(Ok(result)) => {
        pretty_assertions::assert_eq!(result, RpcResult {
            arguments: List::from_iter([Value::Integer(2)]),
            progress: true,
            ..Default::default()
        });
    });
    assert_matches::assert_matches!(stream.next().await, Some(Ok(result)) => {
        pretty_assertions::assert_eq!(result, RpcResult {
            arguments: List::from_iter([Value::Integer(3)]),
            progress: true,
            ..Default::default()
        });
    });
    assert_matches::assert_matches!(stream.next().await, Some(Ok(result)) => {
        pretty_assertions::assert_eq!(result, RpcResult {
            arguments: List::from_iter([]),
            ..Default::default()
        });
    });
    assert_matches::assert_matches!(stream.next().await, None);
}

#[tokio::test]
async fn peer_receives_progressive_call_results_and_error() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let caller = create_peer("caller").unwrap();
    let callee = create_peer("callee").unwrap();

    assert_matches::assert_matches!(
        caller
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee.join_realm(REALM).await, Ok(()));

    let procedure = callee
        .register(Uri::try_from("com.battler.fn").unwrap())
        .await
        .unwrap();

    async fn handler(mut procedure: Procedure) {
        async fn handle_invocation(invocation: Invocation) {
            assert_matches::assert_matches!(
                invocation.progress(RpcYield {
                    arguments: List::from_iter([Value::Integer(1)]),
                    ..Default::default()
                }),
                Ok(())
            );
            assert_matches::assert_matches!(invocation.respond_error(Error::msg("failed")), Ok(()));
        }

        while let Ok(message) = procedure.procedure_message_rx.recv().await {
            match message {
                ProcedureMessage::Invocation(invocation) => {
                    tokio::spawn(handle_invocation(invocation));
                }
                _ => (),
            }
        }
    }

    tokio::spawn(handler(procedure));

    let rpc = caller
        .call_with_progress(Uri::try_from("com.battler.fn").unwrap(), RpcCall::default())
        .await
        .unwrap();

    let mut stream = rpc.into_stream();
    assert_matches::assert_matches!(stream.next().await, Some(Ok(result)) => {
        pretty_assertions::assert_eq!(result, RpcResult {
            arguments: List::from_iter([Value::Integer(1)]),
            progress: true,
            ..Default::default()
        });
    });
    assert_matches::assert_matches!(stream.next().await, Some(Err(err)) => {
        assert_eq!(err.to_string(), "failed");
    });
    assert_matches::assert_matches!(stream.next().await, None);
}

#[tokio::test]
async fn peer_kills_progressive_call_and_ends_stream() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let caller = create_peer("caller").unwrap();
    let callee = create_peer("callee").unwrap();

    assert_matches::assert_matches!(
        caller
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee.join_realm(REALM).await, Ok(()));

    let procedure = callee
        .register(Uri::try_from("com.battler.fn").unwrap())
        .await
        .unwrap();

    async fn handler(mut procedure: Procedure) {
        async fn handle_invocation(
            invocation: Invocation,
            mut interrupt_rx: UnboundedReceiver<()>,
        ) {
            loop {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(10)) => {
                        invocation.respond_error(Error::msg("timeout")).unwrap();
                        return;
                    }
                    _ = interrupt_rx.recv() => {
                        invocation.progress(RpcYield::default()).unwrap();
                        return;
                    }
                }
            }
        }

        let mut interrupt_txs = HashMap::default();
        while let Ok(message) = procedure.procedure_message_rx.recv().await {
            match message {
                ProcedureMessage::Invocation(invocation) => {
                    let (interrupt_tx, interrupt_rx) = unbounded_channel();
                    interrupt_txs.insert(invocation.id(), interrupt_tx);
                    tokio::spawn(handle_invocation(invocation, interrupt_rx));
                }
                ProcedureMessage::Interrupt(interrupt) => {
                    if let Some(interrupt_tx) = interrupt_txs.get(&interrupt.id()) {
                        interrupt_tx.send(()).unwrap();
                    }
                }
            }
        }
    }

    tokio::spawn(handler(procedure));

    let mut rpc = caller
        .call_with_progress(Uri::try_from("com.battler.fn").unwrap(), RpcCall::default())
        .await
        .unwrap();

    // Kill the call and wait for the result.
    assert_matches::assert_matches!(rpc.kill().await, Ok(()));
    assert!(!rpc.done());
    assert_matches::assert_matches!(rpc.next_result().await, Ok(Some(result)) => {
        pretty_assertions::assert_eq!(result, RpcResult {
            progress: true,
            ..Default::default()
        })
    });
    assert!(rpc.done());
    assert_matches::assert_matches!(rpc.next_result().await, Ok(None));
}

#[tokio::test]
async fn progressive_call_interrupted_when_caller_leaves() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let caller = create_peer("caller").unwrap();
    let callee = create_peer("callee").unwrap();

    assert_matches::assert_matches!(
        caller
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee.join_realm(REALM).await, Ok(()));

    let procedure = callee
        .register(Uri::try_from("com.battler.fn").unwrap())
        .await
        .unwrap();

    // Must wait for the invocation to be received, since a canceled call could never be sent to the
    // callee.
    let (invocation_received_tx, mut invocation_received_rx) = unbounded_channel();

    async fn handler(mut procedure: Procedure, invocation_received_tx: UnboundedSender<()>) {
        async fn handle_invocation(invocation: Invocation) {
            loop {
                tokio::time::sleep(Duration::from_secs(1)).await;
                invocation.progress(RpcYield::default()).unwrap();
            }
        }

        let mut request_id = None;
        while let Ok(message) = procedure.procedure_message_rx.recv().await {
            match message {
                ProcedureMessage::Invocation(invocation) => {
                    invocation_received_tx.send(()).unwrap();
                    request_id = Some(invocation.id());
                    tokio::spawn(handle_invocation(invocation));
                }
                ProcedureMessage::Interrupt(interrupt) => {
                    // Return when our single invocation is interrupted.
                    if request_id.is_some_and(|id| id == interrupt.id()) {
                        return;
                    }
                }
            }
        }
    }

    let handler_handle = tokio::spawn(handler(procedure, invocation_received_tx));

    let mut rpc = caller
        .call_with_progress(Uri::try_from("com.battler.fn").unwrap(), RpcCall::default())
        .await
        .unwrap();

    // Wait for the invocation to be received.
    invocation_received_rx.recv().await.unwrap();

    // Leave the realm.
    caller.leave_realm().await.unwrap();

    assert_matches::assert_matches!(rpc.next_result().await, Err(err) => {
        assert_matches::assert_matches!(err.downcast::<PeerNotConnectedError>(), Ok(_));
    });

    // Wait for the RPC to be interrupted.
    handler_handle.await.unwrap();
}

#[tokio::test]
async fn router_times_out_procedure_call() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let caller = create_peer("caller").unwrap();
    let callee = create_peer("callee").unwrap();

    assert_matches::assert_matches!(
        caller
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee.join_realm(REALM).await, Ok(()));

    let procedure = callee
        .register(Uri::try_from("com.battler.fn").unwrap())
        .await
        .unwrap();

    async fn handler(mut procedure: Procedure) {
        let mut request_id = None;
        while let Ok(message) = procedure.procedure_message_rx.recv().await {
            match message {
                ProcedureMessage::Invocation(invocation) => {
                    request_id = Some(invocation.id());
                    // Never respond to the invocation. The router should time the procedure out.
                    assert!(invocation.timeout.is_zero());
                }
                ProcedureMessage::Interrupt(interrupt) => {
                    // Return when our single invocation is interrupted.
                    if request_id.is_some_and(|id| id == interrupt.id()) {
                        return;
                    }
                }
            }
        }
    }

    let handler_handle = tokio::spawn(handler(procedure));

    assert_matches::assert_matches!(
        caller
            .call_and_wait(Uri::try_from("com.battler.fn").unwrap(), RpcCall {
                timeout: Some(Duration::from_secs(2)),
                ..Default::default()
            })
            .await,
        Err(err) => {
            assert_matches::assert_matches!(err.downcast::<InteractionError>(), Ok(InteractionError::Canceled));
        }
    );

    // Wait for the interrupt to the callee.
    handler_handle.await.unwrap();
}

#[tokio::test]
async fn callee_times_out_procedure_call() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let caller = create_peer("caller").unwrap();

    let mut callee_config = PeerConfig::default();
    callee_config.callee.enforce_timeouts = true;
    let callee = create_peer_with_config("callee", callee_config).unwrap();

    assert_matches::assert_matches!(
        caller
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee.join_realm(REALM).await, Ok(()));

    let procedure = callee
        .register(Uri::try_from("com.battler.fn").unwrap())
        .await
        .unwrap();

    async fn handler(mut procedure: Procedure) {
        async fn handle_invocation(invocation: Invocation) {
            assert_ne!(invocation.timeout, Duration::ZERO);
            tokio::time::sleep(invocation.timeout).await;
            invocation
                .respond_error(InteractionError::Canceled)
                .unwrap();
        }

        while let Ok(message) = procedure.procedure_message_rx.recv().await {
            match message {
                ProcedureMessage::Invocation(invocation) => {
                    tokio::spawn(handle_invocation(invocation));
                }
                _ => (),
            }
        }
    }

    tokio::spawn(handler(procedure));

    assert_matches::assert_matches!(
        caller
            .call_and_wait(Uri::try_from("com.battler.fn").unwrap(), RpcCall {
                timeout: Some(Duration::from_secs(2)),
                ..Default::default()
            })
            .await,
        Err(err) => {
            assert_matches::assert_matches!(err.downcast::<InteractionError>(), Ok(InteractionError::Canceled));
        }
    );
}

#[tokio::test]
async fn procedure_call_matches_registration_by_prefix() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let caller = create_peer("caller").unwrap();
    let callee = create_peer("callee").unwrap();

    assert_matches::assert_matches!(
        caller
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee.join_realm(REALM).await, Ok(()));

    let procedure = callee
        .register_with_options(
            WildcardUri::try_from("com.battler.fn").unwrap(),
            ProcedureOptions {
                match_style: Some(MatchStyle::Prefix),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    async fn handler(mut procedure: Procedure) {
        async fn handle_invocation(invocation: Invocation) {
            assert_matches::assert_matches!(invocation.procedure.as_ref(), Some(procedure) => {
                assert_eq!(procedure, &Uri::try_from("com.battler.fn.a.b.c").unwrap());
                invocation.respond_ok(RpcYield::default()).unwrap();
            });
        }

        while let Ok(message) = procedure.procedure_message_rx.recv().await {
            match message {
                ProcedureMessage::Invocation(invocation) => {
                    tokio::spawn(handle_invocation(invocation));
                }
                _ => (),
            }
        }
    }

    tokio::spawn(handler(procedure));

    assert_matches::assert_matches!(
        caller
            .call_and_wait(
                Uri::try_from("com.battler.fn.a.b.c").unwrap(),
                RpcCall::default()
            )
            .await,
        Ok(_)
    );
}

#[tokio::test]
async fn procedure_call_matches_registration_by_wildcard() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let caller = create_peer("caller").unwrap();
    let callee = create_peer("callee").unwrap();

    assert_matches::assert_matches!(
        caller
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee.join_realm(REALM).await, Ok(()));

    let procedure = callee
        .register_with_options(
            WildcardUri::try_from("com.battler.battle..start").unwrap(),
            ProcedureOptions {
                match_style: Some(MatchStyle::Wildcard),
                ..Default::default()
            },
        )
        .await
        .unwrap();

    async fn handler(mut procedure: Procedure) {
        async fn handle_invocation(invocation: Invocation) {
            assert_matches::assert_matches!(invocation.procedure.as_ref(), Some(procedure) => {
                assert_eq!(procedure, &Uri::try_from("com.battler.battle.abcd.start").unwrap());
                invocation.respond_ok(RpcYield::default()).unwrap();
            });
        }

        while let Ok(message) = procedure.procedure_message_rx.recv().await {
            match message {
                ProcedureMessage::Invocation(invocation) => {
                    tokio::spawn(handle_invocation(invocation));
                }
                _ => (),
            }
        }
    }

    tokio::spawn(handler(procedure));

    assert_matches::assert_matches!(
        caller
            .call_and_wait(
                Uri::try_from("com.battler.battle").unwrap(),
                RpcCall::default()
            )
            .await,
        Err(err) => {
            assert_matches::assert_matches!(err.downcast::<InteractionError>(), Ok(InteractionError::NoSuchProcedure));
        }
    );
    assert_matches::assert_matches!(
        caller
            .call_and_wait(
                Uri::try_from("com.battler.battle.abcd").unwrap(),
                RpcCall::default()
            )
            .await,
        Err(err) => {
            assert_matches::assert_matches!(err.downcast::<InteractionError>(), Ok(InteractionError::NoSuchProcedure));
        }
    );
    assert_matches::assert_matches!(
        caller
            .call_and_wait(
                Uri::try_from("com.battler.battle.abcd.start").unwrap(),
                RpcCall::default()
            )
            .await,
        Ok(_)
    );
}

mod procedure_wildcard_match_test {
    use battler_wamp::{
        core::{
            error::InteractionError,
            match_style::MatchStyle,
            uri::{
                Uri,
                WildcardUri,
            },
        },
        peer::{
            Peer,
            Procedure,
            ProcedureMessage,
            ProcedureOptions,
            RpcCall,
            RpcYield,
        },
    };
    use tokio::{
        sync::broadcast,
        task::JoinHandle,
    };

    use crate::{
        create_peer,
        start_router,
        REALM,
    };

    async fn register_handler_that_expects_invocation<S>(
        peer: &Peer<S>,
        uri: WildcardUri,
        match_style: Option<MatchStyle>,
        cancel_rx: broadcast::Receiver<()>,
    ) -> JoinHandle<()>
    where
        S: Send + 'static,
    {
        let procedure = peer
            .register_with_options(
                uri.clone(),
                ProcedureOptions {
                    match_style,
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        async fn handler(
            mut procedure: Procedure,
            uri: WildcardUri,
            mut cancel_rx: broadcast::Receiver<()>,
        ) {
            loop {
                tokio::select! {
                    message = procedure.procedure_message_rx.recv() => {
                        match message {
                            Ok(ProcedureMessage::Invocation(invocation)) => {
                                invocation.respond_ok(RpcYield::default()).unwrap();
                                return;
                            }
                            _ => (),
                        }
                    }
                    _ = cancel_rx.recv() => {
                        panic!("no invocation received for {uri}");
                    }
                }
            }
        }

        tokio::spawn(handler(procedure, uri, cancel_rx))
    }

    async fn register_handler_that_expects_no_invocation<S>(
        peer: &Peer<S>,
        uri: WildcardUri,
        match_style: Option<MatchStyle>,
        cancel_rx: broadcast::Receiver<()>,
    ) -> JoinHandle<()>
    where
        S: Send + 'static,
    {
        let procedure = peer
            .register_with_options(
                uri.clone(),
                ProcedureOptions {
                    match_style,
                    ..Default::default()
                },
            )
            .await
            .unwrap();

        async fn handler(mut procedure: Procedure, mut cancel_rx: broadcast::Receiver<()>) {
            loop {
                tokio::select! {
                    message = procedure.procedure_message_rx.recv() => {
                        match message {
                            Ok(ProcedureMessage::Invocation(invocation)) => {
                                panic!("unexpected invocation {invocation:?}")
                            }
                            _ => (),
                        }
                    }
                    _ = cancel_rx.recv() => {
                        return;
                    }
                }
            }
        }

        tokio::spawn(handler(procedure, cancel_rx))
    }

    #[tokio::test]
    async fn uses_exact_match() {
        test_utils::setup::setup_test_environment();

        let (router_handle, _) = start_router().await.unwrap();
        let caller = create_peer("caller").unwrap();
        let callee = create_peer("callee").unwrap();

        assert_matches::assert_matches!(
            caller
                .connect(&format!("ws://{}", router_handle.local_addr()))
                .await,
            Ok(())
        );
        assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

        assert_matches::assert_matches!(
            callee
                .connect(&format!("ws://{}", router_handle.local_addr()))
                .await,
            Ok(())
        );
        assert_matches::assert_matches!(callee.join_realm(REALM).await, Ok(()));

        let (cancel_tx, cancel_rx) = broadcast::channel(16);
        let handles = Vec::from_iter([register_handler_that_expects_invocation(
            &callee,
            WildcardUri::try_from("a1.b2.c3.d4.e55").unwrap(),
            None,
            cancel_rx.resubscribe(),
        )
        .await]);

        assert_matches::assert_matches!(
            caller
                .call_and_wait(
                    Uri::try_from("a1.b2.c3.d4.e55").unwrap(),
                    RpcCall::default()
                )
                .await,
            Ok(_)
        );

        cancel_tx.send(()).unwrap();
        for result in futures_util::future::join_all(handles).await {
            assert_matches::assert_matches!(result, Ok(()));
        }
    }

    #[tokio::test]
    async fn uses_single_prefix_match() {
        test_utils::setup::setup_test_environment();

        let (router_handle, _) = start_router().await.unwrap();
        let caller = create_peer("caller").unwrap();
        let callee = create_peer("callee").unwrap();

        assert_matches::assert_matches!(
            caller
                .connect(&format!("ws://{}", router_handle.local_addr()))
                .await,
            Ok(())
        );
        assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

        assert_matches::assert_matches!(
            callee
                .connect(&format!("ws://{}", router_handle.local_addr()))
                .await,
            Ok(())
        );
        assert_matches::assert_matches!(callee.join_realm(REALM).await, Ok(()));

        let (cancel_tx, cancel_rx) = broadcast::channel(16);
        let handles = Vec::from_iter([
            register_handler_that_expects_no_invocation(
                &callee,
                WildcardUri::try_from("a1.b2.c3.d4.e55").unwrap(),
                None,
                cancel_rx.resubscribe(),
            )
            .await,
            register_handler_that_expects_invocation(
                &callee,
                WildcardUri::try_from("a1.b2.c3").unwrap(),
                Some(MatchStyle::Prefix),
                cancel_rx.resubscribe(),
            )
            .await,
        ]);

        assert_matches::assert_matches!(
            caller
                .call_and_wait(
                    Uri::try_from("a1.b2.c3.d98.e74").unwrap(),
                    RpcCall::default()
                )
                .await,
            Ok(_)
        );

        cancel_tx.send(()).unwrap();
        for result in futures_util::future::join_all(handles).await {
            assert_matches::assert_matches!(result, Ok(()));
        }
    }

    #[tokio::test]
    async fn uses_longest_prefix_match() {
        test_utils::setup::setup_test_environment();

        let (router_handle, _) = start_router().await.unwrap();
        let caller = create_peer("caller").unwrap();
        let callee = create_peer("callee").unwrap();

        assert_matches::assert_matches!(
            caller
                .connect(&format!("ws://{}", router_handle.local_addr()))
                .await,
            Ok(())
        );
        assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

        assert_matches::assert_matches!(
            callee
                .connect(&format!("ws://{}", router_handle.local_addr()))
                .await,
            Ok(())
        );
        assert_matches::assert_matches!(callee.join_realm(REALM).await, Ok(()));

        let (cancel_tx, cancel_rx) = broadcast::channel(16);
        let handles = Vec::from_iter([
            register_handler_that_expects_no_invocation(
                &callee,
                WildcardUri::try_from("a1.b2.c3").unwrap(),
                Some(MatchStyle::Prefix),
                cancel_rx.resubscribe(),
            )
            .await,
            register_handler_that_expects_invocation(
                &callee,
                WildcardUri::try_from("a1.b2.c3.d4").unwrap(),
                Some(MatchStyle::Prefix),
                cancel_rx.resubscribe(),
            )
            .await,
        ]);

        assert_matches::assert_matches!(
            caller
                .call_and_wait(
                    Uri::try_from("a1.b2.c3.d4.325").unwrap(),
                    RpcCall::default()
                )
                .await,
            Ok(_)
        );

        cancel_tx.send(()).unwrap();
        for result in futures_util::future::join_all(handles).await {
            assert_matches::assert_matches!(result, Ok(()));
        }
    }

    #[tokio::test]
    async fn uses_wildcard_match() {
        test_utils::setup::setup_test_environment();

        let (router_handle, _) = start_router().await.unwrap();
        let caller = create_peer("caller").unwrap();
        let callee = create_peer("callee").unwrap();

        assert_matches::assert_matches!(
            caller
                .connect(&format!("ws://{}", router_handle.local_addr()))
                .await,
            Ok(())
        );
        assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

        assert_matches::assert_matches!(
            callee
                .connect(&format!("ws://{}", router_handle.local_addr()))
                .await,
            Ok(())
        );
        assert_matches::assert_matches!(callee.join_realm(REALM).await, Ok(()));

        let (cancel_tx, cancel_rx) = broadcast::channel(16);
        let handles = Vec::from_iter([
            register_handler_that_expects_no_invocation(
                &callee,
                WildcardUri::try_from("a1.b2.c3.d4").unwrap(),
                Some(MatchStyle::Prefix),
                cancel_rx.resubscribe(),
            )
            .await,
            register_handler_that_expects_invocation(
                &callee,
                WildcardUri::try_from("a1.b2..d4.e5").unwrap(),
                Some(MatchStyle::Wildcard),
                cancel_rx.resubscribe(),
            )
            .await,
        ]);

        assert_matches::assert_matches!(
            caller
                .call_and_wait(
                    Uri::try_from("a1.b2.c55.d4.e5").unwrap(),
                    RpcCall::default()
                )
                .await,
            Ok(_)
        );

        cancel_tx.send(()).unwrap();
        for result in futures_util::future::join_all(handles).await {
            assert_matches::assert_matches!(result, Ok(()));
        }
    }

    #[tokio::test]
    async fn uses_longer_wildcard_match_by_first_portion() {
        test_utils::setup::setup_test_environment();

        let (router_handle, _) = start_router().await.unwrap();
        let caller = create_peer("caller").unwrap();
        let callee = create_peer("callee").unwrap();

        assert_matches::assert_matches!(
            caller
                .connect(&format!("ws://{}", router_handle.local_addr()))
                .await,
            Ok(())
        );
        assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

        assert_matches::assert_matches!(
            callee
                .connect(&format!("ws://{}", router_handle.local_addr()))
                .await,
            Ok(())
        );
        assert_matches::assert_matches!(callee.join_realm(REALM).await, Ok(()));

        let (cancel_tx, cancel_rx) = broadcast::channel(16);
        let handles = Vec::from_iter([
            register_handler_that_expects_no_invocation(
                &callee,
                WildcardUri::try_from("a1.b2..d4.e5").unwrap(),
                Some(MatchStyle::Wildcard),
                cancel_rx.resubscribe(),
            )
            .await,
            register_handler_that_expects_invocation(
                &callee,
                WildcardUri::try_from("a1.b2.c33..e5").unwrap(),
                Some(MatchStyle::Wildcard),
                cancel_rx.resubscribe(),
            )
            .await,
        ]);

        assert_matches::assert_matches!(
            caller
                .call_and_wait(
                    Uri::try_from("a1.b2.c33.d4.e5").unwrap(),
                    RpcCall::default()
                )
                .await,
            Ok(_)
        );

        cancel_tx.send(()).unwrap();
        for result in futures_util::future::join_all(handles).await {
            assert_matches::assert_matches!(result, Ok(()));
        }
    }

    #[tokio::test]
    async fn uses_longer_wildcard_match_by_second_portion() {
        test_utils::setup::setup_test_environment();

        let (router_handle, _) = start_router().await.unwrap();
        let caller = create_peer("caller").unwrap();
        let callee = create_peer("callee").unwrap();

        assert_matches::assert_matches!(
            caller
                .connect(&format!("ws://{}", router_handle.local_addr()))
                .await,
            Ok(())
        );
        assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

        assert_matches::assert_matches!(
            callee
                .connect(&format!("ws://{}", router_handle.local_addr()))
                .await,
            Ok(())
        );
        assert_matches::assert_matches!(callee.join_realm(REALM).await, Ok(()));

        let (cancel_tx, cancel_rx) = broadcast::channel(16);
        let handles = Vec::from_iter([
            register_handler_that_expects_invocation(
                &callee,
                WildcardUri::try_from("a1.b2..d4.e5..g7").unwrap(),
                Some(MatchStyle::Wildcard),
                cancel_rx.resubscribe(),
            )
            .await,
            register_handler_that_expects_no_invocation(
                &callee,
                WildcardUri::try_from("a1.b2..d4..f6.g7").unwrap(),
                Some(MatchStyle::Wildcard),
                cancel_rx.resubscribe(),
            )
            .await,
        ]);

        assert_matches::assert_matches!(
            caller
                .call_and_wait(
                    Uri::try_from("a1.b2.c88.d4.e5.f6.g7").unwrap(),
                    RpcCall::default()
                )
                .await,
            Ok(_)
        );

        cancel_tx.send(()).unwrap();
        for result in futures_util::future::join_all(handles).await {
            assert_matches::assert_matches!(result, Ok(()));
        }
    }

    #[tokio::test]
    async fn no_match_found() {
        test_utils::setup::setup_test_environment();

        let (router_handle, _) = start_router().await.unwrap();
        let caller = create_peer("caller").unwrap();
        let callee = create_peer("callee").unwrap();

        assert_matches::assert_matches!(
            caller
                .connect(&format!("ws://{}", router_handle.local_addr()))
                .await,
            Ok(())
        );
        assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

        assert_matches::assert_matches!(
            callee
                .connect(&format!("ws://{}", router_handle.local_addr()))
                .await,
            Ok(())
        );
        assert_matches::assert_matches!(callee.join_realm(REALM).await, Ok(()));

        let (cancel_tx, cancel_rx) = broadcast::channel(16);
        let handles = Vec::from_iter([
            register_handler_that_expects_no_invocation(
                &callee,
                WildcardUri::try_from("a1.b2.c3.d4.e55").unwrap(),
                None,
                cancel_rx.resubscribe(),
            )
            .await,
            register_handler_that_expects_no_invocation(
                &callee,
                WildcardUri::try_from("a1.b2.c3").unwrap(),
                Some(MatchStyle::Prefix),
                cancel_rx.resubscribe(),
            )
            .await,
            register_handler_that_expects_no_invocation(
                &callee,
                WildcardUri::try_from("a1.b2.c3.d4").unwrap(),
                Some(MatchStyle::Prefix),
                cancel_rx.resubscribe(),
            )
            .await,
            register_handler_that_expects_no_invocation(
                &callee,
                WildcardUri::try_from("a1.b2..d4.e5").unwrap(),
                Some(MatchStyle::Wildcard),
                cancel_rx.resubscribe(),
            )
            .await,
            register_handler_that_expects_no_invocation(
                &callee,
                WildcardUri::try_from("a1.b2.c33..e5").unwrap(),
                Some(MatchStyle::Wildcard),
                cancel_rx.resubscribe(),
            )
            .await,
            register_handler_that_expects_no_invocation(
                &callee,
                WildcardUri::try_from("a1.b2..d4.e5..g7").unwrap(),
                Some(MatchStyle::Wildcard),
                cancel_rx.resubscribe(),
            )
            .await,
            register_handler_that_expects_no_invocation(
                &callee,
                WildcardUri::try_from("a1.b2..d4..f6.g7").unwrap(),
                Some(MatchStyle::Wildcard),
                cancel_rx.resubscribe(),
            )
            .await,
        ]);

        assert_matches::assert_matches!(
            caller
                .call_and_wait(
                    Uri::try_from("a2.b2.c2.d2.e2").unwrap(),
                    RpcCall::default()
                )
                .await,
            Err(err) => {
                assert_matches::assert_matches!(err.downcast::<InteractionError>(), Ok(InteractionError::NoSuchProcedure));
            }
        );

        cancel_tx.send(()).unwrap();
        for result in futures_util::future::join_all(handles).await {
            assert_matches::assert_matches!(result, Ok(()));
        }
    }
}

#[tokio::test]
async fn no_available_callee_when_single_callee_returns_unavailable() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let caller = create_peer("caller").unwrap();
    let callee = create_peer("callee").unwrap();

    assert_matches::assert_matches!(
        caller
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee.join_realm(REALM).await, Ok(()));

    let procedure = callee
        .register(Uri::try_from("com.battler.fn").unwrap())
        .await
        .unwrap();

    async fn handler(mut procedure: Procedure) {
        while let Ok(message) = procedure.procedure_message_rx.recv().await {
            match message {
                ProcedureMessage::Invocation(invocation) => {
                    invocation
                        .respond_error(InteractionError::Unavailable)
                        .unwrap();
                }
                _ => (),
            }
        }
    }

    tokio::spawn(handler(procedure));

    assert_matches::assert_matches!(
        caller
            .call_and_wait(Uri::try_from("com.battler.fn").unwrap(), RpcCall::default())
            .await,
        Err(err) => {
            assert_matches::assert_matches!(err.downcast::<InteractionError>(), Ok(InteractionError::NoAvailableCallee));
        }
    );
}

#[tokio::test]
async fn shared_registration_fails_with_single_invocation_policy() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let callee_1 = create_peer("callee_1").unwrap();
    let callee_2 = create_peer("callee_2").unwrap();

    assert_matches::assert_matches!(
        callee_1
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee_1.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee_2
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee_2.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee_1
            .register_with_options(
                WildcardUri::try_from("com.battler.fn").unwrap(),
                ProcedureOptions {
                    invocation_policy: InvocationPolicy::Single,
                    ..Default::default()
                },
            )
            .await,
        Ok(_)
    );

    assert_matches::assert_matches!(
        callee_2
            .register_with_options(
                WildcardUri::try_from("com.battler.fn").unwrap(),
                ProcedureOptions {
                    invocation_policy: InvocationPolicy::Single,
                    ..Default::default()
                },
            )
            .await,
        Err(err) => {
            assert_matches::assert_matches!(err.downcast::<BasicError>(), Ok(BasicError::NotAllowed(_)));
        }
    );

    assert_matches::assert_matches!(
        callee_2
            .register_with_options(
                WildcardUri::try_from("com.battler.fn").unwrap(),
                ProcedureOptions {
                    invocation_policy: InvocationPolicy::First,
                    ..Default::default()
                },
            )
            .await,
        Err(err) => {
            assert_matches::assert_matches!(err.downcast::<InteractionError>(), Ok(InteractionError::ProcedureAlreadyExists));
        }
    );
}

#[tokio::test]
async fn shared_registration_fails_with_different_invocation_policy() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let callee_1 = create_peer("callee_1").unwrap();
    let callee_2 = create_peer("callee_2").unwrap();

    assert_matches::assert_matches!(
        callee_1
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee_1.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee_2
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee_2.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee_1
            .register_with_options(
                WildcardUri::try_from("com.battler.fn").unwrap(),
                ProcedureOptions {
                    invocation_policy: InvocationPolicy::Random,
                    ..Default::default()
                },
            )
            .await,
        Ok(_)
    );

    assert_matches::assert_matches!(
        callee_2
            .register_with_options(
                WildcardUri::try_from("com.battler.fn").unwrap(),
                ProcedureOptions {
                    invocation_policy: InvocationPolicy::RoundRobin,
                    ..Default::default()
                },
            )
            .await,
        Err(err) => {
            assert_matches::assert_matches!(err.downcast::<InteractionError>(), Ok(InteractionError::ProcedureAlreadyExists));
        }
    );
}

#[tokio::test]
async fn shared_registration_succeeds_with_same_invocation_policy() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let callee_1 = create_peer("callee_1").unwrap();
    let callee_2 = create_peer("callee_2").unwrap();

    assert_matches::assert_matches!(
        callee_1
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee_1.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee_2
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee_2.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee_1
            .register_with_options(
                WildcardUri::try_from("com.battler.fn").unwrap(),
                ProcedureOptions {
                    invocation_policy: InvocationPolicy::Random,
                    ..Default::default()
                },
            )
            .await,
        Ok(_)
    );

    assert_matches::assert_matches!(
        callee_2
            .register_with_options(
                WildcardUri::try_from("com.battler.fn").unwrap(),
                ProcedureOptions {
                    invocation_policy: InvocationPolicy::Random,
                    ..Default::default()
                },
            )
            .await,
        Ok(_)
    );
}

#[tokio::test]
async fn invokes_second_caller_when_first_reports_unavailable() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let caller = create_peer("caller").unwrap();
    let callee_1 = create_peer("callee_1").unwrap();
    let callee_2 = create_peer("callee_2").unwrap();

    assert_matches::assert_matches!(
        caller
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee_1
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee_1.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee_2
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee_2.join_realm(REALM).await, Ok(()));

    let procedure = callee_1
        .register_with_options(
            WildcardUri::try_from("com.battler.fn").unwrap(),
            ProcedureOptions {
                invocation_policy: InvocationPolicy::First,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    async fn unavailable_handler(mut procedure: Procedure) {
        while let Ok(message) = procedure.procedure_message_rx.recv().await {
            match message {
                ProcedureMessage::Invocation(invocation) => {
                    invocation
                        .respond_error(InteractionError::Unavailable)
                        .unwrap();
                }
                _ => (),
            }
        }
    }

    tokio::spawn(unavailable_handler(procedure));

    let procedure = callee_2
        .register_with_options(
            WildcardUri::try_from("com.battler.fn").unwrap(),
            ProcedureOptions {
                invocation_policy: InvocationPolicy::First,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    async fn available_handler(mut procedure: Procedure) {
        while let Ok(message) = procedure.procedure_message_rx.recv().await {
            match message {
                ProcedureMessage::Invocation(invocation) => {
                    invocation.respond_ok(RpcYield::default()).unwrap();
                }
                _ => (),
            }
        }
    }

    tokio::spawn(available_handler(procedure));

    assert_matches::assert_matches!(
        caller
            .call_and_wait(Uri::try_from("com.battler.fn").unwrap(), RpcCall::default())
            .await,
        Ok(_)
    );
}

#[tokio::test]
async fn no_available_callee_when_all_callees_report_unavailable() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router().await.unwrap();
    let caller = create_peer("caller").unwrap();
    let callee_1 = create_peer("callee_1").unwrap();
    let callee_2 = create_peer("callee_2").unwrap();

    assert_matches::assert_matches!(
        caller
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(caller.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee_1
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee_1.join_realm(REALM).await, Ok(()));

    assert_matches::assert_matches!(
        callee_2
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee_2.join_realm(REALM).await, Ok(()));

    let procedure = callee_1
        .register_with_options(
            WildcardUri::try_from("com.battler.fn").unwrap(),
            ProcedureOptions {
                invocation_policy: InvocationPolicy::First,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    async fn unavailable_handler(mut procedure: Procedure) {
        while let Ok(message) = procedure.procedure_message_rx.recv().await {
            match message {
                ProcedureMessage::Invocation(invocation) => {
                    invocation
                        .respond_error(InteractionError::Unavailable)
                        .unwrap();
                }
                _ => (),
            }
        }
    }

    tokio::spawn(unavailable_handler(procedure));

    let procedure = callee_2
        .register_with_options(
            WildcardUri::try_from("com.battler.fn").unwrap(),
            ProcedureOptions {
                invocation_policy: InvocationPolicy::First,
                ..Default::default()
            },
        )
        .await
        .unwrap();

    tokio::spawn(unavailable_handler(procedure));

    assert_matches::assert_matches!(
        caller
            .call_and_wait(Uri::try_from("com.battler.fn").unwrap(), RpcCall::default())
            .await,
        Err(err) => {
            assert_matches::assert_matches!(err.downcast::<InteractionError>(), Ok(InteractionError::NoAvailableCallee));
        }
    );
}
