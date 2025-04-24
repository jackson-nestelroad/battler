use anyhow::Result;
use async_trait::async_trait;
use futures_util::lock::Mutex;

use crate::{
    auth::{
        Identity,
        auth_method::AuthMethod,
        authenticator::{
            ClientAuthenticator as ClientAuthenticatorInterface,
            ServerAuthenticator as ServerAuthenticatorInterface,
        },
        undisputed::{
            UserData,
            message::{
                ClientFinalMessage,
                ClientFinalMessageExtra,
                ClientFirstMessage,
                ClientFirstMessageExtra,
                ServerFinalMessage,
                ServerFinalMessageExtra,
                ServerFirstMessage,
                ServerFirstMessageExtra,
            },
        },
    },
    core::{
        error::InteractionError,
        hash::HashSet,
    },
};

/// Server authenticator for WAMP-SCRAM.
pub struct ServerAuthenticator {
    user: Mutex<Option<UserData>>,
}

impl ServerAuthenticator {
    /// Creates a new server authenticator.
    pub fn new() -> Self {
        Self {
            user: Mutex::new(None),
        }
    }
}

#[async_trait]
impl
    ServerAuthenticatorInterface<
        ClientFirstMessageExtra,
        ServerFirstMessageExtra,
        ClientFinalMessageExtra,
        ServerFinalMessageExtra,
    > for ServerAuthenticator
{
    fn auth_method(&self) -> AuthMethod {
        AuthMethod::Undisputed
    }

    async fn challenge(&self, message: ClientFirstMessage) -> Result<ServerFirstMessage> {
        let user = UserData {
            identity: Identity {
                id: message.id.clone(),
                role: message.extra.role.clone(),
            },
        };
        *self.user.lock().await = Some(user);
        Ok(ServerFirstMessage {
            method: self.auth_method(),
            extra: ServerFirstMessageExtra {},
        })
    }

    async fn authenticate(&self, _: ClientFinalMessage) -> Result<ServerFinalMessage> {
        let user = self.user.lock().await;
        let user = user.as_ref().ok_or_else(|| {
            InteractionError::AuthenticationFailed("expected pending user".to_owned())
        })?;
        Ok(ServerFinalMessage {
            identity: user.identity.clone(),
            method: self.auth_method(),
            provider: "static".to_owned(),
            extra: ServerFinalMessageExtra {},
        })
    }
}

/// Client authenticator for WAMP-SCRAM.
pub struct ClientAuthenticator {
    id: String,
    role: String,
}

impl ClientAuthenticator {
    /// Creates a new client authenticator.
    pub fn new(id: String, role: String) -> Self {
        Self { id, role }
    }
}

#[async_trait]
impl
    ClientAuthenticatorInterface<
        ClientFirstMessageExtra,
        ServerFirstMessageExtra,
        ClientFinalMessageExtra,
        ServerFinalMessageExtra,
    > for ClientAuthenticator
{
    fn auth_method(&self) -> AuthMethod {
        AuthMethod::Undisputed
    }

    async fn hello(&self) -> Result<ClientFirstMessage> {
        Ok(ClientFirstMessage {
            id: self.id.clone(),
            methods: HashSet::from_iter([self.auth_method()]),
            extra: ClientFirstMessageExtra {
                role: self.role.clone(),
            },
        })
    }

    async fn handle_challenge(&self, _: ServerFirstMessage) -> Result<ClientFinalMessage> {
        Ok(ClientFinalMessage {
            signature: "not_applicable".to_owned(),
            extra: ClientFinalMessageExtra {},
        })
    }

    async fn verify_signature(&self, _: ServerFinalMessage) -> Result<()> {
        Ok(())
    }
}

#[cfg(test)]
mod scram_test {
    use anyhow::Result;
    use async_trait::async_trait;

    use crate::{
        auth::{
            authenticator::{
                ClientAuthenticator,
                ServerAuthenticator,
            },
            scram::{
                authenticator::{
                    ClientAuthenticator as ScramClientAuthenticator,
                    ServerAuthenticator as ScramServerAuthenticator,
                },
                user::{
                    UserData,
                    UserDatabase,
                    new_user,
                },
            },
        },
        core::{
            error::InteractionError,
            hash::HashMap,
        },
    };

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

    #[tokio::test]
    async fn client_and_server_authenticate_correctly() {
        let server_authenticator = ScramServerAuthenticator::new(Box::new(
            FakeUserDatabase::from_iter([("user", "password123!")]),
        ));
        let client_authenticator =
            ScramClientAuthenticator::new("user".to_owned(), "password123!".to_owned());

        let client_first = client_authenticator.hello().await.unwrap();
        let server_first = server_authenticator.challenge(client_first).await.unwrap();
        let client_final = client_authenticator
            .handle_challenge(server_first)
            .await
            .unwrap();
        let server_final = server_authenticator
            .authenticate(client_final)
            .await
            .unwrap();
        assert_matches::assert_matches!(
            client_authenticator.verify_signature(server_final).await,
            Ok(())
        );
    }

    #[tokio::test]
    async fn authentication_fails_for_invalid_password() {
        let server_authenticator = ScramServerAuthenticator::new(Box::new(
            FakeUserDatabase::from_iter([("user", "password123!")]),
        ));
        let client_authenticator =
            ScramClientAuthenticator::new("user".to_owned(), "wrong".to_owned());

        let client_first = client_authenticator.hello().await.unwrap();
        let server_first = server_authenticator.challenge(client_first).await.unwrap();
        let client_final = client_authenticator
            .handle_challenge(server_first)
            .await
            .unwrap();
        assert_matches::assert_matches!(server_authenticator.authenticate(client_final).await, Err(err) => {
            assert_matches::assert_matches!(err.downcast::<InteractionError>(), Ok(InteractionError::AuthenticationDenied(_)));
        });
    }

    #[tokio::test]
    async fn authentication_fails_for_invalid_user() {
        let server_authenticator = ScramServerAuthenticator::new(Box::new(
            FakeUserDatabase::from_iter([("user", "password123!")]),
        ));
        let client_authenticator =
            ScramClientAuthenticator::new("another".to_owned(), "password123!".to_owned());

        let client_first = client_authenticator.hello().await.unwrap();
        assert_matches::assert_matches!(server_authenticator.challenge(client_first).await, Err(err) => {
            assert_matches::assert_matches!(err.downcast::<InteractionError>(), Ok(InteractionError::NoSuchPrincipal));
        });
    }
}
