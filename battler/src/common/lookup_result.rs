use std::{
    convert::Infallible,
    fmt::Debug,
    ops::{
        ControlFlow,
        FromResidual,
        Try,
    },
};

use crate::{
    battler_error,
    common::Error,
};

/// The result of a lookup operation.
///
/// Some implementations may fail due to some internal error and may want to distinguish internal
/// errors from errors when data is not found. Thus, both [`Option`] and [`Result`] are inadequate.
/// This type is a sort of hybrid between the two.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum LookupResult<T, E> {
    /// Value not found.
    NotFound,
    /// Value found for the lookup operation.
    Found(T),
    /// An internal error that occurred during the lookup operation.
    Error(E),
}

fn unwrap_failed(msg: &str) -> ! {
    panic!("{msg}")
}

fn unwrap_failed_with_value(msg: &str, error: &dyn Debug) -> ! {
    panic!("{msg}: {error:?}")
}

impl<T, E> LookupResult<T, E> {
    /// Returns the contained [`LookupResult::Found`] value, consuming the `self` value.
    ///
    /// Panics if the enum is some other variant.
    pub fn unwrap(self) -> T
    where
        E: Debug,
    {
        match self {
            Self::NotFound => {
                unwrap_failed("called `LookupResult::unwrap()` on a `NotFound` value")
            }
            Self::Found(value) => value,
            Self::Error(error) => unwrap_failed_with_value(
                "called `LookupResult::unwrap() on an `Error` value",
                &error,
            ),
        }
    }

    /// Converts the [`LookupResult`] into an [`Option`].
    ///
    /// [`LookupResult::Error`] is converted into [`None`].
    pub fn into_option(self) -> Option<T> {
        match self {
            Self::NotFound => None,
            Self::Found(value) => Some(value),
            Self::Error(_) => None,
        }
    }
}

impl<T> LookupResult<T, Error> {
    /// Returns the contained [`LookupResult::Error`] value, consuming the `self` value.
    ///
    /// Panics if the enum is some other variant.
    pub fn unwrap_err(self) -> Error
    where
        T: Debug,
    {
        match self.into_result() {
            Ok(value) => unwrap_failed_with_value(
                "called `LookupResult::unwrap() on an `Error` value",
                &value,
            ),
            Err(error) => error,
        }
    }

    /// Converts the [`LookupResult`] into a [`Result`].
    ///
    /// [`LookupResult::NotFound`] is converted into an [`Error`].
    pub fn into_result(self) -> Result<T, Error> {
        match self {
            Self::NotFound => Err(battler_error!("not found")),
            Self::Found(value) => Ok(value),
            Self::Error(error) => Err(error),
        }
    }
}

impl<T, E> LookupResult<&T, E>
where
    T: Clone,
{
    pub fn cloned(self) -> LookupResult<T, E> {
        match self {
            Self::NotFound => LookupResult::NotFound,
            Self::Found(value) => LookupResult::Found(value.clone()),
            Self::Error(error) => LookupResult::Error(error),
        }
    }
}

impl<T, E> Try for LookupResult<T, E> {
    type Output = T;
    type Residual = LookupResult<Infallible, E>;

    fn from_output(output: Self::Output) -> Self {
        Self::Found(output)
    }

    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        match self {
            Self::NotFound => ControlFlow::Break(LookupResult::NotFound),
            Self::Found(value) => ControlFlow::Continue(value),
            Self::Error(error) => ControlFlow::Break(LookupResult::Error(error)),
        }
    }
}

impl<T, E> FromResidual<LookupResult<Infallible, E>> for LookupResult<T, E> {
    fn from_residual(residual: <Self as Try>::Residual) -> Self {
        match residual {
            LookupResult::NotFound => Self::NotFound,
            LookupResult::Found(_) => unreachable!(),
            LookupResult::Error(error) => Self::Error(error),
        }
    }
}

impl<T, E> From<Result<T, E>> for LookupResult<T, E> {
    fn from(value: Result<T, E>) -> Self {
        match value {
            Ok(value) => Self::Found(value),
            Err(error) => Self::Error(error),
        }
    }
}

impl<T, E> From<Option<T>> for LookupResult<T, E> {
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => Self::Found(value),
            None => Self::NotFound,
        }
    }
}

impl<T, E> From<LookupResult<T, E>> for Option<T> {
    fn from(value: LookupResult<T, E>) -> Self {
        value.into_option()
    }
}
