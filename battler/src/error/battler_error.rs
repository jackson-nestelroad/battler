use std::fmt::Display;

use thiserror::Error;

use crate::error::{
    Error,
    WrapError,
};

/// A general error, consisting of only a message.
#[derive(Error, Debug)]
#[error("{message}")]
pub struct GeneralError {
    message: String,
}

impl GeneralError {
    /// Construcs a new general error.
    pub fn new<M>(message: M) -> Self
    where
        M: Display,
    {
        Self {
            message: message.to_string(),
        }
    }
}

/// A not found error.
#[derive(Error, Debug)]
#[error("{target} not found")]
pub struct NotFoundError {
    target: String,
}

impl NotFoundError {
    /// Construcs a new not found error.
    pub fn new<M>(target: M) -> Self
    where
        M: Display,
    {
        Self {
            target: target.to_string(),
        }
    }
}

/// A team validation error, consisting of all problems.
#[derive(Error, Debug)]
#[error("team validation failed: {}", .problems.join("; "))]
pub struct TeamValidationError {
    problems: Vec<String>,
}

impl TeamValidationError {
    /// Constructs a new team validation error.
    pub fn new(problems: Vec<String>) -> Self {
        Self { problems }
    }

    /// THe problems generated from team validation.
    pub fn problems(&self) -> impl Iterator<Item = &str> {
        self.problems.iter().map(|s| s.as_str())
    }
}

/// A borrow failure.
#[derive(Error, Debug)]
#[error("failed to borrow {target}")]
pub struct BorrowFailedError {
    #[source]
    error: anyhow::Error,
    target: String,
}

impl BorrowFailedError {
    /// Construcs a new borrow failure.
    pub fn new<E, M>(error: E, target: M) -> Self
    where
        E: Into<anyhow::Error>,
        M: Display,
    {
        Self {
            error: error.into(),
            target: target.to_string(),
        }
    }
}

/// An integer overflow error.
#[derive(Error, Debug)]
#[error("integer overflow")]
pub struct IntegerOverflowError {
    #[source]
    error: anyhow::Error,
}

impl IntegerOverflowError {
    pub fn wrap(error: anyhow::Error) -> Self {
        Self { error }
    }
}

/// Helper for an [`struct@Error`] wrapping a [`GeneralError`].
#[track_caller]
pub fn general_error<M>(message: M) -> Error
where
    M: Display,
{
    GeneralError::new(message).wrap_error()
}

/// Helper for an [`struct@Error`] wrapping a [`NotFoundError`].
#[track_caller]
pub fn not_found_error<M>(target: M) -> Error
where
    M: Display,
{
    NotFoundError::new(target).wrap_error()
}

/// Helper for an [`struct@Error`] wrapping a [`BorrowFailedError`].
#[track_caller]
pub fn borrow_failed_error<E, M>(error: E, target: M) -> Error
where
    E: Into<anyhow::Error>,
    M: Display,
{
    BorrowFailedError::new(error, target).wrap_error()
}

/// Helper for an [`struct@Error`] wrapping an [`IntegerOverflowError`].
#[track_caller]
pub fn integer_overflow_error<E>(error: E) -> Error
where
    E: Into<anyhow::Error>,
{
    IntegerOverflowError::wrap(error.into()).wrap_error()
}
