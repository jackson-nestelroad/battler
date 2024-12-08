use crate::core::uri::Uri;

/// The reason for closing a WAMP session.
#[derive(Debug, Default, Clone, Copy)]
pub enum CloseReason {
    #[default]
    Normal,
    SystemShutdown,
    CloseRealm,
    Killed,
    TimedOut,
    GoodbyeAndOut,
}

impl CloseReason {
    fn uri_component(&self) -> &str {
        match self {
            Self::Normal => "normal",
            Self::SystemShutdown => "system_shutdown",
            Self::CloseRealm => "close_realm",
            Self::Killed => "killed",
            Self::TimedOut => "timed_out",
            Self::GoodbyeAndOut => "goodbye_and_out",
        }
    }

    /// URI for the close reason.
    pub fn uri(&self) -> Uri {
        Uri::from_known(format!("wamp.close.{}", self.uri_component()))
    }
}
