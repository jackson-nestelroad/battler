use std::sync::Arc;

use anyhow::Result;
use futures_util::lock::MutexGuard;

use crate::{
    core::{
        error::InteractionError,
        id::Id,
        uri::Uri,
    },
    router::{
        realm::Realm,
        router::Router,
        session::SessionHandle,
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
    pub async fn realm_context(&self, realm: &Uri) -> Result<RealmContext<'_, '_, S>> {
        let realm = self
            .router
            .realm_manager
            .get(realm)
            .ok_or_else(|| InteractionError::NoSuchRealm)?;
        let realm = realm.lock().await;
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
pub struct RealmContext<'realm, 'router, S>
where
    S: 'static,
{
    context: &'router RouterContext<S>,
    realm: MutexGuard<'realm, Realm>,
}

impl<'realm, 'router, S> RealmContext<'realm, 'router, S> {
    /// The router.
    pub fn router(&self) -> &Router<S> {
        self.context.router()
    }

    /// The realm.
    pub fn realm(&self) -> &Realm {
        &*self.realm
    }

    /// The realm.
    pub fn realm_mut(&mut self) -> &mut Realm {
        &mut *self.realm
    }

    /// Looks up a session by ID.
    pub fn session(&self, id: Id) -> Option<&SessionHandle> {
        self.realm.sessions.get(&id).map(|session| &session.session)
    }

    /// Looks up a session by ID.
    pub fn session_mut(&mut self, id: Id) -> Option<&mut SessionHandle> {
        self.realm
            .sessions
            .get_mut(&id)
            .map(|session| &mut session.session)
    }
}
