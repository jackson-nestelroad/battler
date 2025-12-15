use anyhow::{
    Error,
    Result,
};
use battler_wamp_uri::{
    InvalidUri,
    Uri,
};
use battler_wamp_values::Value;
use thiserror::Error;

use crate::{
    core::id::Id,
    message::message::Message,
    peer::PeerNotConnectedError,
};

/// A generic WAMP error.
///
/// Errors that are passed into the library by applications must use this type, which can be
/// consistently communicated via an ERROR message.
#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("{message}")]
pub struct WampError {
    reason: Uri,
    message: String,
}

impl WampError {
    /// Creates a new WAMP error.
    pub fn new<S>(reason: Uri, message: S) -> Self
    where
        S: Into<String>,
    {
        Self {
            reason,
            message: message.into(),
        }
    }

    /// The reason URI.
    pub fn reason(&self) -> &Uri {
        &self.reason
    }

    /// The message explaining the error.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl Into<WampError> for Error {
    fn into(self) -> WampError {
        WampError {
            reason: uri_for_error(&self),
            message: self.to_string(),
        }
    }
}

impl Into<WampError> for &Error {
    fn into(self) -> WampError {
        WampError {
            reason: uri_for_error(self),
            message: self.to_string(),
        }
    }
}

/// A basic error that occurs while processing a WAMP message.
#[derive(Debug, Error)]
pub enum BasicError {
    /// A generic resource was not found.
    ///
    /// WAMP defines standard URIs for not finding specific resource types. This error should only
    /// be used when the standard URI cannot be used.
    #[error("{0}")]
    NotFound(String),
    /// An invalid argument was passed.
    #[error("{0}")]
    InvalidArgument(String),
    /// The operation is not allowed based on process configuration.
    #[error("{0}")]
    NotAllowed(String),
    /// The operation is not allowed based on user permissions.
    #[error("{0}")]
    PermissionDenied(String),
    /// Some internal error occurred.
    ///
    /// Should only be used when there is no other error variant that describes the error, since
    /// the message is very vague and not very useful for debugging.
    #[error("{0}")]
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

impl Into<WampError> for BasicError {
    fn into(self) -> WampError {
        WampError::new(
            Uri::from_known(format!("wamp.error.{}", self.uri_component())),
            self.to_string(),
        )
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
    /// The principal being referenced does not exist.
    #[error("no such principal")]
    NoSuchPrincipal,
    /// No authentication method the client offered is accepted.
    #[error("no matching auth method")]
    NoMatchingAuthMethod,
    /// The authentication as presented by the client is denied.
    #[error("authentication denied: {0}")]
    AuthenticationDenied(String),
    /// The authentication was rejected due to a technical runtime failure.
    #[error("authentication failed: {0}")]
    AuthenticationFailed(String),
    /// The client did not provide the required, non-anonymous, authentication information.
    #[error("authentication required")]
    AuthenticationRequired,
    /// A procedure call was canceled due to the callee leaving.
    #[error("canceled")]
    Canceled,
    /// A procedure call timed out.
    #[error("timeout")]
    Timeout,
    /// A callee is unavailable to handle an invocation.
    #[error("unavailable")]
    Unavailable,
    /// There is no available callee to handle a procedure call.
    #[error("no available callee")]
    NoAvailableCallee,
}

impl InteractionError {
    fn uri_component(&self) -> &str {
        match self {
            Self::ProtocolViolation(_) => "protocol_violation",
            Self::NoSuchProcedure => "no_such_procedure",
            Self::ProcedureAlreadyExists => "procedure_already_exists",
            Self::NoSuchRegistration => "no_such_registration",
            Self::NoSuchSubscription => "no_such_subscription",
            Self::NoSuchRealm => "no_such_realm",
            Self::NoSuchRole => "no_such_role",
            Self::NoSuchPrincipal => "no_such_principal",
            Self::NoMatchingAuthMethod => "no_matching_auth_method",
            Self::AuthenticationDenied(_) => "authentication_denied",
            Self::AuthenticationFailed(_) => "authentication_failed",
            Self::AuthenticationRequired => "authentication_required",
            Self::Canceled => "canceled",
            Self::Timeout => "timeout",
            Self::Unavailable => "unavailable",
            Self::NoAvailableCallee => "no_available_callee",
        }
    }
}

impl Into<WampError> for InteractionError {
    fn into(self) -> WampError {
        WampError::new(
            Uri::from_known(format!("wamp.error.{}", self.uri_component())),
            self.to_string(),
        )
    }
}

/// Creates an [`struct@Error`] from a URI error reason and message.
///
/// Standard WAMP errors are converted to the correct variant of [`BasicError`] or
/// [`InteractionError`]. Otherwise, the reason and message are stored in [`WampError`].
fn error_from_uri_reason_and_message(reason: Uri, message: String) -> Error {
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
        "wamp.error.no_such_principal" => InteractionError::NoSuchPrincipal.into(),
        "wamp.error.no_matching_auth_method" => InteractionError::NoMatchingAuthMethod.into(),
        "wamp.error.authentication_denied" => {
            InteractionError::AuthenticationDenied(message).into()
        }
        "wamp.error.authentication_failed" => {
            InteractionError::AuthenticationFailed(message).into()
        }
        "wamp.error.authentication_required" => InteractionError::AuthenticationRequired.into(),
        "wamp.error.canceled" => InteractionError::Canceled.into(),
        "wamp.error.timeout" => InteractionError::Timeout.into(),
        "wamp.error.unavailable" => InteractionError::Unavailable.into(),
        "wamp.error.no_available_callee" => InteractionError::NoAvailableCallee.into(),
        "wamp.error.invalid_uri" => InvalidUri.into(),
        "com.battler_wamp.peer_not_connected" => PeerNotConnectedError.into(),
        _ => WampError::new(reason, message).into(),
    }
}

/// Extracts a URI error reason and message from a WAMP message.
fn extract_error_uri_reason_and_message(message: &Message) -> Result<(&Uri, &str), Error> {
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

impl Into<Error> for &Message {
    fn into(self) -> Error {
        match extract_error_uri_reason_and_message(self) {
            Ok((reason, message)) => {
                error_from_uri_reason_and_message(reason.clone(), message.to_owned())
            }
            Err(err) => err.context("message does not contain any error"),
        }
    }
}

/// An error that can be transmitted over channels.
///
/// Maintains a [`WampError`] (reason URI and message), as well as a request ID for connecting
/// errors to individual requests.
#[derive(Debug, Clone)]
pub struct ChannelTransmittableError {
    pub error: WampError,
    pub request_id: Option<Id>,
}

// Manual implementation, rather than using [`thiserror::Error`], to ensure we maintain URI reason
// in the error type.
impl Into<Error> for ChannelTransmittableError {
    fn into(self) -> Error {
        error_from_uri_reason_and_message(self.error.reason, self.error.message)
    }
}

impl TryFrom<&Message> for ChannelTransmittableError {
    type Error = Error;
    fn try_from(value: &Message) -> std::result::Result<Self, Self::Error> {
        let (reason, message) = extract_error_uri_reason_and_message(&value)?;
        Ok(Self {
            error: WampError::new(reason.clone(), message),
            request_id: value.request_id(),
        })
    }
}

impl From<&Error> for ChannelTransmittableError {
    fn from(value: &Error) -> Self {
        // We must maintain URIs as much as possible inside and outside the library.
        Self {
            error: WampError::new(uri_for_error(value), value.to_string()),
            request_id: None,
        }
    }
}

impl From<Error> for ChannelTransmittableError {
    fn from(value: Error) -> Self {
        Self::from(&value)
    }
}

/// Type alias for a channel-transmittable result.
///
/// Assumes `T` is channel-transmittable.
pub type ChannelTransmittableResult<T> = Result<T, ChannelTransmittableError>;

/// Creates a URI for a generic error, generated within the WAMP library.
pub(crate) fn uri_for_error(error: &Error) -> Uri {
    if error.is::<InvalidUri>() {
        Uri::from_known("wamp.error.invalid_uri")
    } else if let Some(error) = error.downcast_ref::<BasicError>() {
        Uri::from_known(format!("wamp.error.{}", error.uri_component()))
    } else if let Some(error) = error.downcast_ref::<InteractionError>() {
        Uri::from_known(format!("wamp.error.{}", error.uri_component()))
    } else if error.is::<tokio::sync::broadcast::error::SendError<Message>>() {
        Uri::from_known("com.battler_wamp.send_error")
    } else if error.is::<tokio::sync::broadcast::error::RecvError>() {
        Uri::from_known("com.battler_wamp.recv_error")
    } else if error.is::<PeerNotConnectedError>() {
        Uri::from_known("com.battler_wamp.peer_not_connected")
    } else if let Some(error) = error.downcast_ref::<WampError>() {
        error.reason.clone()
    } else {
        Uri::from_known("com.battler_wamp.unknown_error")
    }
}
