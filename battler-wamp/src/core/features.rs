use battler_wamp_values::WampDictionary;

/// Advanced features for WAMP routers and peers, related to pub/sub.
#[derive(Debug, Default, Clone, WampDictionary)]
pub struct PubSubFeatures {}

/// Advanced features for WAMP routers and peers, related to RPCs.
#[derive(Debug, Default, Clone, WampDictionary)]
pub struct RpcFeatures {
    /// A caller may actively cancel a procedure call.
    #[battler_wamp_values(default)]
    pub call_canceling: bool,
    /// Procedures may produce progressive results.
    #[battler_wamp_values(default)]
    pub progressive_call_results: bool,
    /// The peer can enforce call timeouts.
    #[battler_wamp_values(default)]
    pub call_timeout: bool,
    /// A procedure can be shared by multiple callees.
    #[battler_wamp_values(default)]
    pub shared_registration: bool,
    /// Callers can be identified.
    #[battler_wamp_values(default)]
    pub caller_identification: bool,
}
