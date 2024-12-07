use crate::core::{
    hash::HashMap,
    id::Id,
    uri::Uri,
};

#[derive(Default)]
pub struct TopicSubscriber {
    pub id: Id,
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
    pub fn subscribe(&mut self, session: Id, topic: Uri, id: Id) -> Id {
        let topic = self.topics.entry(topic).or_insert_with(|| Topic::default());
        let subscriber = topic
            .subscribers
            .entry(session)
            .or_insert_with(|| TopicSubscriber { id });
        subscriber.id
    }
}
