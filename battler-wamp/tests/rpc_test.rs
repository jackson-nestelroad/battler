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
        uri::Uri,
    },
    peer::{
        new_web_socket_peer,
        Invocation,
        PeerConfig,
        PeerNotConnectedError,
        Procedure,
        ProcedureMessage,
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

fn create_peer(name: &str) -> Result<WebSocketPeer> {
    let mut config = PeerConfig::default();
    config.name = name.to_owned();
    new_web_socket_peer(config)
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
