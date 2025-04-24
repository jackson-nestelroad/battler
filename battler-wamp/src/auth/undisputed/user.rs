use crate::auth::Identity;

/// Per-user data required for undisputed authentication.
#[derive(Clone)]
pub struct UserData {
    /// User identity.
    pub identity: Identity,
}
