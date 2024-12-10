use std::fmt::Debug;

use anyhow::{
    Error,
    Result,
};
use futures_util::{
    Sink,
    Stream,
};

use crate::serializer::serializer::SerializerType;

/// Data received from a [`Transport`].
pub enum TransportData {
    /// Data that should be perceived as a health check and immediately sent back to the sender.
    Ping(Vec<u8>),
    /// Data representing a meaningful WAMP message.
    Message(Vec<u8>),
}

/// A transport, over which WAMP messages can be sent and received.
///
/// Implemented as a [`Stream`] and [`Sink`] that extracts out meaningful data and reports protocol
/// violations to be handled at higher layers.
pub trait Transport:
    Send + Stream<Item = Result<TransportData>> + Sink<TransportData, Error = Error> + Unpin + Debug
{
}

/// A factory for creating a new [`Transport`].
pub trait TransportFactory<S>: Send + Sync {
    /// Creates a new [`Transport`] for WAMP messaging.
    fn new_transport(&self, stream: S, serializer_type: SerializerType) -> Box<dyn Transport>;
}
