use std::sync::Arc;

use anyhow::Result;

use crate::{
    core::{
        error::InteractionError,
        id::Id,
        uri::{
            Uri,
            WildcardUri,
        },
    },
    router::{
        procedure::{
            Procedure,
            ProcedureManager,
        },
        realm::{
            Realm,
            RealmSession,
        },
        router::Router,
        topic::{
            Topic,
            TopicManager,
        },
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

    /// Creates a [`RealmContext`] with a reference to the realm.
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
/// Used for convenient access to a single realm.
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
    pub async fn topic(&self, topic: &WildcardUri) -> Option<Arc<Topic>> {
        TopicManager::get(self, topic).await
    }

    /// Looks up a procedure by URI.
    pub async fn procedure(&self, procedure: &WildcardUri) -> Option<Arc<Procedure>> {
        ProcedureManager::get(self, procedure).await
    }
}
