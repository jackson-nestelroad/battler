use battler_wamp_values::{
    Dictionary,
    List,
    WampDeserializeError,
    WampSerializeError,
};
pub use battler_wamprat_message_proc_macro::WampApplicationMessage;

/// Trait for a WAMP application message, which can be passed between applications using pub/sub or
/// RPCs.
pub trait WampApplicationMessage: Sized {
    /// Serializes the object into arguments and keyword arguments.
    fn wamp_serialize_application_message(self) -> Result<(List, Dictionary), WampSerializeError>;

    /// Deserializes the object from arguments and keyword arguments.
    fn wamp_deserialize_application_message(
        arguments: List,
        arguments_keyword: Dictionary,
    ) -> Result<Self, WampDeserializeError>;
}
