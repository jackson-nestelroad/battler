use std::{
    collections::hash_map::Entry,
    marker::PhantomData,
    sync::Arc,
};

use anyhow::{
    Error,
    Result,
};
use async_trait::async_trait;
use battler_wamp::{
    core::{
        error::WampError,
        id::Id,
        match_style::MatchStyle,
    },
    peer::ReceivedEvent,
};
use battler_wamp_uri::{
    Uri,
    WildcardUri,
};
use tokio::sync::broadcast;

use crate::{
    error::{
        WampratDeserializeError,
        WampratEventMissingTopic,
    },
    peer::error::AlreadySubscribedError,
    subscription::{
        Subscription,
        TypedPatternMatchedSubscription,
        TypedSubscription,
    },
};

/// A subscription that persists across multiple peer sessions.
pub(crate) struct PersistentSubscription {
    subscription: Arc<Box<dyn Subscription>>,
    match_style: Option<MatchStyle>,
    current_id: Option<Id>,
}

/// Module for managing persistent subscriptions on a [`Peer`][`crate::peer::Peer`].
///
/// Subscriptions can be created and removed during at any point in a peer's lifetime.
pub(crate) struct Subscriber<S> {
    _peer: PhantomData<S>,
    subscriptions: ahash::HashMap<WildcardUri, PersistentSubscription>,
}

impl<S> Subscriber<S>
where
    S: Send + 'static,
{
    /// Creates a new subscriber.
    pub fn new(_peer: Arc<battler_wamp::peer::Peer<S>>) -> Self {
        Self {
            _peer: PhantomData,
            subscriptions: ahash::HashMap::default(),
        }
    }

    /// Adds a new strongly-typed subscription entry without making network calls.
    pub fn add_subscription<T, Event>(&mut self, topic: Uri, subscription: T) -> Result<WildcardUri>
    where
        T: TypedSubscription<Event = Event> + 'static,
        Event: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
    {
        // Wrap the typed subscription with a generic wrapper that serializes and deserializes
        // application messages.
        struct SubscriptionWrapper<T, Event> {
            subscription: T,
            _event: PhantomData<Event>,
        }

        impl<T, Event> SubscriptionWrapper<T, Event>
        where
            T: TypedSubscription<Event = Event>,
            Event: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
        {
            fn new(subscription: T) -> Self {
                Self {
                    subscription,
                    _event: PhantomData,
                }
            }
        }

        #[async_trait]
        impl<T, Event> Subscription for SubscriptionWrapper<T, Event>
        where
            T: TypedSubscription<Event = Event>,
            Event: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
        {
            async fn handle_event(&self, event: ReceivedEvent) {
                match Event::wamp_deserialize_application_message(
                    event.arguments.clone(),
                    event.arguments_keyword.clone(),
                ) {
                    Ok(event) => self.subscription.handle_event(event).await,
                    Err(err) => {
                        self.subscription
                            .handle_invalid_event(event, err.into())
                            .await
                    }
                }
            }
        }

        let wildcard_topic: WildcardUri = topic.clone().into();
        match self.subscriptions.entry(wildcard_topic.clone()) {
            Entry::Occupied(_) => Err(AlreadySubscribedError::new(format!(
                "already actively subscribed to {topic}"
            ))
            .into()),
            Entry::Vacant(entry) => {
                entry.insert(PersistentSubscription {
                    subscription: Arc::new(Box::new(SubscriptionWrapper::new(subscription))),
                    match_style: None,
                    current_id: None,
                });
                Ok(wildcard_topic)
            }
        }
    }

    /// Adds a new strongly-typed, pattern-matched subscription entry with a specific URI without
    /// making network calls.
    pub fn add_subscription_pattern_matched_with_uri<T, Pattern, Event>(
        &mut self,
        topic: WildcardUri,
        match_style: Option<MatchStyle>,
        subscription: T,
    ) -> Result<WildcardUri>
    where
        T: TypedPatternMatchedSubscription<Pattern = Pattern, Event = Event> + 'static,
        Pattern: battler_wamprat_uri::WampUriMatcher + Send + Sync + 'static,
        Event: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
    {
        // Wrap the typed subscription with a generic wrapper that serializes and deserializes
        // application messages.
        struct SubscriptionWrapper<T, Pattern, Event> {
            subscription: T,
            _pattern: PhantomData<Pattern>,
            _event: PhantomData<Event>,
        }

        impl<T, Pattern, Event> SubscriptionWrapper<T, Pattern, Event>
        where
            T: TypedPatternMatchedSubscription<Event = Event>,
            Pattern: battler_wamprat_uri::WampUriMatcher + Send + Sync + 'static,
            Event: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
        {
            fn new(subscription: T) -> Self {
                Self {
                    subscription,
                    _pattern: PhantomData,
                    _event: PhantomData,
                }
            }
        }

        impl<T, Pattern, Event> SubscriptionWrapper<T, Pattern, Event>
        where
            T: TypedPatternMatchedSubscription<Pattern = Pattern, Event = Event>,
            Pattern: battler_wamprat_uri::WampUriMatcher + Send + Sync + 'static,
            Event: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
        {
            async fn handle_event_internal(&self, event: &ReceivedEvent) -> Result<(), WampError> {
                let topic = Pattern::wamp_match_uri(
                    event
                        .topic
                        .as_ref()
                        .ok_or_else(|| WampratEventMissingTopic.into())?
                        .as_ref(),
                )
                .map_err(Into::<WampError>::into)?;
                let event = Event::wamp_deserialize_application_message(
                    event.arguments.clone(),
                    event.arguments_keyword.clone(),
                )
                .map_err(Into::<WampratDeserializeError>::into)
                .map_err(Into::<WampError>::into)?;
                self.subscription.handle_event(event, topic).await;
                Ok(())
            }
        }

        #[async_trait]
        impl<T, Pattern, Event> Subscription for SubscriptionWrapper<T, Pattern, Event>
        where
            T: TypedPatternMatchedSubscription<Pattern = Pattern, Event = Event>,
            Pattern: battler_wamprat_uri::WampUriMatcher + Send + Sync + 'static,
            Event: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
        {
            async fn handle_event(&self, event: ReceivedEvent) {
                if let Err(err) = self.handle_event_internal(&event).await {
                    self.subscription
                        .handle_invalid_event(event, err.into())
                        .await;
                }
            }
        }

        match self.subscriptions.entry(topic.clone()) {
            Entry::Occupied(_) => Err(AlreadySubscribedError::new(format!(
                "already actively subscribed to {topic}"
            ))
            .into()),
            Entry::Vacant(entry) => {
                entry.insert(PersistentSubscription {
                    subscription: Arc::new(Box::new(SubscriptionWrapper::new(subscription))),
                    match_style,
                    current_id: None,
                });
                Ok(topic)
            }
        }
    }

    /// Activates a subscription by recording its active subscription ID and spawning its event
    /// loop.
    pub fn activate_subscription(
        &mut self,
        topic: &WildcardUri,
        id: Id,
        event_rx: broadcast::Receiver<ReceivedEvent>,
    ) -> Result<()> {
        let persistent_subscription = self
            .subscriptions
            .get_mut(topic)
            .ok_or_else(|| Error::msg(format!("subscription for {topic} missing")))?;
        persistent_subscription.current_id = Some(id);
        tokio::spawn(Self::event_loop(
            persistent_subscription.subscription.clone(),
            event_rx,
        ));
        Ok(())
    }

    /// Removes a subscription by topic.
    pub fn remove_subscription(&mut self, topic: &WildcardUri) -> Option<Id> {
        self.subscriptions
            .remove(topic)
            .and_then(|subscription| subscription.current_id)
    }

    /// Removes a subscription by topic.
    pub fn unsubscribe(&mut self, topic: &WildcardUri) -> Option<Id> {
        self.remove_subscription(topic)
    }

    /// Returns a list of all subscriptions and their match styles, suitable for persistence.
    pub fn subscriptions_for_restoration(&self) -> Vec<(WildcardUri, Option<MatchStyle>)> {
        self.subscriptions
            .iter()
            .map(|(topic, sub)| (topic.clone(), sub.match_style))
            .collect()
    }

    /// Event loop that processes received events and dispatches them to the subscription.
    pub(crate) async fn event_loop(
        subscription: Arc<Box<dyn Subscription>>,
        mut event_rx: broadcast::Receiver<ReceivedEvent>,
    ) {
        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    subscription.handle_event(event).await;
                }
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    log::warn!("Subscriber event loop lagged by {skipped} events");
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    }
}
