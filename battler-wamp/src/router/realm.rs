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
    router::session::SessionHandle,
};

#[derive(Debug, Clone)]
pub struct RealmConfig {
    pub name: String,
    pub uri: Uri,
}

pub struct Realm {
    pub config: RealmConfig,
    pub sessions: Mutex<HashMap<Id, SessionHandle>>,
}

impl Realm {
    pub fn new(config: RealmConfig) -> Self {
        let sessions = HashMap::default();
        Self {
            config,
            sessions: Mutex::new(sessions),
        }
    }

    pub fn uri(&self) -> &Uri {
        &self.config.uri
    }

    pub async fn shut_down(&self, close_reason: CloseReason) -> Result<()> {
        let mut sessions = self.sessions.lock().await;
        let mut futures = Vec::default();
        for (_, session) in &mut *sessions {
            session.close(close_reason)?;
            futures.push(session.closed_session_rx_mut().recv());
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
    realms: HashMap<Uri, Realm>,
}

impl RealmManager {
    pub fn get(&self, uri: &Uri) -> Option<&Realm> {
        self.realms.get(uri)
    }

    pub fn insert(&mut self, realm: Realm) {
        let uri = realm.uri().clone();
        self.realms.insert(uri, realm);
    }

    pub fn remove(&mut self, uri: &Uri) -> Option<Realm> {
        self.realms.remove(uri)
    }

    pub fn uris(&self) -> impl Iterator<Item = &Uri> {
        self.realms.keys()
    }
}
