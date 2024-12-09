use std::collections::hash_map::Entry;

use anyhow::Result;

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
}

/// A manager for all procedures owned by a realm.
#[derive(Default)]
pub struct ProcedureManager {
    /// Map of procedures.
    pub procedures: HashMap<Uri, Procedure>,
}

impl ProcedureManager {
    /// Registers a procedure.
    pub async fn register<S>(
        context: &mut RealmContext<'_, '_, S>,
        session: Id,
        procedure: Uri,
    ) -> Result<Id> {
        if !context.router().config.roles.contains(&RouterRole::Dealer) {
            return Err(BasicError::NotAllowed("router is not a dealer".to_owned()).into());
        }

        context
            .router()
            .rpc_policies
            .validate_registration(&context, session, &procedure)
            .await?;
        let registration_id = context.router().id_allocator.generate_id().await;
        match context
            .realm_mut()
            .procedure_manager
            .procedures
            .entry(procedure)
        {
            Entry::Occupied(_) => return Err(InteractionError::ProcedureAlreadyExists.into()),
            Entry::Vacant(entry) => {
                entry.insert(Procedure {
                    registration_id,
                    callee: session,
                });
            }
        }
        Ok(registration_id)
    }

    /// Deregisters a procedure.
    pub async fn unregister<S>(context: &mut RealmContext<'_, '_, S>, procedure: &Uri) {
        context
            .realm_mut()
            .procedure_manager
            .procedures
            .remove(procedure);
    }
}
