use anyhow::{
    Error,
    Result,
};
use async_trait::async_trait;
use base64::Engine;
use futures_util::lock::Mutex;
use password_hash::Salt;
use rand::Rng;

use crate::{
    auth::{
        auth_method::AuthMethod,
        authenticator::{
            ClientAuthenticator as ClientAuthenticatorInterface,
            ServerAuthenticator as ServerAuthenticatorInterface,
        },
        scram::{
            core::{
                auth_message,
                client_key,
                client_proof,
                client_signature,
                recovered_client_key,
                salt_password,
                server_key,
                server_signature,
                stored_key,
            },
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
            user::{
                UserData,
                UserDatabase,
            },
        },
    },
    core::{
        error::{
            BasicError,
            InteractionError,
        },
        hash::HashSet,
    },
};

fn generate_nonce() -> Vec<u8> {
    (0..16)
        .map(|_| rand::rng().sample(rand::distr::Alphanumeric))
        .collect()
}

/// Server authenticator for WAMP-SCRAM.
pub struct ServerAuthenticator {
    user_database: Box<dyn UserDatabase>,

    nonce: String,
    user: Mutex<Option<UserData>>,
}

impl ServerAuthenticator {
    /// Creates a new server authenticator.
    pub fn new(user_database: Box<dyn UserDatabase>) -> Self {
        Self {
            user_database,
            nonce: generate_nonce().into_iter().map(|c| c as char).collect(),
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
        AuthMethod::WampScram
    }

    async fn challenge(&self, message: ClientFirstMessage) -> Result<ServerFirstMessage> {
        let user = self
            .user_database
            .user_data(&message.id)
            .await
            .map_err(|err| match err.downcast::<BasicError>() {
                Ok(BasicError::NotFound(_)) => InteractionError::NoSuchPrincipal.into(),
                Ok(err) => err.into(),
                Err(err) => err,
            })?;
        *self.user.lock().await = Some(user.clone());
        Ok(ServerFirstMessage {
            method: self.auth_method(),
            extra: ServerFirstMessageExtra {
                nonce: format!("{}{}", message.extra.nonce, self.nonce),
                salt: user.salt.to_string(),
                kdf: user.key_derivation_function,
                iterations: user.iterations as u64,
                memory: user.memory.map(|n| n as u64),
            },
        })
    }

    async fn authenticate(&self, message: ClientFinalMessage) -> Result<ServerFinalMessage> {
        let user = self.user.lock().await;
        let user = user.as_ref().ok_or_else(|| {
            InteractionError::AuthenticationFailed("expected pending user".to_owned())
        })?;
        let client_proof = base64::prelude::BASE64_STANDARD.decode(message.signature)?;
        let client_nonce = message
            .extra
            .nonce
            .strip_suffix(&self.nonce)
            .ok_or_else(|| InteractionError::AuthenticationDenied("invalid nonce".to_owned()))?;
        let auth_message = auth_message(
            &user.identity.id,
            client_nonce,
            &self.nonce,
            user.salt.as_str(),
            user.iterations,
            message.extra.channel_binding,
            message.extra.cbind_data.as_ref().map(|s| s.as_str()),
        )?;
        let client_signature = client_signature(&user.stored_key, auth_message.as_bytes())?;
        let recovered_client_key = recovered_client_key(&client_signature, &client_proof);
        let recovered_stored_key = stored_key(&recovered_client_key)?;

        if recovered_stored_key != user.stored_key {
            return Err(
                InteractionError::AuthenticationDenied("invalid password".to_owned()).into(),
            );
        }

        let server_signature = server_signature(&user.server_key, auth_message.as_bytes())?;
        let verifier = base64::prelude::BASE64_STANDARD.encode(server_signature);
        Ok(ServerFinalMessage {
            identity: user.identity.clone(),
            method: self.auth_method(),
            provider: "static".to_owned(),
            extra: ServerFinalMessageExtra { verifier },
        })
    }
}

struct ClientChallengeResponse {
    salted_password: String,
    auth_message: String,
}

/// Client authenticator for WAMP-SCRAM.
pub struct ClientAuthenticator {
    id: String,
    password: String,

    nonce: String,
    response: Mutex<Option<ClientChallengeResponse>>,
}

impl ClientAuthenticator {
    /// Creates a new client authenticator.
    pub fn new(id: String, password: String) -> Self {
        Self {
            id,
            password,
            nonce: generate_nonce().into_iter().map(|c| c as char).collect(),
            response: Mutex::new(None),
        }
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
        AuthMethod::WampScram
    }

    async fn hello(&self) -> Result<ClientFirstMessage> {
        Ok(ClientFirstMessage {
            id: self.id.clone(),
            methods: HashSet::from_iter([self.auth_method()]),
            extra: ClientFirstMessageExtra {
                nonce: self.nonce.clone(),
                channel_binding: None,
            },
        })
    }

    async fn handle_challenge(&self, message: ServerFirstMessage) -> Result<ClientFinalMessage> {
        let salted_password = salt_password(
            &self.password,
            Salt::from_b64(&message.extra.salt)
                .map_err(|err| Error::msg(format!("invalid salt: {err:?}")))?,
            message.extra.kdf,
            message.extra.iterations.try_into()?,
            message
                .extra
                .memory
                .map(|n| n.try_into())
                .map_or(Ok(None), |v| v.map(Some))?,
        )?;
        let client_key = client_key(salted_password.as_bytes())?;
        let stored_key = stored_key(&client_key)?;
        let server_nonce = message
            .extra
            .nonce
            .strip_prefix(&self.nonce)
            .ok_or_else(|| Error::msg("invalid nonce from server"))?;
        let auth_message = auth_message(
            &self.id,
            &self.nonce,
            server_nonce,
            &message.extra.salt,
            message.extra.iterations.try_into()?,
            None,
            None,
        )?;
        let client_signature = client_signature(&stored_key, auth_message.as_bytes())?;
        let client_proof = client_proof(&client_key, &client_signature);

        let signature = base64::prelude::BASE64_STANDARD.encode(client_proof);

        *self.response.lock().await = Some(ClientChallengeResponse {
            salted_password: salted_password.to_string(),
            auth_message,
        });

        Ok(ClientFinalMessage {
            signature,
            extra: ClientFinalMessageExtra {
                nonce: message.extra.nonce.clone(),
                channel_binding: None,
                cbind_data: None,
            },
        })
    }

    async fn verify_signature(&self, message: ServerFinalMessage) -> Result<()> {
        let response = self.response.lock().await;
        let ClientChallengeResponse {
            salted_password,
            auth_message,
        } = response
            .as_ref()
            .ok_or_else(|| Error::msg("expected cached client response"))?;

        let server_key = server_key(salted_password.as_bytes())?;
        let expected_server_signature = server_signature(&server_key, auth_message.as_bytes())?;

        let server_signature = base64::prelude::BASE64_STANDARD.decode(message.extra.verifier)?;

        if server_signature != expected_server_signature {
            return Err(
                BasicError::InvalidArgument("incorrect server signature".to_owned()).into(),
            );
        }

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
