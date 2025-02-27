use anyhow::Result;
use async_trait::async_trait;
use battler_wamp_values::{
    Dictionary,
    WampDeserialize,
    WampSerialize,
};

use crate::{
    auth::{
        auth_method::AuthMethod,
        message::{
            ClientFinalMessage,
            ClientFirstMessage,
            ServerFinalMessage,
            ServerFirstMessage,
        },
    },
    message::message::{
        AuthenticateMessage,
        ChallengeMessage,
        HelloMessage,
        WelcomeMessage,
    },
};

/// Module for handling server-side authentication for WAMP sessions.
#[async_trait]
pub trait ServerAuthenticator<
    ClientFirstMessageExtra,
    ServerFirstMessageExtra,
    ClientFinalMessageExtra,
    ServerFinalMessageExtra,
>: Send + Sync
{
    /// Authentication method.
    fn auth_method(&self) -> AuthMethod;

    /// Generates the authentication challenge to the client.
    async fn challenge(
        &self,
        message: ClientFirstMessage<ClientFirstMessageExtra>,
    ) -> Result<ServerFirstMessage<ServerFirstMessageExtra>>;

    /// Authenticates the client's response to the challenge.
    async fn authenticate(
        &self,
        message: ClientFinalMessage<ClientFinalMessageExtra>,
    ) -> Result<ServerFinalMessage<ServerFinalMessageExtra>>;
}

/// Generic version of [`ServerAuthenticator`].
#[async_trait]
pub trait GenericServerAuthenticator {
    /// Authentication method.
    fn auth_method(&self) -> AuthMethod;

    /// Generates the authentication challenge to the client.
    async fn challenge(&self, message: &HelloMessage) -> Result<ChallengeMessage>;

    /// Authenticates the client's response to the challenge.
    async fn authenticate(
        &self,
        message: &AuthenticateMessage,
    ) -> Result<ServerFinalMessage<Dictionary>>;
}

/// Creates a [`GenericServerAuthenticator`] around a concrete implementation of
/// [`ServerAuthenticator`].
pub fn make_generic_server_authenticator<A, B, C, D>(
    authenticator: Box<dyn ServerAuthenticator<A, B, C, D>>,
) -> Box<dyn GenericServerAuthenticator>
where
    A: WampDeserialize + 'static,
    B: WampSerialize + 'static,
    C: WampDeserialize + 'static,
    D: WampSerialize + 'static,
{
    struct Authenticator<A, B, C, D> {
        inner: Box<dyn ServerAuthenticator<A, B, C, D>>,
    }

    #[async_trait]
    impl<A, B, C, D> GenericServerAuthenticator for Authenticator<A, B, C, D>
    where
        A: WampDeserialize,
        B: WampSerialize,
        C: WampDeserialize,
        D: WampSerialize,
    {
        fn auth_method(&self) -> AuthMethod {
            self.inner.auth_method()
        }

        async fn challenge(&self, message: &HelloMessage) -> Result<ChallengeMessage> {
            let client_first = ClientFirstMessage::try_from(message)?;
            let server_first = self.inner.challenge(client_first).await?;
            server_first.try_into()
        }

        async fn authenticate(
            &self,
            message: &AuthenticateMessage,
        ) -> Result<ServerFinalMessage<Dictionary>> {
            let client_final = ClientFinalMessage::try_from(message)?;
            let server_final = self.inner.authenticate(client_final).await?;
            server_final.try_into_generic()
        }
    }

    Box::new(Authenticator {
        inner: authenticator,
    })
}

/// Module for handling client-side authentication for WAMP sessions.
#[async_trait]
pub trait ClientAuthenticator<
    ClientFirstMessageExtra,
    ServerFirstMessageExtra,
    ClientFinalMessageExtra,
    ServerFinalMessageExtra,
>: Send + Sync
{
    /// Authentication method.
    fn auth_method(&self) -> AuthMethod;

    /// Generates the client's first message for authentication.
    async fn hello(&self) -> Result<ClientFirstMessage<ClientFirstMessageExtra>>;

    /// Handles the server's authentication challenge.
    async fn handle_challenge(
        &self,
        message: ServerFirstMessage<ServerFirstMessageExtra>,
    ) -> Result<ClientFinalMessage<ClientFinalMessageExtra>>;

    /// Verifies the server's signature.
    async fn verify_signature(
        &self,
        message: ServerFinalMessage<ServerFinalMessageExtra>,
    ) -> Result<()>;
}

/// Generic version of [`ClientAuthenticator`].
#[async_trait]
pub trait GenericClientAuthenticator {
    /// Authentication method.
    fn auth_method(&self) -> AuthMethod;

    /// Generates the client's first message for authentication.
    async fn hello(&self) -> Result<ClientFirstMessage<Dictionary>>;

    /// Handles the server's authentication challenge.
    async fn handle_challenge(&self, message: &ChallengeMessage) -> Result<AuthenticateMessage>;

    /// Verifies the server's signature.
    async fn verify_signature(&self, message: &WelcomeMessage) -> Result<()>;
}

/// Creates a [`GenericClientAuthenticator`] around a concrete implementation of
/// [`ClientAuthenticator`].
pub fn make_generic_client_authenticator<A, B, C, D>(
    authenticator: Box<dyn ClientAuthenticator<A, B, C, D>>,
) -> Box<dyn GenericClientAuthenticator>
where
    A: WampSerialize + 'static,
    B: WampDeserialize + 'static,
    C: WampSerialize + 'static,
    D: WampDeserialize + 'static,
{
    struct Authenticator<A, B, C, D> {
        inner: Box<dyn ClientAuthenticator<A, B, C, D>>,
    }

    #[async_trait]
    impl<A, B, C, D> GenericClientAuthenticator for Authenticator<A, B, C, D>
    where
        A: WampSerialize,
        B: WampDeserialize,
        C: WampSerialize,
        D: WampDeserialize,
    {
        fn auth_method(&self) -> AuthMethod {
            self.inner.auth_method()
        }

        async fn hello(&self) -> Result<ClientFirstMessage<Dictionary>> {
            let client_first = self.inner.hello().await?;
            client_first.try_into_generic()
        }

        async fn handle_challenge(
            &self,
            message: &ChallengeMessage,
        ) -> Result<AuthenticateMessage> {
            let server_first = ServerFirstMessage::try_from(message)?;
            let client_final = self.inner.handle_challenge(server_first).await?;
            client_final.try_into()
        }

        async fn verify_signature(&self, message: &WelcomeMessage) -> Result<()> {
            let server_final = ServerFinalMessage::try_from(message)?;
            self.inner.verify_signature(server_final).await
        }
    }

    Box::new(Authenticator {
        inner: authenticator,
    })
}
