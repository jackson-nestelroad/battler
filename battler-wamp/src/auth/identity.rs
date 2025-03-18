/// The identity of an authenticated peer.
#[derive(Debug, Default, Clone)]
pub struct Identity {
    /// The authentication ID the client was actually authenticated as.
    pub id: String,
    /// The authentication role the client was authenticated for.
    pub role: String,
}
