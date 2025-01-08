use std::{
    collections::hash_map::Entry,
    sync::Arc,
};

use anyhow::Result;
use tokio::sync::RwLock;

use crate::{
    core::{
        error::{
            BasicError,
            InteractionError,
        },
        hash::HashMap,
        id::Id,
        roles::RouterRole,
        uri::Uri,
    },
    router::context::RealmContext,
};

/// A procedure that can be invoked by peers to perform some operation on the callee.
pub struct Procedure {
    /// The ID of the procedure.
    pub registration_id: Id,
    /// The session ID of the callee.
    pub callee: Id,

    active: bool,
}

/// A manager for all procedures owned by a realm.
#[derive(Default)]
pub struct ProcedureManager {
    /// Map of procedures.
    pub procedures: RwLock<HashMap<Uri, Arc<RwLock<Procedure>>>>,
}

impl ProcedureManager {
    /// Registers a procedure.
    pub async fn register<S>(
        context: &RealmContext<'_, S>,
        session: Id,
        procedure: Uri,
    ) -> Result<Id> {
        if !context.router().config.roles.contains(&RouterRole::Dealer) {
            return Err(BasicError::NotAllowed("router is not a dealer".to_owned()).into());
        }
        if context
            .session(session)
            .await
            .ok_or_else(|| BasicError::NotFound("expected callee session to exist".to_owned()))?
            .session
            .roles()
            .await
            .callee
            .is_none()
        {
            return Err(BasicError::NotAllowed("peer is not a callee".to_owned()).into());
        }

        context
            .router()
            .rpc_policies
            .validate_registration(&context, session, &procedure)
            .await?;
        let registration_id = context.router().id_allocator.generate_id().await;
        match context
            .realm()
            .procedure_manager
            .procedures
            .write()
            .await
            .entry(procedure)
        {
            Entry::Occupied(_) => return Err(InteractionError::ProcedureAlreadyExists.into()),
            Entry::Vacant(entry) => {
                entry.insert(Arc::new(RwLock::new(Procedure {
                    registration_id,
                    callee: session,
                    active: false,
                })));
            }
        }
        Ok(registration_id)
    }

    /// Activates a callee's procedure.
    ///
    /// Required for proper ordering of messages. The procedure should not receive invocations until
    /// after the peer has received the registration confirmation.
    pub async fn activate_procedure<S>(context: &RealmContext<'_, S>, procedure: &Uri) {
        if let Some(procedure) = context.procedure(procedure).await {
            procedure.write().await.active = true;
        }
    }

    /// Deregisters a procedure.
    pub async fn unregister<S>(context: &RealmContext<'_, S>, procedure: &Uri) {
        context
            .realm()
            .procedure_manager
            .procedures
            .write()
            .await
            .remove(procedure);
    }
}
