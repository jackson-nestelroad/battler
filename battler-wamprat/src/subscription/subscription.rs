use anyhow::Error;
use async_trait::async_trait;
use battler_wamp::peer::ReceivedEvent;

/// A subscription that receives events for a single topic.
#[async_trait]
pub trait Subscription: Send + Sync {
    /// Handles an event.
    async fn handle_event(&self, event: ReceivedEvent);
}

/// A strongly-typed subscription that receives events for a single topic.
///
/// Only events conforming to the [`Self::Event`] type parameter are processed. Invalid arguments
/// may be processed separately.
#[async_trait]
pub trait TypedSubscription: Send + Sync {
    /// Event from the publisher.
    type Event: battler_wamprat_message::WampApplicationMessage;

    /// Handles an event.
    async fn handle_event(&self, event: Self::Event);

    /// Handles an event that could not be deserialized to the [`Self::Event`] type parameter.
    #[allow(unused)]
    async fn handle_invalid_event(&self, event: ReceivedEvent, error: Error) {}
}

/// A strongly-typed, pattern-matched subscription that receives events for a single topic.
///
/// Only events conforming to the [`Self::Event`] type parameter are processed. Invalid arguments
/// may be processed separately.
#[async_trait]
pub trait TypedPatternMatchedSubscription: Send + Sync {
    /// Pattern of the procedure.
    type Pattern: battler_wamprat_uri::WampUriMatcher;

    /// Event from the publisher.
    type Event: battler_wamprat_message::WampApplicationMessage;

    /// Handles an event.
    async fn handle_event(&self, event: Self::Event, topic: Self::Pattern);

    /// Handles an event that could not be deserialized to the [`Self::Event`] type parameter.
    #[allow(unused)]
    async fn handle_invalid_event(&self, event: ReceivedEvent, error: Error) {}
}
