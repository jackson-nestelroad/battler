use battler_wamp_values::WampDictionary;

use crate::core::features::{
    PubSubFeatures,
    RpcFeatures,
};

/// A role a peer can take on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
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

/// Features for RPCs.
#[derive(Debug, Default, Clone, WampDictionary)]
pub struct RpcRoleDetails {
    #[battler_wamp_values(default)]
    pub features: RpcFeatures,
}

/// Features for pub/sub.
#[derive(Debug, Default, Clone, WampDictionary)]
pub struct PubSubRoleDetails {
    #[battler_wamp_values(default)]
    pub features: PubSubFeatures,
}

/// Roles and features taken on by a peer.
#[derive(Debug, Default, Clone, WampDictionary)]
pub struct PeerRoles {
    #[battler_wamp_values(default, skip_serializing_if = Option::is_none)]
    pub caller: Option<RpcRoleDetails>,
    #[battler_wamp_values(default, skip_serializing_if = Option::is_none)]
    pub callee: Option<RpcRoleDetails>,
    #[battler_wamp_values(default, skip_serializing_if = Option::is_none)]
    pub publisher: Option<PubSubRoleDetails>,
    #[battler_wamp_values(default, skip_serializing_if = Option::is_none)]
    pub subscriber: Option<PubSubRoleDetails>,
}

impl PeerRoles {
    pub(crate) fn new(
        roles: impl Iterator<Item = PeerRole>,
        pub_sub_features: PubSubFeatures,
        rpc_features: RpcFeatures,
    ) -> Self {
        let mut result = Self::default();
        for role in roles {
            match role {
                PeerRole::Caller => {
                    result.caller = Some(RpcRoleDetails {
                        features: rpc_features.clone(),
                    })
                }
                PeerRole::Callee => {
                    result.callee = Some(RpcRoleDetails {
                        features: rpc_features.clone(),
                    })
                }
                PeerRole::Publisher => {
                    result.publisher = Some(PubSubRoleDetails {
                        features: pub_sub_features.clone(),
                    })
                }
                PeerRole::Subscriber => {
                    result.subscriber = Some(PubSubRoleDetails {
                        features: pub_sub_features.clone(),
                    })
                }
            }
        }
        result
    }
}

/// Roles and features taken on by a router.
#[derive(Debug, Default, Clone, WampDictionary)]
pub struct RouterRoles {
    #[battler_wamp_values(default, skip_serializing_if = Option::is_none)]
    pub dealer: Option<RpcRoleDetails>,
    #[battler_wamp_values(default, skip_serializing_if = Option::is_none)]
    pub broker: Option<PubSubRoleDetails>,
}

impl RouterRoles {
    pub(crate) fn new(
        roles: impl Iterator<Item = RouterRole>,
        pub_sub_features: PubSubFeatures,
        rpc_features: RpcFeatures,
    ) -> Self {
        let mut result = Self::default();
        for role in roles {
            match role {
                RouterRole::Dealer => {
                    result.dealer = Some(RpcRoleDetails {
                        features: rpc_features.clone(),
                    })
                }
                RouterRole::Broker => {
                    result.broker = Some(PubSubRoleDetails {
                        features: pub_sub_features.clone(),
                    })
                }
            }
        }
        result
    }
}
