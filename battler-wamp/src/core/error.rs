use anyhow::Error;
use thiserror::Error;

use super::types::Value;
use crate::{
    core::uri::Uri,
    message::message::Message,
};

#[derive(Debug, Error)]
pub enum BasicError {
    #[error("not found: {0}")]
    NotFound(String),
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    #[error("internal error: {0}")]
    Internal(String),
}

impl BasicError {
    pub fn uri_component(&self) -> &str {
        match self {
            Self::NotFound(_) => "not_found",
            Self::InvalidArgument(_) => "invalid_argument",
            Self::Internal(_) => "internal",
        }
    }
}

#[derive(Debug, Error)]
pub enum InteractionError {
    #[error("protocol violation: {0}")]
    ProtocolViolation(String),
    #[error("no such procedure")]
    NoSuchProcedure,
    #[error("procedure already exists")]
    ProcedureAlreadyExists,
    #[error("no such registration")]
    NoSuchRegistration,
    #[error("no such subscription")]
    NoSuchSubscription,
    #[error("no such realm")]
    NoSuchRealm,
    #[error("no such role")]
    NoSuchRole,
}

impl InteractionError {
    pub fn uri_component(&self) -> &str {
        match self {
            Self::ProtocolViolation(_) => "protocol_violation",
            Self::NoSuchProcedure => "no_such_procedure",
            Self::ProcedureAlreadyExists => "procedure_already_exists",
            Self::NoSuchRegistration => "no_such_registration",
            Self::NoSuchSubscription => "no_such_subscription",
            Self::NoSuchRealm => "no_such_realm",
            Self::NoSuchRole => "no_such_role",
        }
    }
}

pub fn error_from_uri_reason_and_message(reason: Uri, message: String) -> Error {
    match reason.as_ref() {
        "wamp.error.not_found" => BasicError::NotFound(message).into(),
        "wamp.error.invalid_argument" => BasicError::InvalidArgument(message).into(),
        "wamp.error.protocol_violation" => InteractionError::ProtocolViolation(message).into(),
        "wamp.error.no_such_procedure" => InteractionError::NoSuchProcedure.into(),
        "wamp.error.procedure_already_exists" => InteractionError::ProcedureAlreadyExists.into(),
        "wamp.error.no_such_registration" => InteractionError::NoSuchRegistration.into(),
        "wamp.error.no_such_subscription" => InteractionError::NoSuchSubscription.into(),
        "wamp.error.no_such_realm" => InteractionError::NoSuchRealm.into(),
        "wamp.error.no_such_role" => InteractionError::NoSuchRole.into(),
        _ => BasicError::Internal(message).into(),
    }
}

pub fn extract_error_uri_reason_and_message(message: &Message) -> Result<(&Uri, &str), Error> {
    let reason = match message.reason() {
        Some(reason) => reason,
        None => return Err(Error::msg("message does not contain a reason uri")),
    };
    let message = match message
        .details()
        .map(|details| details.get("message"))
        .flatten()
    {
        Some(Value::String(message)) => message.as_str(),
        _ => "unknown error",
    };
    Ok((reason, message))
}

pub fn error_from_message(message: &Message) -> Result<Error, Error> {
    let (uri, message) = extract_error_uri_reason_and_message(message)?;
    Ok(error_from_uri_reason_and_message(
        uri.clone(),
        message.to_owned(),
    ))
}
