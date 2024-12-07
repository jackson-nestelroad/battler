use std::sync::Arc;

use futures_util::lock::Mutex;

use crate::router::{
    realm::RealmManager,
    router::Router,
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

    pub fn realm_manager(&self) -> &Mutex<RealmManager> {
        &self.router.realm_manager
    }
}

impl<S> Clone for RouterContext<S> {
    fn clone(&self) -> Self {
        Self {
            router: self.router.clone(),
        }
    }
}
