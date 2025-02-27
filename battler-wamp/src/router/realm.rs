use std::{
    sync::Arc,
    time::Duration,
};

use anyhow::{
    Error,
    Result,
};
use futures_util::future::join_all;
use tokio::sync::RwLock;

use crate::{
    core::{
        close::CloseReason,
        hash::HashMap,
        id::Id,
        uri::Uri,
    },
    router::{
        procedure::ProcedureManager,
        session::SessionHandle,
        topic::TopicManager,
    },
};

/// Configuration for a realm.
#[derive(Debug, Clone)]
pub struct RealmConfig {
    /// Name of the realm, mostly for logging.
    pub name: String,
    /// URI for peers to connect to the realm.
    pub uri: Uri,
}

/// A single session on a realm.
pub struct RealmSession {
    pub session: SessionHandle,
}

/// A realm, which is a scoped area for peer sessions and resources.
///
/// WAMP sessions cannot communicate across realms.
pub struct Realm {
    /// The realm configuration when created.
    pub config: RealmConfig,

    /// Sessions in the realm.
    pub sessions: RwLock<HashMap<Id, Arc<RealmSession>>>,

    /// Topic manager for pub/sub functionality.
    pub topic_manager: TopicManager,

    /// Procedure manager for RPC functionality.
    pub procedure_manager: ProcedureManager,
}

impl Realm {
    /// Creates a new realm.
    pub fn new(config: RealmConfig) -> Self {
        Self {
            config,
            sessions: RwLock::new(HashMap::default()),
            topic_manager: TopicManager::default(),
            procedure_manager: ProcedureManager::default(),
        }
    }

    /// The URI for accessing the realm.
    pub fn uri(&self) -> &Uri {
        &self.config.uri
    }

    /// Shuts down the realm by attempting to end all sessions cleanly.
    ///
    /// If sessions cannot be cleaned up properly, everything will be dropped anyway.
    pub async fn shut_down(&self, close_reason: CloseReason) -> Result<()> {
        let mut futures = Vec::default();
        for session in self.sessions.read().await.values().cloned() {
            futures.push((async |session: Arc<RealmSession>| {
                session.session.closed_session_rx().recv().await
            })(session.clone()));
            session.session.close(close_reason).await.ok();
        }

        tokio::select! {
            _ = join_all(futures) => {},
            _ = tokio::time::sleep(Duration::from_secs(2)) => {
                return Err(Error::msg("timed out waiting for sessions to close cleanly"));
            }
        }

        self.sessions.write().await.clear();
        Ok(())
    }
}

/// A manager for all realms owned by a router.
#[derive(Default)]
pub struct RealmManager {
    /// Map of realms.
    pub realms: HashMap<Uri, Arc<Realm>>,
}

impl RealmManager {
    /// Looks up realm by URI.
    pub fn get(&self, uri: &Uri) -> Option<Arc<Realm>> {
        self.realms.get(uri).cloned()
    }

    /// Inserts a new realm.
    pub fn insert(&mut self, realm: Realm) {
        let uri = realm.uri().clone();
        self.realms.insert(uri, Arc::new(realm));
    }

    /// Returns an iterator over all realm URIs.
    pub fn uris(&self) -> impl Iterator<Item = &Uri> {
        self.realms.keys()
    }
}
