use anyhow::{
    Error,
    Result,
};
use battler_wamp_values::{
    Dictionary,
    Value,
    WampDeserialize,
    WampSerialize,
};

use crate::{
    auth::{
        auth_method::AuthMethod,
        identity::Identity,
    },
    core::hash::HashSet,
    message::message::{
        AuthenticateMessage,
        ChallengeMessage,
        HelloMessage,
        WelcomeMessage,
    },
};

/// The client's first message of the WAMP-SCRAM authentication method.
#[derive(Debug)]
pub struct ClientFirstMessage<Extra> {
    /// The identity of the user performing authentication.
    pub id: String,
    /// Supported authentication methods.
    pub methods: HashSet<AuthMethod>,
    /// Extra data.
    pub extra: Extra,
}

impl<Extra> ClientFirstMessage<Extra>
where
    Extra: WampSerialize,
{
    /// Embeds the authentication information into a HELLO message.
    pub fn embed_into_hello_message(self, message: &mut HelloMessage) -> Result<()> {
        message.details.insert(
            "authmethods".to_owned(),
            Value::List(
                self.methods
                    .into_iter()
                    .map(|method| method.wamp_serialize())
                    .collect::<Result<_, _>>()?,
            ),
        );
        message
            .details
            .insert("authid".to_owned(), Value::String(self.id));
        message
            .details
            .insert("authextra".to_owned(), self.extra.wamp_serialize()?);
        Ok(())
    }

    /// Converts the message into a generic form.
    pub fn try_into_generic(self) -> Result<ClientFirstMessage<Dictionary>> {
        let extra = self
            .extra
            .wamp_serialize()?
            .dictionary()
            .ok_or_else(|| {
                Error::msg("expected authentication extra data to serialize as a dictionary")
            })?
            .clone();
        Ok(ClientFirstMessage {
            id: self.id,
            methods: self.methods,
            extra,
        })
    }
}

impl<Extra> TryFrom<&HelloMessage> for ClientFirstMessage<Extra>
where
    Extra: WampDeserialize,
{
    type Error = Error;
    fn try_from(value: &HelloMessage) -> Result<Self, Self::Error> {
        let id = value
            .details
            .get("authid")
            .ok_or_else(|| Error::msg("missing authid"))?
            .string()
            .ok_or_else(|| Error::msg("authid must be a string"))?
            .to_owned();
        let methods = value
            .details
            .get("authmethods")
            .ok_or_else(|| Error::msg("missing authmethods"))?
            .list()
            .ok_or_else(|| Error::msg("authmethods must be a string"))?
            .clone()
            .into_iter()
            .map(|val| AuthMethod::wamp_deserialize(val))
            .collect::<Result<_, _>>()?;
        let extra = value
            .details
            .get("authextra")
            .ok_or_else(|| Error::msg("missing authextra"))?;
        let extra = Extra::wamp_deserialize(extra.clone())?;
        Ok(Self { id, methods, extra })
    }
}

/// The server's first message of the WAMP-SCRAM authentication method.
#[derive(Debug)]
pub struct ServerFirstMessage<Extra> {
    /// The selected authentication method.
    pub method: AuthMethod,
    /// Extra data.
    pub extra: Extra,
}

impl<Extra> TryInto<ChallengeMessage> for ServerFirstMessage<Extra>
where
    Extra: WampSerialize,
{
    type Error = Error;
    fn try_into(self) -> Result<ChallengeMessage, Self::Error> {
        Ok(ChallengeMessage {
            auth_method: self.method,
            extra: self
                .extra
                .wamp_serialize()?
                .dictionary()
                .ok_or_else(|| {
                    Error::msg("expected challenge extra data to serialize as a dictionary")
                })?
                .clone(),
        })
    }
}

impl<Extra> TryFrom<&ChallengeMessage> for ServerFirstMessage<Extra>
where
    Extra: WampDeserialize,
{
    type Error = Error;
    fn try_from(value: &ChallengeMessage) -> Result<Self, Self::Error> {
        let method = value.auth_method;
        let extra = Extra::wamp_deserialize(Value::Dictionary(value.extra.clone()))?;
        Ok(Self { method, extra })
    }
}

/// The client's final message of the WAMP-SCRAM authentication method.
#[derive(Debug)]
pub struct ClientFinalMessage<Extra> {
    /// Base64-encoded client proof.
    pub signature: String,
    /// Extra data.
    pub extra: Extra,
}

impl<Extra> TryInto<AuthenticateMessage> for ClientFinalMessage<Extra>
where
    Extra: WampSerialize,
{
    type Error = Error;
    fn try_into(self) -> Result<AuthenticateMessage, Self::Error> {
        Ok(AuthenticateMessage {
            signature: self.signature,
            extra: self
                .extra
                .wamp_serialize()?
                .dictionary()
                .ok_or_else(|| {
                    Error::msg("expected authenticate extra data to serialize as a dictionary")
                })?
                .clone(),
        })
    }
}

impl<Extra> TryFrom<&AuthenticateMessage> for ClientFinalMessage<Extra>
where
    Extra: WampDeserialize,
{
    type Error = Error;
    fn try_from(value: &AuthenticateMessage) -> Result<Self, Self::Error> {
        let signature = value.signature.clone();
        let extra = Extra::wamp_deserialize(Value::Dictionary(value.extra.clone()))?;
        Ok(Self { signature, extra })
    }
}

/// The server's final message of the WAMP-SCRAM authentication method.
#[derive(Debug)]
pub struct ServerFinalMessage<Extra> {
    /// The identity the client was actually authenticated as.
    pub identity: Identity,
    /// The authentication method.
    pub method: AuthMethod,
    /// The actual provider of authentication.
    pub provider: String,
    /// Extra data.
    pub extra: Extra,
}

impl<Extra> ServerFinalMessage<Extra>
where
    Extra: WampSerialize,
{
    /// Embeds the authentication information into a WELCOME message.
    pub fn embed_into_welcome_message(self, message: &mut WelcomeMessage) -> Result<()> {
        message
            .details
            .insert("authid".to_owned(), Value::String(self.identity.id));
        message
            .details
            .insert("authrole".to_owned(), Value::String(self.identity.role));
        message
            .details
            .insert("authmethod".to_owned(), self.method.wamp_serialize()?);
        message
            .details
            .insert("authprovider".to_owned(), Value::String(self.provider));
        message
            .details
            .insert("authextra".to_owned(), self.extra.wamp_serialize()?);
        Ok(())
    }

    /// Converts the message into a generic form.
    pub fn try_into_generic(self) -> Result<ServerFinalMessage<Dictionary>> {
        let extra = self
            .extra
            .wamp_serialize()?
            .dictionary()
            .ok_or_else(|| {
                Error::msg("expected authentication extra data to serialize as a dictionary")
            })?
            .clone();
        Ok(ServerFinalMessage {
            identity: self.identity,
            method: self.method,
            provider: self.provider,
            extra,
        })
    }
}

impl<Extra> TryFrom<&WelcomeMessage> for ServerFinalMessage<Extra>
where
    Extra: WampDeserialize,
{
    type Error = Error;
    fn try_from(value: &WelcomeMessage) -> Result<Self, Self::Error> {
        let id = value
            .details
            .get("authid")
            .ok_or_else(|| Error::msg("missing authid"))?
            .string()
            .ok_or_else(|| Error::msg("authid must be a string"))?
            .to_owned();
        let role = value
            .details
            .get("authrole")
            .ok_or_else(|| Error::msg("missing authrole"))?
            .string()
            .ok_or_else(|| Error::msg("authrole must be a string"))?
            .to_owned();
        let method = value
            .details
            .get("authmethod")
            .ok_or_else(|| Error::msg("missing authmethod"))?
            .clone();
        let method = AuthMethod::wamp_deserialize(method)?;
        let provider = value
            .details
            .get("authprovider")
            .ok_or_else(|| Error::msg("missing authprovider"))?
            .string()
            .ok_or_else(|| Error::msg("authprovider must be a string"))?
            .to_owned();
        let extra = value
            .details
            .get("authextra")
            .ok_or_else(|| Error::msg("missing authextra"))?;
        let extra = Extra::wamp_deserialize(extra.clone())?;
        Ok(Self {
            identity: Identity { id, role },
            method,
            provider,
            extra,
        })
    }
}
