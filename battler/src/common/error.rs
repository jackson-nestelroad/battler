use std::{
    any::Any,
    error::Error as StdError,
    fmt::{
        format,
        Arguments,
        Debug,
        Display,
        Formatter,
        Result as DisplayResult,
    },
};

/// Helper trait for cloning wrappable errors.
pub trait WrappableErrorClone {
    fn clone_wrappable_error(&self) -> Box<dyn WrappableError>;
}

impl<E> WrappableErrorClone for E
where
    E: 'static + WrappableError + Clone,
{
    fn clone_wrappable_error(&self) -> Box<dyn WrappableError> {
        Box::new(self.clone())
    }
}

/// Helper trait for converting a [`WrappableError`] to [`Any`].
pub trait WrappableErrorAsAny {
    /// Returns an [`Any`] reference to the object.
    fn as_any(&self) -> &dyn Any;
}

impl<E> WrappableErrorAsAny for E
where
    E: 'static,
{
    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// Helper trait for equality checks for wrappable errors.
pub trait WrappableErrorPartialEq: WrappableErrorAsAny {
    /// Checks equality with another wrappable error.
    fn eq_wrappable_error(&self, _: &dyn WrappableError) -> bool;
}

impl<E> WrappableErrorPartialEq for E
where
    E: 'static + PartialEq,
{
    fn eq_wrappable_error(&self, other: &dyn WrappableError) -> bool {
        other
            .as_any()
            .downcast_ref::<E>()
            .map_or(false, |other| self.eq(other))
    }
}

/// A error that can be wrapped in the [`Error`] type.
pub trait WrappableError: StdError + WrappableErrorClone + WrappableErrorPartialEq {
    /// Converts the [`WrappableError`] to the concrete [`Error`] type, if possible.
    fn as_error(&self) -> Option<&Error> {
        self.as_any().downcast_ref::<Error>()
    }
}

impl Clone for Box<dyn WrappableError> {
    fn clone(&self) -> Self {
        self.clone_wrappable_error()
    }
}

impl PartialEq for Box<dyn WrappableError> {
    fn eq(&self, other: &Self) -> bool {
        self.as_ref().eq_wrappable_error(other.as_ref())
    }
}

/// A simple error type that wraps an error message.
#[derive(Debug, Clone, PartialEq)]
struct SimpleError {
    message: String,
}

impl SimpleError {
    fn new<S: Into<String>>(message: S) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl Display for SimpleError {
    fn fmt(&self, f: &mut Formatter<'_>) -> DisplayResult {
        write!(f, "{}", self.message)
    }
}

impl StdError for SimpleError {}
impl WrappableError for SimpleError {}

/// An error node in the stack formed by the [`Error`] type.
#[derive(Debug, Clone)]
enum ErrorNode {
    Leaf(Box<dyn WrappableError>),
    Wrapped {
        message: String,
        inner: Box<dyn WrappableError>,
    },
}

impl PartialEq for ErrorNode {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Leaf(a), Self::Leaf(b)) => a.eq(b),
            (
                Self::Wrapped {
                    message: a_message,
                    inner: a_inner,
                },
                Self::Wrapped {
                    message: b_message,
                    inner: b_inner,
                },
            ) => a_message.eq(b_message) && a_inner.eq(b_inner),
            _ => false,
        }
    }
}

/// A common error type for the entire library.
#[derive(Debug, Clone, PartialEq)]
pub struct Error(ErrorNode);

impl Error {
    /// Constructs a new error with the given message.
    pub fn new<S: Into<String>>(message: S) -> Self {
        Self(ErrorNode::Leaf(Box::new(SimpleError::new(message))))
    }

    /// Constructs a new error with the given formatted string.
    pub fn format(args: Arguments) -> Self {
        Self::new(format(args))
    }

    /// Wraps the given error with a prefixed message.
    pub fn wrap<S: Into<String>, E: WrappableError + 'static>(message: S, error: E) -> Self {
        Self(ErrorNode::Wrapped {
            message: message.into(),
            inner: Box::new(error),
        })
    }

    /// Generates the message for this error, including all wrapped errors.
    pub fn message(&self) -> String {
        match &self.0 {
            ErrorNode::Leaf(error) => error.to_string(),
            ErrorNode::Wrapped { message, inner } => format!("{message}: {inner}"),
        }
    }

    /// Returns a reference to the inner error.
    pub fn inner(&self) -> Option<&dyn WrappableError> {
        match &self.0 {
            ErrorNode::Leaf(_) => None,
            ErrorNode::Wrapped { message: _, inner } => Some(inner.as_ref()),
        }
    }
}

/// A macro that creates a new error using the same arguments as the [`format!`] macro.
#[macro_export]
macro_rules! battler_error {
    ($($arg:tt)*) => {{
        Error::format(format_args!($($arg)*))
    }}
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> DisplayResult {
        write!(f, "{}", self.message())
    }
}

impl StdError for Error {}
impl WrappableError for Error {}

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

#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
mod error_tests {
    use crate::common::Error;

    #[test]
    fn error_message_includes_all_wrapped_errors() {
        let error = Error::wrap("abc", Error::wrap("def", Error::new("ghi")));
        assert_eq!(error.message(), "abc: def: ghi");
        let error = Error::wrap("uvw", Error::new("xyz"));
        assert_eq!(error.message(), "uvw: xyz");
    }

    #[test]
    fn returns_inner_error() {
        let error = Error::wrap("abc", Error::wrap("def", Error::new("ghi")));
        assert_eq!(error.message(), "abc: def: ghi");

        let inner = error.inner();
        assert!(inner.is_some());
        let inner = inner.unwrap().as_error();
        assert!(inner.is_some());
        let inner = inner.unwrap();
        assert_eq!(inner, &Error::wrap("def", Error::new("ghi")));
        assert_eq!(inner.message(), "def: ghi");

        let inner = inner.inner();
        assert!(inner.is_some());
        let inner = inner.unwrap().as_error();
        assert!(inner.is_some());
        let inner = inner.unwrap();
        assert_eq!(inner, &Error::new("ghi"));
        assert_eq!(inner.message(), "ghi");

        let inner = inner.inner();
        assert!(inner.is_none());
    }

    #[test]
    fn error_equality() {
        let left = Error::wrap("abc", Error::wrap("def", Error::new("ghi")));
        let right = left.clone();
        assert_eq!(left, right);
        let right = Error::new("abc: def: ghi");
        assert_ne!(left, right);
        let right = Error::wrap("abc", Error::wrap("xyz", Error::new("ghi")));
        assert_ne!(left, right);
    }
}
