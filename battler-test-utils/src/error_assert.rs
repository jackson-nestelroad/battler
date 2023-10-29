use std::fmt::Debug;

use battler::common::Error;

/// [`assert`]s that the result is an [`Error`] with the given message.
#[track_caller]
pub fn assert_error_message<T>(result: Result<T, Error>, message: &str)
where
    T: Debug,
{
    assert!(
        result.as_ref().is_err_and(|err| err.message() == message),
        "result is {result:?}, not an error with message \"{message}\"",
    )
}

/// [`assert`]s that the result is an [`Error`] that contains the given message.
#[track_caller]
pub fn assert_error_message_contains<T>(result: Result<T, Error>, message: &str)
where
    T: Debug,
{
    assert!(
        result
            .as_ref()
            .is_err_and(|err| err.message().contains(message)),
        "result is {result:?}, not an error with a message that contains \"{message}\""
    )
}
