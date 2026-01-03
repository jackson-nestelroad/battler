use alloc::string::{
    String,
    ToString,
};
use core::{
    fmt::{
        Debug,
        Display,
    },
    panic::Location,
};

/// An error that provides context to another error.
///
/// Provides a new message and the source location of where the wrapper error was generated.
pub struct ContextError {
    message: String,
    location: &'static Location<'static>,
}

impl ContextError {
    /// Constructs a new context error.
    #[track_caller]
    pub fn new<M>(message: M) -> Self
    where
        M: Display,
    {
        Self {
            message: message.to_string(),
            location: Location::caller(),
        }
    }
}

impl Debug for ContextError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            f,
            "{} (at {}:{})",
            self.message,
            self.location.file(),
            self.location.line()
        )
    }
}

impl Display for ContextError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.message)
    }
}
