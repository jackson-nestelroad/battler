use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use battler_wamp::{
    auth::scram::{
        UserData,
        UserDatabase,
        UserDatabaseFactory,
        new_user,
    },
    core::{
        error::InteractionError,
        hash::HashMap,
        uri::Uri,
    },
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

async fn start_router(
    user_database_factory: Box<dyn UserDatabaseFactory>,
    authentication_required: bool,
) -> Result<(RouterHandle, JoinHandle<()>)> {
    let router = new_web_socket_router(
        RouterConfig {
            realms: Vec::from_iter([RealmConfig {
                name: "test".to_owned(),
                uri: Uri::try_from(REALM)?,
                authentication: RealmAuthenticationConfig {
                    required: authentication_required,
                    methods: Vec::from_iter([RouterSupportedAuthMethod::WampScram(Arc::new(
                        user_database_factory,
                    ))]),
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

#[derive(Default)]
struct FakeUserDatabase {
    users: HashMap<String, UserData>,
}

impl<S, T> FromIterator<(S, T)> for FakeUserDatabase
where
    S: Into<String> + AsRef<str>,
    T: Into<String> + AsRef<str>,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (S, T)>,
    {
        Self {
            users: iter
                .into_iter()
                .map(|(s, t)| {
                    let user = new_user(s.as_ref(), t.as_ref()).unwrap();
                    (s.into(), user)
                })
                .collect(),
        }
    }
}

#[async_trait]
impl UserDatabase for FakeUserDatabase {
    async fn user_data(&self, id: &str) -> Result<UserData> {
        self.users
            .get(id)
            .ok_or_else(|| InteractionError::NoSuchPrincipal.into())
            .cloned()
    }
}

#[derive(Debug)]
struct FakeUserDatabaseFactory {
    users: HashMap<String, String>,
}

impl<S, T> FromIterator<(S, T)> for FakeUserDatabaseFactory
where
    S: Into<String> + AsRef<str>,
    T: Into<String> + AsRef<str>,
{
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = (S, T)>,
    {
        Self {
            users: iter
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect(),
        }
    }
}

#[async_trait]
impl UserDatabaseFactory for FakeUserDatabaseFactory {
    async fn create_user_database(&self) -> Result<Box<dyn UserDatabase>> {
        Ok(Box::new(FakeUserDatabase::from_iter(self.users.iter())))
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn peer_joins_realm_without_authentication() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router(
        Box::new(FakeUserDatabaseFactory::from_iter([(
            "test-user",
            "password123!!",
        )])),
        false,
    )
    .await
    .unwrap();
    let peer = create_peer("peer").unwrap();

    assert_matches::assert_matches!(
        peer.connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );

    assert_matches::assert_matches!(peer.join_realm(REALM).await, Ok(()));
}

#[tokio::test(flavor = "multi_thread")]
async fn peer_joins_realm_with_optional_authentication() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router(
        Box::new(FakeUserDatabaseFactory::from_iter([(
            "test-user",
            "password123!!",
        )])),
        false,
    )
    .await
    .unwrap();
    let peer = create_peer("peer").unwrap();

    assert_matches::assert_matches!(
        peer.connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );

    assert_matches::assert_matches!(
        peer.join_realm_with_authentication(
            REALM,
            &[PeerSupportedAuthMethod::WampScram {
                id: "test-user".to_owned(),
                password: "password123!!".to_owned()
            }]
        )
        .await,
        Ok(())
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn peer_cannot_join_realm_without_authentication() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router(
        Box::new(FakeUserDatabaseFactory::from_iter([(
            "test-user",
            "password123!!",
        )])),
        true,
    )
    .await
    .unwrap();
    let peer = create_peer("peer").unwrap();

    assert_matches::assert_matches!(
        peer.connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );

    assert_matches::assert_matches!(peer.join_realm(REALM).await, Err(err) => {
        assert_matches::assert_matches!(err.downcast::<InteractionError>(), Ok(InteractionError::AuthenticationRequired));
    });
}

#[tokio::test(flavor = "multi_thread")]
async fn peer_joins_realm_with_required_authentication() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router(
        Box::new(FakeUserDatabaseFactory::from_iter([(
            "test-user",
            "AnOtHeRpAsSwOrD2345678654%%%%",
        )])),
        true,
    )
    .await
    .unwrap();
    let peer = create_peer("peer").unwrap();

    assert_matches::assert_matches!(
        peer.connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );

    assert_matches::assert_matches!(
        peer.join_realm_with_authentication(
            REALM,
            &[PeerSupportedAuthMethod::WampScram {
                id: "test-user".to_owned(),
                password: "AnOtHeRpAsSwOrD2345678654%%%%".to_owned()
            }]
        )
        .await,
        Ok(())
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn peer_fails_authentication_with_invalid_password() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router(
        Box::new(FakeUserDatabaseFactory::from_iter([(
            "test-user",
            "correct",
        )])),
        true,
    )
    .await
    .unwrap();
    let peer = create_peer("peer").unwrap();

    assert_matches::assert_matches!(
        peer.connect(&format!("ws://{}", router_handle.local_addr()))
            .await,
        Ok(())
    );

    assert_matches::assert_matches!(
        peer.join_realm_with_authentication(
            REALM,
            &[PeerSupportedAuthMethod::WampScram {
                id: "test-user".to_owned(),
                password: "incorrect".to_owned()
            }]
        )
        .await,
        Err(err) => {
            assert_matches::assert_matches!(err.downcast::<InteractionError>(), Ok(InteractionError::AuthenticationDenied(message)) => {
                assert!(message.contains("invalid password"), "{message}");
            });
        }
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn rpc_invocation_holds_empty_identity_for_caller_without_authentication() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router(
        Box::new(FakeUserDatabaseFactory::from_iter([(
            "test-user",
            "password",
        )])),
        false,
    )
    .await
    .unwrap();
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
                        Value::String(invocation.identity.id.clone()),
                    ),
                    (
                        "role".to_owned(),
                        Value::String(invocation.identity.role.clone()),
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
                    ("id".to_owned(), Value::String("".to_owned())),
                    ("role".to_owned(), Value::String("".to_owned())),
                ]),
                ..Default::default()
            });
        }
    );

    assert_matches::assert_matches!(callee.unregister(procedure_id).await, Ok(()));
    echo_caller_id_handle.await.unwrap();
}

#[tokio::test(flavor = "multi_thread")]
async fn rpc_invocation_holds_identity_for_caller_with_authentication() {
    test_utils::setup::setup_test_environment();

    let (router_handle, _) = start_router(
        Box::new(FakeUserDatabaseFactory::from_iter([(
            "test-user",
            "password",
        )])),
        false,
    )
    .await
    .unwrap();
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
                &[PeerSupportedAuthMethod::WampScram {
                    id: "test-user".to_owned(),
                    password: "password".to_owned(),
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
                        Value::String(invocation.identity.id.clone()),
                    ),
                    (
                        "role".to_owned(),
                        Value::String(invocation.identity.role.clone()),
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
                    ("role".to_owned(), Value::String("user".to_owned())),
                ]),
                ..Default::default()
            });
        }
    );

    assert_matches::assert_matches!(callee.unregister(procedure_id).await, Ok(()));
    echo_caller_id_handle.await.unwrap();
}
