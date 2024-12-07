use std::{
    cell::UnsafeCell,
    sync::Arc,
};

use anyhow::Result;
use futures_util::lock::MutexGuard;

use crate::{
    core::{
        error::InteractionError,
        uri::Uri,
    },
    router::{
        realm::{
            Realm,
            RealmManager,
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

    pub fn router(&self) -> &Router<S> {
        self.router.as_ref()
    }

    pub async fn realm_context(&self, realm: &Uri) -> Result<RealmContext<'_, '_, '_, S>> {
        let mut realm_manager = self.router.realm_manager.lock().await;
        let realm = realm_manager
            .get_mut(realm)
            .ok_or_else(|| InteractionError::NoSuchRealm)?;
        // SAFETY: realm_manager is unused in RealmContext.
        let realm = unsafe { std::mem::transmute(realm) };
        Ok(RealmContext {
            context: self,
            _realm_manager: realm_manager.into(),
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

pub struct RealmContext<'realm, 'realm_manager, 'router, S>
where
    S: 'static,
{
    context: &'router RouterContext<S>,
    // SAFETY: Do not use realm_manager.
    _realm_manager: UnsafeCell<MutexGuard<'realm_manager, RealmManager>>,
    realm: &'realm mut Realm,
}

impl<'realm, 'realm_manager, 'router, S> RealmContext<'realm, 'realm_manager, 'router, S> {
    pub fn router(&self) -> &Router<S> {
        self.context.router()
    }

    pub fn realm(&self) -> &Realm {
        self.realm
    }

    pub fn realm_mut(&mut self) -> &mut Realm {
        self.realm
    }
}
