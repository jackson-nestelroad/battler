use std::sync::Arc;

use anyhow::Result;
use tokio::sync::RwLock;

use super::{
    procedure::Procedure,
    topic::Topic,
};
use crate::{
    core::{
        error::InteractionError,
        id::Id,
        uri::Uri,
    },
    router::{
        realm::{
            Realm,
            RealmSession,
        },
        router::Router,
    },
};

/// The context of a task running for a router.
///
/// Used to share ownership of the router across many tasks.
pub struct RouterContext<S>
where
    S: 'static,
{
    router: Arc<Router<S>>,
}

impl<S> RouterContext<S> {
    /// Constructs a new context wrapper around a router.
    pub fn new(router: Router<S>) -> Self {
        Self {
            router: Arc::new(router),
        }
    }

    /// The router.
    pub fn router(&self) -> &Router<S> {
        self.router.as_ref()
    }

    /// Creates a [`RealmContext`] with the given realm locked and ready for use.
    pub fn realm_context(&self, realm: &Uri) -> Result<RealmContext<'_, S>> {
        let realm = self
            .router
            .realm_manager
            .get(realm)
            .ok_or_else(|| InteractionError::NoSuchRealm)?;
        Ok(RealmContext {
            context: self,
            realm,
        })
    }
}

impl<S> Clone for RouterContext<S> {
    fn clone(&self) -> Self {
        Self {
            router: self.router.clone(),
        }
    }
}

/// The context of a task running for a realm.
///
/// Used to lock the realm for use.
pub struct RealmContext<'router, S>
where
    S: 'static,
{
    context: &'router RouterContext<S>,
    realm: Arc<Realm>,
}

impl<'router, S> RealmContext<'router, S> {
    /// The router.
    pub fn router(&self) -> &Router<S> {
        self.context.router()
    }

    /// The realm.
    pub fn realm(&self) -> &Realm {
        &*self.realm
    }

    /// Looks up a session by ID.
    pub async fn session(&self, id: Id) -> Option<Arc<RealmSession>> {
        self.realm.sessions.read().await.get(&id).cloned()
    }

    /// Looks up a topic by URI.
    pub async fn topic(&self, topic: &Uri) -> Option<Arc<Topic>> {
        self.realm
            .topic_manager
            .topics
            .read()
            .await
            .get(topic)
            .cloned()
    }

    /// Looks up a procedure by URI.
    pub async fn procedure(&self, procedure: &Uri) -> Option<Arc<RwLock<Procedure>>> {
        self.realm
            .procedure_manager
            .procedures
            .read()
            .await
            .get(procedure)
            .cloned()
    }
}
