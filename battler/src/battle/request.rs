/// Type type of [`Request`] that should be requested from a player.
#[derive(Debug, Clone, Copy)]
pub enum RequestType {
    /// A request for a team order to be chosen during team preview.
    TeamPreview,
    /// A request for the active Mon(s) to act at the beginning of a turn.
    Turn,
    /// A request for one or more Mons to be switched in.
    Switch,
}

/// A request for a Mon to be switched in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwitchRequest {
    /// Team slots that are eligible for switch in.
    pub options: Vec<usize>,
}

/// A request for an action that a [`Player`][`crate::battle::Player`] must make before the battle
/// can continue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Request {
    /// A request for a team order to be chosen during team preview.
    TeamPreview,
    /// A request for the active Mon(s) to act at the beginning of a turn.
    Turn,
    /// A request for one or more Mons to be switched in.
    Switch(SwitchRequest),
}

impl Request {
    /// The type of the request.
    pub fn request_type(&self) -> RequestType {
        match self {
            Self::TeamPreview => RequestType::TeamPreview,
            Self::Turn => RequestType::Turn,
            Self::Switch(_) => RequestType::Switch,
        }
    }
}
