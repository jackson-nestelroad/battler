use battler_wamp_values::WampDictionary;

/// Advanced features for WAMP routers and peers.
#[derive(Debug, Default, Clone, WampDictionary)]
pub struct Features {
    /// A caller may actively cancel a procedure call.
    pub call_canceling: bool,
    /// Procedures may produce progressive results.
    pub progressive_call_results: bool,
}
