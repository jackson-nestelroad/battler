use std::time::Duration;

use anyhow::{
    Error,
    Result,
};
use futures_util::{
    future::join_all,
    lock::Mutex,
};
use zone_alloc::{
    BorrowError,
    ElementRefMut,
    KeyedRegistry,
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
    pub sessions: Mutex<HashMap<Id, RealmSession>>,
    pub topic_manager: TopicManager,
}

impl Realm {
    pub fn new(config: RealmConfig) -> Self {
        let sessions = HashMap::default();
        Self {
            config,
            sessions: Mutex::new(sessions),
            topic_manager: TopicManager::default(),
        }
    }

    pub fn uri(&self) -> &Uri {
        &self.config.uri
    }

    pub async fn shut_down(&self, close_reason: CloseReason) -> Result<()> {
        let mut sessions = self.sessions.lock().await;
        let mut futures = Vec::default();
        for (_, session) in &mut *sessions {
            session.session.close(close_reason)?;
            futures.push(session.session.closed_session_rx_mut().recv());
        }

        tokio::select! {
            _ = join_all(futures) => {},
            _ = tokio::time::sleep(Duration::from_secs(2)) => {
                return Err(Error::msg("timed out waiting for sessions to close cleanly"));
            }
        }

        Ok(())
    }
}

#[derive(Default)]
pub struct RealmManager {
    pub realms: KeyedRegistry<Uri, Realm>,
}

impl RealmManager {
    pub fn get_mut(&mut self, uri: &Uri) -> Result<Option<ElementRefMut<'_, Realm>>> {
        match self.realms.get_mut(uri) {
            Ok(realm) => Ok(Some(realm)),
            Err(BorrowError::OutOfBounds) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    pub fn insert(&mut self, realm: Realm) {
        let uri = realm.uri().clone();
        self.realms.register(uri, realm);
    }

    pub fn uris(&self) -> impl Iterator<Item = &Uri> {
        self.realms.keys()
    }
}
