/// A role a peer can take on.
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

impl TryFrom<&str> for PeerRole {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "caller" => Ok(Self::Caller),
            "callee" => Ok(Self::Callee),
            "publisher" => Ok(Self::Publisher),
            "subscriber" => Ok(Self::Subscriber),
            _ => Err(Self::Error::msg(format!("invalid peer role: {value}"))),
        }
    }
}

impl Into<&'static str> for PeerRole {
    fn into(self) -> &'static str {
        match self {
            Self::Caller => "caller",
            Self::Callee => "callee",
            Self::Publisher => "publisher",
            Self::Subscriber => "subscriber",
        }
    }
}

impl Into<String> for PeerRole {
    fn into(self) -> String {
        Into::<&'static str>::into(self).to_owned()
    }
}

impl ToString for PeerRole {
    fn to_string(&self) -> String {
        (*self).into()
    }
}

/// A role a router can take on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum RouterRole {
    // Supports RPC calls.
    Dealer,
    // Supports pub/sub.
    Broker,
}

impl TryFrom<&str> for RouterRole {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "dealer" => Ok(Self::Dealer),
            "broker" => Ok(Self::Broker),
            _ => Err(Self::Error::msg(format!("invalid router role: {value}"))),
        }
    }
}

impl Into<&'static str> for RouterRole {
    fn into(self) -> &'static str {
        match self {
            Self::Dealer => "dealer",
            Self::Broker => "broker",
        }
    }
}

impl Into<String> for RouterRole {
    fn into(self) -> String {
        Into::<&'static str>::into(self).to_owned()
    }
}

impl ToString for RouterRole {
    fn to_string(&self) -> String {
        (*self).into()
    }
}
