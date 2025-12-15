use crate::auth::Identity;

/// The type of connection a peer is communicating over.
#[derive(Debug, Clone)]
pub enum ConnectionType {
    /// Connection to a remote address.
    Remote(String),
    /// Direct connection.
    Direct,
}

impl Default for ConnectionType {
    fn default() -> Self {
        Self::Remote(String::default())
    }
}

/// Information about a peer.
#[derive(Debug, Default, Clone)]
pub struct PeerInfo {
    /// Type of connection the peer is communicating over.
    pub connection_type: ConnectionType,
    /// Identity, established when joining a realm.
    pub identity: Identity,
}
