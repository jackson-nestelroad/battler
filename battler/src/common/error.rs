use std::{
    error,
    fmt::{
        format,
        Arguments,
        Display,
        Formatter,
        Result as DisplayResult,
    },
};

/// A common error type for the entire library.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Error {
    message: String,
}

impl Error {
    pub fn new<S: Into<String>>(message: S) -> Self {
        Self {
            message: message.into(),
        }
    }

    /// Prefixes the given error with a message.
    pub fn prefix(self, message: &str) -> Self {
        Self::new(format!("{message}: {}", self.message))
    }
}

/// A macro that creates a new error using the same arguments as the [`format!`] macro.
#[macro_export]
macro_rules! battler_error {
    ($($arg:tt)*) => {{
        Error::new(format!($($arg)*))
    }}
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> DisplayResult {
        write!(f, "{}", self.message)
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        &self.message
    }
}

/// A trait that wraps some object into an [`Error`].
pub trait WrapResultError<T> {
    /// Wraps the object directly into a [`Result<T, Error>`].
    fn wrap_error(self) -> Result<T, Error>;
    /// Wraps the object into a [`Result<T, Error>`] with the given message.
    fn wrap_error_with_message(self, message: &str) -> Result<T, Error>;
    /// Wraps the object intoa [`Result<T, Error>`] with the given formatted arguments.
    fn wrap_error_with_format<'a>(self, args: Arguments<'a>) -> Result<T, Error>;
}

impl<T, E: ToString> WrapResultError<T> for Result<T, E> {
    fn wrap_error(self) -> Result<T, Error> {
        match self {
            Err(err) => Err(Error::new(err.to_string())),
            Ok(res) => Ok(res),
        }
    }

    fn wrap_error_with_message(self, message: &str) -> Result<T, Error> {
        match self {
            Err(err) => Err(Error::new(format!("{}: {}", message, err.to_string()))),
            Ok(res) => Ok(res),
        }
    }

    fn wrap_error_with_format<'a>(self, args: Arguments<'a>) -> Result<T, Error> {
        match self {
            Err(err) => Err(Error::new(format!("{}: {}", args, err.to_string()))),
            Ok(res) => Ok(res),
        }
    }
}

impl<T> WrapResultError<T> for Option<T> {
    fn wrap_error(self) -> Result<T, Error> {
        match self {
            None => Err(Error::new("option contained no value")),
            Some(val) => Ok(val),
        }
    }

    fn wrap_error_with_message(self, message: &str) -> Result<T, Error> {
        match self {
            None => Err(Error::new(message)),
            Some(val) => Ok(val),
        }
    }

    fn wrap_error_with_format<'a>(self, args: Arguments<'a>) -> Result<T, Error> {
        match self {
            None => Err(Error::new(format(args))),
            Some(val) => Ok(val),
        }
    }
}
