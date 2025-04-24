use std::{
    collections::{
        VecDeque,
        hash_map::Entry,
    },
    sync::Arc,
};

use anyhow::Result;
use battler_wamp_values::{
    Dictionary,
    List,
    Value,
};
use tokio::sync::RwLock;

use crate::{
    core::{
        error::BasicError,
        hash::HashMap,
        id::Id,
        match_style::MatchStyle,
        publish_options::PublishOptions,
        roles::RouterRole,
        uri::{
            Uri,
            WildcardUri,
        },
    },
    message::message::{
        EventMessage,
        Message,
    },
    router::{
        context::RealmContext,
        realm::RealmSession,
    },
};

/// A single subscriber to a topic.
#[derive(Default)]
pub struct TopicSubscriber {
    subscription_id: Id,
    active: bool,
    match_style: Option<MatchStyle>,
}

/// A topic that events can be published to for subscribers.
#[derive(Default)]
pub struct Topic {
    /// All subscribers to the topic.
    ///
    /// Key is session ID.
    pub subscribers: RwLock<HashMap<Id, TopicSubscriber>>,
}

#[derive(Default)]
struct TopicNode {
    topic: Arc<Topic>,
    tree: HashMap<String, TopicNode>,
}

impl TopicNode {
    fn get<'a>(&self, mut uri_components: impl Iterator<Item = &'a str>) -> Option<Arc<Topic>> {
        match uri_components.next() {
            Some(uri_component) => self
                .tree
                .get(uri_component)
                .and_then(|topic| topic.get(uri_components)),
            None => Some(self.topic.clone()),
        }
    }

    fn get_or_insert<'a>(
        &mut self,
        mut uri_components: impl Iterator<Item = &'a str>,
    ) -> Arc<Topic> {
        match uri_components.next() {
            Some(uri_component) => match self.tree.entry(uri_component.to_owned()) {
                Entry::Occupied(mut entry) => entry.get_mut().get_or_insert(uri_components),
                Entry::Vacant(entry) => entry
                    .insert(TopicNode::default())
                    .get_or_insert(uri_components),
            },
            None => self.topic.clone(),
        }
    }

    fn get_all<'a>(
        &self,
        mut uri_components: impl Iterator<Item = &'a str> + Clone,
        match_style: Option<MatchStyle>,
        topics: &mut VecDeque<(Arc<Topic>, Option<MatchStyle>)>,
    ) {
        match uri_components.next() {
            Some(uri_component) => {
                topics.push_back((self.topic.clone(), Some(MatchStyle::Prefix)));
                if let Some(topic) = self.tree.get(uri_component) {
                    topic.get_all(uri_components.clone(), match_style, topics);
                }
                if let Some(topic) = self.tree.get("") {
                    topic.get_all(uri_components, Some(MatchStyle::Wildcard), topics);
                }
            }
            None => {
                topics.push_back((self.topic.clone(), match_style));
            }
        }
    }
}

/// A manager for all topics owned by a realm.
#[derive(Default)]
pub struct TopicManager {
    /// Tree of topics.
    topics: RwLock<TopicNode>,
}

impl TopicManager {
    /// Subscribes to a topic.
    pub async fn subscribe<S>(
        context: &RealmContext<'_, S>,
        session: Id,
        topic: WildcardUri,
        match_style: Option<MatchStyle>,
    ) -> Result<Id> {
        if !context.router().config.roles.contains(&RouterRole::Broker) {
            return Err(BasicError::NotAllowed("router is not a broker".to_owned()).into());
        }
        if context
            .session(session)
            .await
            .ok_or_else(|| BasicError::NotFound("expected subscriber session to exist".to_owned()))?
            .session
            .roles()
            .await
            .subscriber
            .is_none()
        {
            return Err(BasicError::NotAllowed("peer is not a subscriber".to_owned()).into());
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
            .get_or_insert(topic.split());
        let subscription_id = topic
            .subscribers
            .write()
            .await
            .entry(session)
            .or_insert_with(|| TopicSubscriber {
                subscription_id,
                active: false,
                match_style,
            })
            .subscription_id;
        Ok(subscription_id)
    }

    /// Activates a subscriber's subscription.
    ///
    /// Required for proper ordering of events. The subscription should not receive events until
    /// after the peer has received the subscription confirmation.
    pub async fn activate_subscription<S>(
        context: &RealmContext<'_, S>,
        session: Id,
        topic: &WildcardUri,
    ) {
        if let Some(topic) = context.topic(topic).await {
            if let Some(subscriber) = topic.subscribers.write().await.get_mut(&session) {
                subscriber.active = true;
            }
        }
    }

    /// Unsubscribes from a topic.
    pub async fn unsubscribe<S>(context: &RealmContext<'_, S>, session: Id, topic: &WildcardUri) {
        let topic = match context.topic(topic).await {
            Some(topic) => topic,
            None => return,
        };
        topic.subscribers.write().await.remove(&session);
    }

    /// Publishes an event to a topic.
    pub async fn publish<S>(
        context: &RealmContext<'_, S>,
        publisher: Id,
        topic: &Uri,
        arguments: List,
        arguments_keyword: Dictionary,
        options: PublishOptions,
    ) -> Result<Id> {
        if !context.router().config.roles.contains(&RouterRole::Broker) {
            return Err(BasicError::NotAllowed("router is not a broker".to_owned()).into());
        }
        if context
            .session(publisher)
            .await
            .ok_or_else(|| BasicError::NotFound("expected publisher session to exist".to_owned()))?
            .session
            .roles()
            .await
            .publisher
            .is_none()
        {
            return Err(BasicError::NotAllowed("peer is not a publisher".to_owned()).into());
        }

        context
            .router()
            .pub_sub_policies
            .validate_publication(context, publisher, &topic)
            .await?;
        let published_id = context.router().id_allocator.generate_id().await;

        let mut topics = VecDeque::new();
        context
            .realm()
            .topic_manager
            .topics
            .read()
            .await
            .get_all(topic.split(), None, &mut topics);

        for (single_topic, required_match_style) in topics {
            Self::publish_to_topic(
                context,
                publisher,
                published_id,
                topic,
                single_topic,
                required_match_style,
                arguments.clone(),
                arguments_keyword.clone(),
                &options,
            )
            .await;
        }

        Ok(published_id)
    }

    async fn authorized_to_receive_event(session: &RealmSession, options: &PublishOptions) -> bool {
        let mut authorized = true;
        if let Some(eligible) = &options.eligible {
            authorized = authorized && eligible.contains(&session.session.id())
        }
        if let Some(eligible_authid) = &options.eligible_authid {
            authorized = authorized
                && eligible_authid
                    .contains(&session.session.identity().await.unwrap_or_default().id)
        }
        if let Some(eligible_authrole) = &options.eligible_authrole {
            authorized = authorized
                && eligible_authrole
                    .contains(&session.session.identity().await.unwrap_or_default().role)
        }
        if let Some(exclude) = &options.exclude {
            authorized = authorized && !exclude.contains(&session.session.id())
        }
        if let Some(exclude_authid) = &options.exclude_authid {
            authorized = authorized
                && !exclude_authid
                    .contains(&session.session.identity().await.unwrap_or_default().id)
        }
        if let Some(exclude_authrole) = &options.exclude_authrole {
            authorized = authorized
                && !exclude_authrole
                    .contains(&session.session.identity().await.unwrap_or_default().role)
        }
        authorized
    }

    async fn publish_to_topic<S>(
        context: &RealmContext<'_, S>,
        publisher: Id,
        published_id: Id,
        topic: &Uri,
        single_topic: Arc<Topic>,
        required_match_style: Option<MatchStyle>,
        arguments: List,
        arguments_keyword: Dictionary,
        options: &PublishOptions,
    ) {
        for (session, subscription) in single_topic.subscribers.read().await.iter() {
            if !subscription.active {
                continue;
            }
            match (required_match_style, subscription.match_style) {
                // Exact match should match anything.
                (None, _) => (),
                (Some(MatchStyle::Prefix), Some(MatchStyle::Prefix)) => (),
                (Some(MatchStyle::Wildcard), Some(MatchStyle::Wildcard)) => (),
                _ => continue,
            }

            if *session == publisher && options.exclude_me {
                continue;
            }

            let session = match context.realm().sessions.read().await.get(&session).cloned() {
                Some(session) => session,
                None => continue,
            };

            if !Self::authorized_to_receive_event(&session, options).await {
                continue;
            }

            let mut details = Dictionary::default();
            details.insert("topic".to_owned(), Value::String(topic.to_string()));

            session
                .session
                .send_message(Message::Event(EventMessage {
                    subscribed_subscription: subscription.subscription_id,
                    published_publication: published_id,
                    details,
                    publish_arguments: arguments.clone(),
                    publish_arguments_keyword: arguments_keyword.clone(),
                }))
                .await
                .ok();
        }
    }

    /// Gets the topic matching the URI.
    pub async fn get<S>(context: &RealmContext<'_, S>, topic: &WildcardUri) -> Option<Arc<Topic>> {
        context
            .realm()
            .topic_manager
            .topics
            .read()
            .await
            .get(topic.split())
    }
}
