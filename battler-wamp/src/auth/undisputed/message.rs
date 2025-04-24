use battler_wamp_values::WampDictionary;

use crate::auth::message::{
    ClientFinalMessage as GenericClientFinalMessage,
    ClientFirstMessage as GenericClientFirstMessage,
    ServerFinalMessage as GenericServerFinalMessage,
    ServerFirstMessage as GenericServerFirstMessage,
};

/// The extra data for the client's first message of the undisputed authentication method.
#[derive(Debug, WampDictionary)]
pub struct ClientFirstMessageExtra {
    pub role: String,
}

/// The extra data for the server's first message of the undisputed authentication method.
#[derive(Debug, WampDictionary)]
pub struct ServerFirstMessageExtra {}

/// The extra data for the client's final message of the undisputed authentication method.
#[derive(Debug, WampDictionary)]
pub struct ClientFinalMessageExtra {}

/// The extra data for the server's final message of the undisputed authentication method.
#[derive(Debug, WampDictionary)]
pub struct ServerFinalMessageExtra {}

pub type ClientFirstMessage = GenericClientFirstMessage<ClientFirstMessageExtra>;
pub type ServerFirstMessage = GenericServerFirstMessage<ServerFirstMessageExtra>;
pub type ClientFinalMessage = GenericClientFinalMessage<ClientFinalMessageExtra>;
pub type ServerFinalMessage = GenericServerFinalMessage<ServerFinalMessageExtra>;
