use anyhow::Error;

use crate::{
    core::{
        close::CloseReason,
        types::{
            Dictionary,
            Value,
        },
        uri::Uri,
    },
    message::message::{
        AbortMessage,
        ErrorMessage,
        GoodbyeMessage,
        Message,
    },
};

pub fn abort_message_for_error(error: &Error) -> Message {
    Message::Abort(AbortMessage {
        details: Dictionary::from_iter([("message".to_owned(), Value::String(error.to_string()))]),
        reason: Uri::for_error(error),
        ..Default::default()
    })
}

pub fn goodbye_with_close_reason(close_reason: CloseReason) -> Message {
    Message::Goodbye(GoodbyeMessage {
        details: Dictionary::default(),
        reason: close_reason.uri(),
        ..Default::default()
    })
}

pub fn goodbye_and_out() -> Message {
    goodbye_with_close_reason(CloseReason::GoodbyeAndOut)
}

pub fn error_for_request(message: &Message, error: &Error) -> Message {
    Message::Error(ErrorMessage {
        request_type: message.tag(),
        request: message.request_id().unwrap_or_default(),
        details: Dictionary::from_iter([("message".to_owned(), Value::String(error.to_string()))]),
        error: Uri::for_error(error),
        ..Default::default()
    })
}
