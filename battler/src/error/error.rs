use alloc::format;
use core::fmt::{
    Arguments,
    Display,
};

use anyhow::Error;

use crate::error::{
    context::ContextError,
    general_error,
    not_found_error,
};

/// Wraps an error into [`Error`], optionally providing additional context.
pub trait WrapError {
    /// Wraps the object into an [`Error`].
    #[track_caller]
    fn wrap_error(self) -> Error;

    /// Wraps the object into an [`Error`], with an additional message.
    #[track_caller]
    fn wrap_error_with_message<M>(self, message: M) -> Error
    where
        M: Display;
}

impl<E> WrapError for E
where
    E: Into<Error>,
{
    #[track_caller]
    fn wrap_error(self) -> Error {
        self.into()
    }

    #[track_caller]
    fn wrap_error_with_message<M>(self, message: M) -> Error
    where
        M: Display,
    {
        Into::<Error>::into(self).context(ContextError::new(message))
    }
}

/// Wraps an object into a result producing an [`Error`], optionally providing additional
/// context.
pub trait WrapResultError<T> {
    /// Wraps the object into a [`Result<T, Error>`].
    #[track_caller]
    fn wrap_error(self) -> Result<T, Error>;

    /// Wraps the object into a [`Result<T, Error>`], with an additional message.
    #[track_caller]
    fn wrap_error_with_message<M>(self, message: M) -> Result<T, Error>
    where
        M: Display;

    /// Wraps the object into a [`Result<T, Error>`], with an additional formatted message.
    #[track_caller]
    fn wrap_error_with_format<'a>(self, args: Arguments<'a>) -> Result<T, Error>;
}

impl<T, E> WrapResultError<T> for Result<T, E>
where
    E: WrapError,
{
    #[track_caller]
    fn wrap_error(self) -> Result<T, Error> {
        match self {
            Ok(val) => Ok(val),
            Err(error) => Err(error.wrap_error()),
        }
    }

    #[track_caller]
    fn wrap_error_with_message<M>(self, message: M) -> Result<T, Error>
    where
        M: Display,
    {
        match self {
            Ok(val) => Ok(val),
            Err(error) => Err(error.wrap_error_with_message(message)),
        }
    }

    #[track_caller]
    fn wrap_error_with_format<'a>(self, args: Arguments<'a>) -> Result<T, Error> {
        match self {
            Ok(val) => Ok(val),
            Err(error) => Err(error.wrap_error_with_message(format!("{args}"))),
        }
    }
}

/// Wraps an [`Option`] into a result producing an [`Error`].
pub trait WrapOptionError<T> {
    /// Wraps the object into a [`Result<T, Error>`].
    #[track_caller]
    fn wrap_expectation<M>(self, message: M) -> Result<T, Error>
    where
        M: Display;

    /// Wraps the object into a [`Result<T, Error>`], with a formatted message.
    #[track_caller]
    fn wrap_expectation_with_format<'a>(self, args: Arguments<'a>) -> Result<T, Error>;

    /// Wraps the object into a [`Result<T, Error>`], with a
    /// [`NotFoundError`][`crate::error::NotFoundError`] behind the scenes.
    #[track_caller]
    fn wrap_not_found_error<M>(self, message: M) -> Result<T, Error>
    where
        M: Display;

    /// Wraps the object into a [`Result<T, Error>`], with a
    /// [`NotFoundError`][`crate::error::NotFoundError`] behind the scenes, with a formatted
    /// message.
    #[track_caller]
    fn wrap_not_found_error_with_format<'a>(self, args: Arguments<'a>) -> Result<T, Error>;
}

impl<T> WrapOptionError<T> for Option<T> {
    fn wrap_expectation<M>(self, message: M) -> Result<T, Error>
    where
        M: Display,
    {
        match self {
            Some(val) => Ok(val),
            None => Err(general_error(message)),
        }
    }

    #[track_caller]
    fn wrap_expectation_with_format<'a>(self, args: Arguments<'a>) -> Result<T, Error> {
        match self {
            Some(val) => Ok(val),
            None => Err(general_error(format!("{args}"))),
        }
    }

    #[track_caller]
    fn wrap_not_found_error<M>(self, message: M) -> Result<T, Error>
    where
        M: Display,
    {
        match self {
            Some(val) => Ok(val),
            None => Err(not_found_error(message)),
        }
    }

    #[track_caller]
    fn wrap_not_found_error_with_format<'a>(self, args: Arguments<'a>) -> Result<T, Error> {
        match self {
            Some(val) => Ok(val),
            None => Err(not_found_error(format!("{args}"))),
        }
    }
}
