use std::{
    collections::hash_map::Entry,
    marker::PhantomData,
    sync::Arc,
};

use anyhow::Result;
use async_trait::async_trait;
use battler_wamp::{
    core::{
        id::Id,
        uri::Uri,
    },
    peer::Event,
};
use tokio::sync::broadcast;

use crate::{
    peer::error::AlreadySubscribedError,
    subscription::{
        Subscription,
        TypedSubscription,
    },
};

/// A subscription that persists across multiple peer sessions.
pub(crate) struct PersistentSubscription {
    subscription: Arc<Box<dyn Subscription>>,
    current_id: Option<Id>,
}

/// Module for managing persistent subscriptions on a [`Peer`][`crate::peer::Peer`].
///
/// Subscriptions can be created and removed during at any point in a peer's lifetime.
pub(crate) struct Subscriber<S> {
    peer: Arc<battler_wamp::peer::Peer<S>>,
    subscriptions: ahash::HashMap<Uri, PersistentSubscription>,
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
    pub async fn subscribe_typed<T, Event>(&mut self, topic: Uri, subscription: T) -> Result<()>
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
            async fn handle_event(&self, event: battler_wamp::peer::Event) {
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

        match self.subscriptions.entry(topic.clone()) {
            Entry::Occupied(_) => Err(AlreadySubscribedError::new(format!(
                "already actively subscribed to {topic}"
            ))
            .into()),
            Entry::Vacant(entry) => {
                let subscription = entry.insert(PersistentSubscription {
                    subscription: Arc::new(Box::new(SubscriptionWrapper::new(subscription))),
                    current_id: None,
                });
                Self::restore_subscription(&self.peer, &topic, subscription).await
            }
        }
    }

    /// Removes a subscription by topic.
    pub async fn unsubscribe(&mut self, topic: &Uri) -> Result<()> {
        let id = match self
            .subscriptions
            .remove(topic)
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
        mut event_rx: broadcast::Receiver<Event>,
    ) {
        while let Ok(event) = event_rx.recv().await {
            subscription.handle_event(event).await;
        }
    }

    async fn restore_subscription(
        peer: &battler_wamp::peer::Peer<S>,
        topic: &Uri,
        persistent_subscription: &mut PersistentSubscription,
    ) -> Result<()> {
        let subscription = peer.subscribe(topic.clone()).await?;
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
