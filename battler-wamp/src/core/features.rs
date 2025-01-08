use battler_wamp_values::WampDictionary;

/// Advanced features for WAMP routers and peers, related to pub/sub.
#[derive(Debug, Default, Clone, WampDictionary)]
pub struct PubSubFeatures {}

/// Advanced features for WAMP routers and peers, related to RPCs.
#[derive(Debug, Default, Clone, WampDictionary)]
pub struct RpcFeatures {
    /// A caller may actively cancel a procedure call.
    pub call_canceling: bool,
    /// Procedures may produce progressive results.
    pub progressive_call_results: bool,
    /// The peer can enforce call timeouts.
    pub call_timeout: bool,
}
