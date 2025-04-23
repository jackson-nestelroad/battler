use std::{
    convert::Infallible,
    fmt::Display,
    ops::FromResidual,
};

use thiserror::Error;

/// An error resulting from some validation process.
#[derive(Debug, Default, Error)]
pub struct ValidationError {
    problems: Vec<String>,
}

impl ValidationError {
    /// All problems.
    pub fn problems(&self) -> impl Iterator<Item = &str> {
        self.problems.iter().map(|s| s.as_str())
    }

    /// Checks if the problem list is empty.
    pub fn is_empty(&self) -> bool {
        self.problems.is_empty()
    }
}

impl Display for ValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "validation failed: {}", self.problems.join("; "))
    }
}

impl<S> FromIterator<S> for ValidationError
where
    S: Into<String>,
{
    fn from_iter<T: IntoIterator<Item = S>>(iter: T) -> Self {
        Self {
            problems: iter.into_iter().map(|s| s.into()).collect(),
        }
    }
}

impl<E> FromResidual<Result<Infallible, E>> for ValidationError
where
    E: Display,
{
    fn from_residual(residual: Result<Infallible, E>) -> Self {
        match residual {
            Err(error) => Self::from_iter([error.to_string()]),
            #[allow(unreachable_patterns)]
            Ok(_) => unreachable!(),
        }
    }
}
