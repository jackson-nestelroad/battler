#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PeerRole {
    // Calls RPC endpoints.
    Caller,
    // Registers RPC endpoints.
    Callee,
    // Publishes events to topics.
    Publisher,
    // Subscribes to events for topics.
    Subscriber,
}

impl PeerRole {
    pub fn key_for_details(&self) -> &str {
        match self {
            Self::Caller => "caller",
            Self::Callee => "callee",
            Self::Publisher => "publisher",
            Self::Subscriber => "subscriber",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RouterRole {
    // Supports RPC calls.
    Dealer,
    // Supports pub/sub.
    Broker,
}

impl RouterRole {
    pub fn key_for_details(&self) -> &str {
        match self {
            Self::Dealer => "dealer",
            Self::Broker => "broker",
        }
    }
}
