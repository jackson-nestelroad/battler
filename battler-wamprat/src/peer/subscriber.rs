use std::{
    collections::hash_map::Entry,
    marker::PhantomData,
    sync::Arc,
};

use anyhow::Result;
use async_trait::async_trait;
use battler_wamp::{
    core::{
        error::WampError,
        id::Id,
        match_style::MatchStyle,
    },
    peer::{
        ReceivedEvent,
        SubscriptionOptions,
    },
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
    peer: Arc<battler_wamp::peer::Peer<S>>,
    subscriptions: ahash::HashMap<WildcardUri, PersistentSubscription>,
}

impl<S> Subscriber<S>
where
    S: Send + 'static,
{
    /// Creates a new subscriber.
    pub fn new(peer: Arc<battler_wamp::peer::Peer<S>>) -> Self {
        Self {
            peer,
            subscriptions: ahash::HashMap::default(),
        }
    }

    /// Adds a new strongly-typed subscription, which will be created on every new connection to a
    /// router.
    pub async fn subscribe<T, Event>(&mut self, topic: Uri, subscription: T) -> Result<()>
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

        match self.subscriptions.entry(topic.clone().into()) {
            Entry::Occupied(_) => Err(AlreadySubscribedError::new(format!(
                "already actively subscribed to {topic}"
            ))
            .into()),
            Entry::Vacant(entry) => {
                let subscription = entry.insert(PersistentSubscription {
                    subscription: Arc::new(Box::new(SubscriptionWrapper::new(subscription))),
                    match_style: None,
                    current_id: None,
                });
                Self::restore_subscription(&self.peer, &topic.into(), subscription).await
            }
        }
    }

    /// Adds a new strongly-typed, pattern-matched subscription, which will be created on every new
    /// connection to a router.
    pub async fn subscribe_pattern_matched<T, Pattern, Event>(
        &mut self,
        subscription: T,
    ) -> Result<()>
    where
        T: TypedPatternMatchedSubscription<Pattern = Pattern, Event = Event> + 'static,
        Pattern: battler_wamprat_uri::WampUriMatcher + Send + Sync + 'static,
        Event: battler_wamprat_message::WampApplicationMessage + Send + Sync + 'static,
    {
        self.subscribe_pattern_matched_with_uri(
            Pattern::uri_for_router(),
            Pattern::match_style(),
            subscription,
        )
        .await
    }

    /// Adds a new strongly-typed, pattern-matched subscription with a specific URI, which will be
    /// created on every new connection to a router.
    ///
    /// Use carefully; the URI passed in must properly overlap with the `Pattern` type parameter for
    /// the subscription handler to work as expected.
    pub async fn subscribe_pattern_matched_with_uri<T, Pattern, Event>(
        &mut self,
        topic: WildcardUri,
        match_style: Option<MatchStyle>,
        subscription: T,
    ) -> Result<()>
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
                let subscription = entry.insert(PersistentSubscription {
                    subscription: Arc::new(Box::new(SubscriptionWrapper::new(subscription))),
                    match_style,
                    current_id: None,
                });
                Self::restore_subscription(&self.peer, &topic, subscription).await
            }
        }
    }

    /// Removes a subscription by topic.
    pub async fn unsubscribe(&mut self, topic: &WildcardUri) -> Result<()> {
        let id = match self
            .subscriptions
            .remove(&topic)
            .map(|subscription| subscription.current_id)
            .flatten()
        {
            Some(id) => id,
            None => return Ok(()),
        };
        self.peer.unsubscribe(id).await
    }

    async fn event_loop(
        subscription: Arc<Box<dyn Subscription>>,
        mut event_rx: broadcast::Receiver<ReceivedEvent>,
    ) {
        while let Ok(event) = event_rx.recv().await {
            subscription.handle_event(event).await;
        }
    }

    async fn restore_subscription(
        peer: &battler_wamp::peer::Peer<S>,
        topic: &WildcardUri,
        persistent_subscription: &mut PersistentSubscription,
    ) -> Result<()> {
        let subscription = peer
            .subscribe_with_options(
                topic.clone(),
                SubscriptionOptions {
                    match_style: persistent_subscription.match_style,
                },
            )
            .await?;
        persistent_subscription.current_id = Some(subscription.id);
        tokio::spawn(Self::event_loop(
            persistent_subscription.subscription.clone(),
            subscription.event_rx,
        ));
        Ok(())
    }

    /// Restores all subscriptions.
    pub async fn restore_subscriptions(&mut self) -> Result<()> {
        for (topic, persistent_subscription) in &mut self.subscriptions {
            Self::restore_subscription(&self.peer, topic, persistent_subscription)
                .await
                .map_err(|err| err.context(format!("failed to resubscribe to {topic}")))?;
        }

        Ok(())
    }
}
