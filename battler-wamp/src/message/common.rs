use anyhow::Error;
use battler_wamp_values::{
    Dictionary,
    Value,
};

use crate::{
    core::{
        close::CloseReason,
        uri::Uri,
    },
    message::message::{
        AbortMessage,
        ErrorMessage,
        GoodbyeMessage,
        Message,
    },
};

/// Generates an ABORT message for an error.
pub fn abort_message_for_error(error: &Error) -> Message {
    Message::Abort(AbortMessage {
        details: Dictionary::from_iter([("message".to_owned(), Value::String(error.to_string()))]),
        reason: Uri::for_error(error),
        ..Default::default()
    })
}

/// Generates a GOODBYE message with a close reason.
pub fn goodbye_with_close_reason(close_reason: CloseReason) -> Message {
    Message::Goodbye(GoodbyeMessage {
        details: Dictionary::default(),
        reason: close_reason.uri(),
        ..Default::default()
    })
}

/// Generates the generic GOODBYE response message.
pub fn goodbye_and_out() -> Message {
    goodbye_with_close_reason(CloseReason::GoodbyeAndOut)
}

/// Generates an ERROR message in response to a request.
pub fn error_for_request(message: &Message, error: &Error) -> Message {
    Message::Error(ErrorMessage {
        request_type: message.tag(),
        request: message.request_id().unwrap_or_default(),
        details: Dictionary::from_iter([("message".to_owned(), Value::String(error.to_string()))]),
        error: Uri::for_error(error),
        ..Default::default()
    })
}
