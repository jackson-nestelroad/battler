use thiserror::Error;

/// An error for a procedure failing to register.
#[derive(Debug, Error)]
#[error("{msg}")]
pub struct ProcedureRegistrationError {
    msg: String,
}

impl ProcedureRegistrationError {
    pub fn new<S>(msg: S) -> Self
    where
        S: Into<String>,
    {
        Self { msg: msg.into() }
    }
}

/// An error for a peer failing to connect to a router.
#[derive(Debug, Error)]
#[error("{msg}")]
pub struct PeerConnectionError {
    msg: String,
}

impl PeerConnectionError {
    pub fn new<S>(msg: S) -> Self
    where
        S: Into<String>,
    {
        Self { msg: msg.into() }
    }
}

/// An error for a topic already having a subscription.
#[derive(Debug, Error)]
#[error("{msg}")]
pub struct AlreadySubscribedError {
    msg: String,
}

impl AlreadySubscribedError {
    pub fn new<S>(msg: S) -> Self
    where
        S: Into<String>,
    {
        Self { msg: msg.into() }
    }
}
