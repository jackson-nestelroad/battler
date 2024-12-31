use std::sync::Arc;

use anyhow::Result;
use battler_wamp_values::{
    Dictionary,
    List,
};
use tokio::sync::RwLock;

use crate::{
    core::{
        error::BasicError,
        hash::HashMap,
        id::Id,
        roles::RouterRole,
        uri::Uri,
    },
    message::message::{
        EventMessage,
        Message,
    },
    router::context::RealmContext,
};

/// A single subscriber to a topic.
#[derive(Default)]
pub struct TopicSubscriber {
    subscription_id: Id,
    active: bool,
}

/// A topic that events can be published to for subscribers.
#[derive(Default)]
pub struct Topic {
    /// All subscribers to the topic.
    ///
    /// Key is session ID.
    pub subscribers: RwLock<HashMap<Id, TopicSubscriber>>,
}

/// A manager for all topics owned by a realm.
#[derive(Default)]
pub struct TopicManager {
    /// Map of topics.
    pub topics: RwLock<HashMap<Uri, Arc<Topic>>>,
}

impl TopicManager {
    /// Subscribes to a topic.
    pub async fn subscribe<S>(
        context: &RealmContext<'_, S>,
        session: Id,
        topic: Uri,
    ) -> Result<Id> {
        if !context.router().config.roles.contains(&RouterRole::Broker) {
            return Err(BasicError::NotAllowed("router is not a broker".to_owned()).into());
        }

        context
            .router()
            .pub_sub_policies
            .validate_subscription(context, session, &topic)
            .await?;
        let subscription_id = context
            .session(session)
            .await
            .ok_or_else(|| BasicError::NotFound(format!("expected session {session} to exist")))?
            .session
            .id_generator()
            .generate_id()
            .await;
        let topic = context
            .realm()
            .topic_manager
            .topics
            .write()
            .await
            .entry(topic)
            .or_insert_with(|| Arc::new(Topic::default()))
            .clone();
        let subscription_id = topic
            .subscribers
            .write()
            .await
            .entry(session)
            .or_insert_with(|| TopicSubscriber {
                subscription_id,
                active: false,
            })
            .subscription_id;
        Ok(subscription_id)
    }

    /// Activates a subscriber's subscription.
    ///
    /// Required for proper ordering of events. The subscription should not receive events until
    /// after the peer has received the subscription confirmation.
    pub async fn activate_subscription<S>(context: &RealmContext<'_, S>, session: Id, topic: &Uri) {
        if let Some(topic) = context.topic(topic).await {
            if let Some(subscriber) = topic.subscribers.write().await.get_mut(&session) {
                subscriber.active = true;
            }
        }
    }

    /// Unsubscribes from a topic.
    pub async fn unsubscribe<S>(context: &RealmContext<'_, S>, session: Id, topic: &Uri) {
        let topic = match context.topic(topic).await {
            Some(topic) => topic,
            None => return,
        };
        topic.subscribers.write().await.remove(&session);
    }

    /// Publishes an event to a topic.
    pub async fn publish<S>(
        context: &RealmContext<'_, S>,
        session: Id,
        topic: &Uri,
        arguments: List,
        arguments_keyword: Dictionary,
    ) -> Result<Id> {
        if !context.router().config.roles.contains(&RouterRole::Broker) {
            return Err(BasicError::NotAllowed("router is not a broker".to_owned()).into());
        }

        context
            .router()
            .pub_sub_policies
            .validate_publication(context, session, &topic)
            .await?;
        let published_id = context.router().id_allocator.generate_id().await;
        let topic = match context.topic(topic).await {
            Some(topic) => topic,
            None => return Ok(published_id),
        };
        for (session, subscription) in topic.subscribers.read().await.iter() {
            if !subscription.active {
                continue;
            }
            let session = match context.realm().sessions.read().await.get(&session).cloned() {
                Some(session) => session,
                None => continue,
            };
            session.session.send_message(Message::Event(EventMessage {
                subscribed_subscription: subscription.subscription_id,
                published_publication: published_id,
                details: Dictionary::default(),
                publish_arguments: arguments.clone(),
                publish_arguments_keyword: arguments_keyword.clone(),
            }))?;
        }
        Ok(published_id)
    }
}
