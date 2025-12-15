use std::{
    sync::Arc,
    time::Duration,
};

use anyhow::{
    Error,
    Result,
};
use battler_wamp_uri::Uri;
use futures_util::future::join_all;
use tokio::sync::RwLock;

use crate::{
    auth::{
        AuthMethod,
        GenericServerAuthenticator,
        make_generic_server_authenticator,
        scram,
        undisputed,
    },
    core::{
        close::CloseReason,
        hash::HashMap,
        id::Id,
    },
    router::{
        procedure::ProcedureManager,
        session::SessionHandle,
        topic::TopicManager,
    },
};

/// Supported authentication types for a realm.
#[derive(Debug, Clone)]
pub enum SupportedAuthMethod {
    /// WAMP-SCRAM.
    WampScram(Arc<Box<dyn scram::UserDatabaseFactory>>),
    /// Undisputed.
    Undisputed,
}

impl SupportedAuthMethod {
    /// The corresponding [`AuthMethod`].
    pub fn auth_method(&self) -> AuthMethod {
        match self {
            Self::WampScram(_) => AuthMethod::WampScram,
            Self::Undisputed => AuthMethod::Undisputed,
        }
    }

    /// Creates a new authenticator for the supported authentication method.
    pub async fn new_authenticator(&self) -> Result<Box<dyn GenericServerAuthenticator>> {
        match self {
            Self::WampScram(user_database) => Ok(make_generic_server_authenticator(Box::new(
                scram::ServerAuthenticator::new(user_database.create_user_database().await?),
            ))),
            Self::Undisputed => Ok(make_generic_server_authenticator(Box::new(
                undisputed::ServerAuthenticator::new(),
            ))),
        }
    }
}

/// Configuration for a realm's authentication.
#[derive(Debug, Default, Clone)]
pub struct RealmAuthenticationConfig {
    /// Is authentication required?
    pub required: bool,
    /// Supported authentication methods.
    ///
    /// Listed in order of selection priority.
    pub methods: Vec<SupportedAuthMethod>,
}

/// Configuration for a realm.
#[derive(Debug, Clone)]
pub struct RealmConfig {
    /// Name of the realm, mostly for logging.
    pub name: String,
    /// URI for peers to connect to the realm.
    pub uri: Uri,
    /// Authentication configuration.
    pub authentication: RealmAuthenticationConfig,
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

    /// Initializes the realm before use.
    pub async fn initialize(&self) -> Result<()> {
        Ok(())
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
