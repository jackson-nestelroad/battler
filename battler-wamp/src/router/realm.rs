use std::time::Duration;

use anyhow::{
    Error,
    Result,
};
use futures_util::{
    future::join_all,
    lock::Mutex,
};

use crate::{
    core::{
        close::CloseReason,
        hash::HashMap,
        id::Id,
        uri::Uri,
    },
    router::{
        session::SessionHandle,
        topic::TopicManager,
    },
};

#[derive(Debug, Clone)]
pub struct RealmConfig {
    pub name: String,
    pub uri: Uri,
}

pub struct RealmSession {
    pub session: SessionHandle,
}

pub struct Realm {
    pub config: RealmConfig,
    pub sessions: HashMap<Id, RealmSession>,
    pub topic_manager: TopicManager,
}

impl Realm {
    pub fn new(config: RealmConfig) -> Self {
        Self {
            config,
            sessions: HashMap::default(),
            topic_manager: TopicManager::default(),
        }
    }

    pub fn uri(&self) -> &Uri {
        &self.config.uri
    }

    pub async fn shut_down(&mut self, close_reason: CloseReason) -> Result<()> {
        let mut futures = Vec::default();
        for (_, session) in &mut self.sessions {
            session.session.close(close_reason)?;
            futures.push(session.session.closed_session_rx_mut().recv());
        }

        tokio::select! {
            _ = join_all(futures) => {},
            _ = tokio::time::sleep(Duration::from_secs(2)) => {
                return Err(Error::msg("timed out waiting for sessions to close cleanly"));
            }
        }

        self.sessions.clear();
        Ok(())
    }
}

#[derive(Default)]
pub struct RealmManager {
    pub realms: HashMap<Uri, Mutex<Realm>>,
}

impl RealmManager {
    pub fn get(&self, uri: &Uri) -> Option<&Mutex<Realm>> {
        self.realms.get(uri)
    }

    pub fn insert(&mut self, realm: Realm) {
        let uri = realm.uri().clone();
        self.realms.insert(uri, realm.into());
    }

    pub fn uris(&self) -> impl Iterator<Item = &Uri> {
        self.realms.keys()
    }
}
