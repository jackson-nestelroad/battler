use anyhow::Result;
use battler_wamp::{
    core::uri::Uri,
    peer::{
        PeerConfig,
        Procedure,
        ProcedureMessage,
        RpcCall,
        RpcResult,
        RpcYield,
        SupportedAuthMethod as PeerSupportedAuthMethod,
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
        SupportedAuthMethod as RouterSupportedAuthMethod,
        new_web_socket_router,
    },
};
use battler_wamp_values::{
    Dictionary,
    Value,
};
use tokio::task::JoinHandle;

const REALM: &str = "com.battler.test";

async fn start_router(authentication_required: bool) -> Result<(RouterHandle, JoinHandle<()>)> {
    let router = new_web_socket_router(
        RouterConfig {
            realms: Vec::from_iter([RealmConfig {
                name: "test".to_owned(),
                uri: Uri::try_from(REALM)?,
                authentication: RealmAuthenticationConfig {
                    required: authentication_required,
                    methods: Vec::from_iter([RouterSupportedAuthMethod::Undisputed]),
                },
            }]),
            ..Default::default()
        },
        Box::new(EmptyPubSubPolicies::default()),
        Box::new(EmptyRpcPolicies::default()),
    )?;
    router.start().await
}

fn create_peer(name: &str) -> Result<WebSocketPeer> {
    new_web_socket_peer(PeerConfig {
        name: name.to_owned(),
        ..Default::default()
    })
}

#[tokio::test(flavor = "multi_thread")]
async fn peer_joins_realm_with_undisputed_identity() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router(false).await.unwrap();
    let caller = create_peer("caller").unwrap();
    let callee = create_peer("callee").unwrap();

    assert_matches::assert_matches!(
        caller
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(
        caller
            .join_realm_with_authentication(
                REALM,
                &[PeerSupportedAuthMethod::Undisputed {
                    id: "test-user".to_owned(),
                    role: "admin".to_owned(),
                }]
            )
            .await,
        Ok(())
    );

    assert_matches::assert_matches!(
        callee
            .connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );
    assert_matches::assert_matches!(callee.join_realm(REALM).await, Ok(()));

    let procedure = callee
        .register(Uri::try_from("com.battler.echo_caller_id").unwrap())
        .await
        .unwrap();
    let procedure_id = procedure.id;

    async fn echo_caller_id(mut procedure: Procedure) {
        while let Ok(ProcedureMessage::Invocation(invocation)) =
            procedure.procedure_message_rx.recv().await
        {
            let rpc_yield = RpcYield {
                arguments_keyword: Dictionary::from_iter([
                    (
                        "id".to_owned(),
                        Value::String(invocation.peer_info.identity.id.clone()),
                    ),
                    (
                        "role".to_owned(),
                        Value::String(invocation.peer_info.identity.role.clone()),
                    ),
                ]),
                ..Default::default()
            };
            assert_matches::assert_matches!(invocation.respond_ok(rpc_yield).await, Ok(()));
        }
    }

    let echo_caller_id_handle = tokio::spawn(echo_caller_id(procedure));

    assert_matches::assert_matches!(
        caller
            .call_and_wait(
                Uri::try_from("com.battler.echo_caller_id").unwrap(),
                RpcCall::default(),
            )
            .await,
        Ok(result) => {
            pretty_assertions::assert_eq!(result, RpcResult {
                arguments_keyword: Dictionary::from_iter([
                    ("id".to_owned(), Value::String("test-user".to_owned())),
                    ("role".to_owned(), Value::String("admin".to_owned())),
                ]),
                ..Default::default()
            });
        }
    );

    assert_matches::assert_matches!(callee.unregister(procedure_id).await, Ok(()));
    echo_caller_id_handle.await.unwrap();
}
