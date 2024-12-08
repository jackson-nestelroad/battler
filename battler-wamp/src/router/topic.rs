use anyhow::Result;

use crate::{
    core::{
        error::BasicError,
        hash::HashMap,
        id::Id,
        types::{
            Dictionary,
            List,
        },
        uri::Uri,
    },
    message::message::{
        EventMessage,
        Message,
    },
    router::context::RealmContext,
};

#[derive(Default)]
pub struct TopicSubscriber {
    pub subscription_id: Id,
}

#[derive(Default)]
pub struct Topic {
    pub subscribers: HashMap<Id, TopicSubscriber>,
}

impl Topic {}

#[derive(Default)]
pub struct TopicManager {
    pub topics: HashMap<Uri, Topic>,
}

impl TopicManager {
    pub async fn subscribe<S>(
        context: &mut RealmContext<'_, '_, S>,
        session: Id,
        topic: Uri,
    ) -> Result<Id> {
        context
            .router()
            .pub_sub_policies
            .lock()
            .await
            .validate_subscription(context, session, &topic)
            .await?;
        let subscription_id = context
            .session(session)
            .ok_or_else(|| BasicError::NotFound(format!("expected session {session} to exist")))?
            .id_generator()
            .generate_id()
            .await;
        let topic = context
            .realm_mut()
            .topic_manager
            .topics
            .entry(topic)
            .or_insert_with(|| Topic::default());
        let subscriber = topic
            .subscribers
            .entry(session)
            .or_insert_with(|| TopicSubscriber { subscription_id });
        Ok(subscriber.subscription_id)
    }

    pub async fn unsubscribe<S>(context: &mut RealmContext<'_, '_, S>, session: Id, topic: &Uri) {
        let topic = match context.realm_mut().topic_manager.topics.get_mut(topic) {
            Some(topic) => topic,
            None => return,
        };
        topic.subscribers.remove(&session);
    }

    pub async fn publish<S>(
        context: &mut RealmContext<'_, '_, S>,
        session: Id,
        topic: &Uri,
        arguments: List,
        arguments_keyword: Dictionary,
    ) -> Result<Id> {
        context
            .router()
            .pub_sub_policies
            .lock()
            .await
            .validate_publication(context, session, &topic)
            .await?;
        let published_id = context.router().id_allocator.generate_id().await;
        let topic = match context.realm().topic_manager.topics.get(topic) {
            Some(topic) => topic,
            None => return Ok(published_id),
        };
        for (session, subscription) in &topic.subscribers {
            let session = match context.realm().sessions.get(&session) {
                Some(session) => &session.session,
                None => continue,
            };
            session.send_message(Message::Event(EventMessage {
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