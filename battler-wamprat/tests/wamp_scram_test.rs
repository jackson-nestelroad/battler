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
        error::{
            BasicError,
            InteractionError,
        },
        hash::HashMap,
    },
    peer::{
        PeerConfig,
        SupportedAuthMethod,
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
use battler_wamp_uri::Uri;
use battler_wamp_values::{
    Integer,
    WampList,
};
use battler_wamprat::{
    peer::{
        CallOptions,
        PeerBuilder,
        PeerConnectionType,
    },
    procedure::{
        Invocation,
        TypedProcedure,
    },
};
use battler_wamprat_message::WampApplicationMessage;
use tokio::task::JoinHandle;

const REALM: &str = "com.battler.test";

async fn start_router(
    user_database_factory: Box<dyn UserDatabaseFactory>,
) -> Result<(RouterHandle, JoinHandle<()>)> {
    let router = new_web_socket_router(
        RouterConfig {
            realms: Vec::from_iter([RealmConfig {
                name: "test".to_owned(),
                uri: Uri::try_from(REALM)?,
                authentication: RealmAuthenticationConfig {
                    required: false,
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

impl TypedProcedure for AddHandler {
    type Input = AddInput;
    type Output = AddOutput;
    type Error = anyhow::Error;
    async fn invoke(&self, invocation: Invocation, input: Self::Input) -> Result<Self::Output> {
        if invocation.peer_info.identity.id != "battler-wamprat-test-user" {
            return Err(BasicError::NotAllowed(format!(
                "unexpected user: {}",
                invocation.peer_info.identity.id
            ))
            .into());
        }

        let sum = input.args.a + input.args.b;
        Ok(AddOutput {
            args: SumArgs { sum },
        })
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn procedure_receives_identity_from_wamp_scram_authentication() {
    test_utils::setup::setup_test_environment();

    // Start a router.
    let (router_handle, _) = start_router(Box::new(FakeUserDatabaseFactory::from_iter([(
        "battler-wamprat-test-user",
        "password",
    )])))
    .await
    .unwrap();

    // Create a callee with that exposes an addition RPC.
    let mut peer_builder = PeerBuilder::new(PeerConnectionType::Remote(format!(
        "ws://{}",
        router_handle.local_addr()
    )));
    peer_builder.add_procedure(Uri::try_from("com.battler.add2").unwrap(), AddHandler);
    let (callee_handle, _) = peer_builder.start(
        create_peer("callee").unwrap(),
        Uri::try_from(REALM).unwrap(),
    );

    // Must wait until the procedure is registered, since it happens asynchronously.
    callee_handle.wait_until_ready().await.unwrap();

    // Create a caller.

    let mut peer_builder = PeerBuilder::new(PeerConnectionType::Remote(format!(
        "ws://{}",
        router_handle.local_addr()
    )));
    peer_builder.set_auth_methods([SupportedAuthMethod::WampScram {
        id: "battler-wamprat-test-user".to_owned(),
        password: "password".to_owned(),
    }]);
    let (caller_handle, _) = peer_builder.start(
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
}
