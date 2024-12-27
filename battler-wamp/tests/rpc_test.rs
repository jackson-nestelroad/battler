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
        types::{
            Integer,
            List,
            Value,
        },
        uri::Uri,
    },
    peer::{
        new_web_socket_peer,
        Invocation,
        PeerConfig,
        Procedure,
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
use futures_util::future::join_all;

const REALM: &str = "com.battler.test";

async fn start_router_with_config(mut config: RouterConfig) -> Result<RouterHandle, Error> {
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

async fn start_router() -> Result<RouterHandle, Error> {
    start_router_with_config(RouterConfig::default()).await
}

fn create_peer(agent: &str) -> Result<WebSocketPeer, Error> {
    let mut config = PeerConfig::default();
    config.name = agent.to_owned();
    new_web_socket_peer(config)
}

#[tokio::test]
async fn peer_invokes_procedure_from_another_peer() {
    test_utils::setup::setup_test_environment();

    let router_handle = start_router().await.unwrap();
    let mut caller = create_peer("caller").unwrap();
    let mut callee = create_peer("callee").unwrap();

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

        while let Ok(invocation) = procedure.invocation_rx.recv().await {
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
            .call(
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
            .call(
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
            .call(
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

    let router_handle = start_router().await.unwrap();
    let mut caller = create_peer("caller").unwrap();
    let mut callee = create_peer("callee").unwrap();

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
        while let Ok(_) = procedure.invocation_rx.recv().await {
            // Leave the realm when we receive an invocation.
            assert_matches::assert_matches!(callee.leave_realm().await, Ok(()));
            return;
        }
    }

    let handler_handle = tokio::spawn(handler(callee, procedure));

    assert_matches::assert_matches!(
        caller.call(
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

    let router_handle = start_router().await.unwrap();
    let mut caller = create_peer("caller").unwrap();
    let mut callee = create_peer("callee").unwrap();

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
        while let Ok(invocation) = procedure.invocation_rx.recv().await {
            invocations.push(invocation);

            // Wait for two invocations.
            if invocations.len() < 2 {
                continue;
            }

            // Respond to invocations at the same time.
            for invocation in invocations {
                let arguments = invocation.arguments.clone();
                assert_matches::assert_matches!(
                    invocation.respond(Ok(RpcYield {
                        arguments,
                        ..Default::default()
                    })),
                    Ok(())
                );
            }
            break;
        }
    }

    let handler_handle = tokio::spawn(handler(procedure));

    // Two calls made in parallel.
    let call_1 = caller.call(
        Uri::try_from("com.battler.fn").unwrap(),
        RpcCall {
            arguments: List::from_iter([Value::Integer(1)]),
            ..Default::default()
        },
    );
    let call_2 = caller.call(
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
