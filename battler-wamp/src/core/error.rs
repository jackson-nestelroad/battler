use anyhow::Error;
use thiserror::Error;

use super::types::Value;
use crate::{
    core::uri::Uri,
    message::message::Message,
};

/// A basic error that occurs while processing a WAMP message.
#[derive(Debug, Error)]
pub enum BasicError {
    /// A generic resource was not found.
    ///
    /// WAMP defines standard URIs for not finding specific resource types. This error should only
    /// be used when the standard URI cannot be used.
    #[error("not found: {0}")]
    NotFound(String),
    /// An invalid argument was passed.
    #[error("invalid argument: {0}")]
    InvalidArgument(String),
    /// The operation is not allowed based on process configuration.
    #[error("not allowed: {0}")]
    NotAllowed(String),
    /// The operation is not allowed based on user permissions.
    #[error("permission denied: {0}")]
    PermissionDenied(String),
    /// Some internal error occurred.
    ///
    /// Should only be used when there is no other error variant that descibes the error, since the
    /// message is very vague and not very useful for debugging.
    #[error("internal error: {0}")]
    Internal(String),
}

impl BasicError {
    /// The trailing URI component for the error.
    pub fn uri_component(&self) -> &str {
        match self {
            Self::NotFound(_) => "not_found",
            Self::InvalidArgument(_) => "invalid_argument",
            Self::NotAllowed(_) => "not_allowed",
            Self::PermissionDenied(_) => "permission_denied",
            Self::Internal(_) => "internal",
        }
    }
}

/// An interaction error that occurs while processing a WAMP message.
///
/// Interaction errors are clearly defined in the WAMP standard and are reserved for errors that
/// peers must be able to parse easily.
#[derive(Debug, Error)]
pub enum InteractionError {
    /// The incoming message violates the WAMP protocol.
    #[error("protocol violation: {0}")]
    ProtocolViolation(String),
    /// The procedure being called does not exist.
    #[error("no such procedure")]
    NoSuchProcedure,
    /// The procedure being registered already exists.
    #[error("procedure already exists")]
    ProcedureAlreadyExists,
    /// The registration being referenced does not exist.
    #[error("no such registration")]
    NoSuchRegistration,
    /// The subscription being referenced does not exist.
    #[error("no such subscription")]
    NoSuchSubscription,
    /// The realm being referenced does not exist.
    #[error("no such realm")]
    NoSuchRealm,
    /// The role being referenced does not exist.
    #[error("no such role")]
    NoSuchRole,
}

impl InteractionError {
    /// The trailing URI component for the error.
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

/// Creates an [`Error`] from a URI error reason and messsage.
pub fn error_from_uri_reason_and_message(reason: Uri, message: String) -> Error {
    match reason.as_ref() {
        "wamp.error.not_found" => BasicError::NotFound(message).into(),
        "wamp.error.invalid_argument" => BasicError::InvalidArgument(message).into(),
        "wamp.error.not_allowed" => BasicError::NotAllowed(message).into(),
        "wamp.error.permission_denied" => BasicError::PermissionDenied(message).into(),
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

/// Extracts a URI error reason and message from a WAMP message.
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

/// Constructs an [`Error`] from a WAMP message.
///
/// Fails if the message does not describe any error.
pub fn error_from_message(message: &Message) -> Result<Error, Error> {
    let (uri, message) = extract_error_uri_reason_and_message(message)?;
    Ok(error_from_uri_reason_and_message(
        uri.clone(),
        message.to_owned(),
    ))
}
