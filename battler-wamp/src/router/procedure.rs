use std::{
    collections::hash_map::Entry,
    sync::Arc,
};

use anyhow::Result;
use futures_util::lock::Mutex;
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
        uri::WildcardUri,
    },
    router::context::RealmContext,
};

/// The procedure match style.
#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum ProcedureMatchStyle {
    #[default]
    None,
    Prefix,
    Wildcard,
}

impl TryFrom<&str> for ProcedureMatchStyle {
    type Error = anyhow::Error;
    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "none" => Ok(Self::None),
            "prefix" => Ok(Self::Prefix),
            "wildcard" => Ok(Self::Wildcard),
            _ => Err(Self::Error::msg(format!(
                "invalid procedure match style: {value}"
            ))),
        }
    }
}

#[derive(Default)]
struct ProcedureState {
    active: bool,
}

/// A procedure that can be invoked by peers to perform some operation on the callee.
#[derive(Clone)]
pub struct Procedure {
    /// The ID of the procedure.
    pub registration_id: Id,
    /// The session ID of the callee.
    pub callee: Id,

    match_style: ProcedureMatchStyle,

    state: Arc<Mutex<ProcedureState>>,
}

#[derive(Default)]
struct ProcedureNode {
    procedure: Option<Procedure>,
    tree: HashMap<String, ProcedureNode>,
}

impl ProcedureNode {
    fn insert<'a>(
        &mut self,
        mut uri_components: impl Iterator<Item = &'a str>,
        procedure: Procedure,
    ) -> Result<()> {
        match uri_components.next() {
            Some(uri_component) => {
                if uri_component.is_empty()
                    && procedure.match_style != ProcedureMatchStyle::Wildcard
                {
                    return Err(BasicError::InvalidArgument("procedure uri described wildcard match, but wildcard matching was not enabled".to_owned()).into());
                }
                match self.tree.entry(uri_component.to_owned()) {
                    Entry::Occupied(mut entry) => entry.get_mut().insert(uri_components, procedure),
                    Entry::Vacant(entry) => entry
                        .insert(ProcedureNode::default())
                        .insert(uri_components, procedure),
                }
            }
            None => match self.procedure {
                Some(_) => Err(InteractionError::ProcedureAlreadyExists.into()),
                None => {
                    self.procedure = Some(procedure);
                    Ok(())
                }
            },
        }
    }

    fn remove<'a>(&mut self, mut uri_components: impl Iterator<Item = &'a str>) {
        match uri_components.next() {
            Some(uri_component) => match self.tree.get_mut(uri_component) {
                Some(node) => node.remove(uri_components),
                None => (),
            },
            None => self.procedure = None,
        }
    }

    fn find<'a>(&self, mut uri_components: impl Iterator<Item = &'a str>) -> Option<&Procedure> {
        match uri_components.next() {
            Some(uri_component) => match self.tree.get(uri_component).or_else(|| self.tree.get(""))
            {
                Some(procedure) => procedure.find(uri_components),
                None => self
                    .procedure
                    .as_ref()
                    .filter(|procedure| procedure.match_style == ProcedureMatchStyle::Prefix),
            },
            None => self.procedure.as_ref(),
        }
    }
}

/// A manager for all procedures owned by a realm.
#[derive(Default)]
pub struct ProcedureManager {
    /// Map of procedures.
    procedures: RwLock<ProcedureNode>,
}

impl ProcedureManager {
    /// Registers a procedure.
    pub async fn register<S>(
        context: &RealmContext<'_, S>,
        session: Id,
        procedure: WildcardUri,
        match_style: ProcedureMatchStyle,
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

        context
            .realm()
            .procedure_manager
            .procedures
            .write()
            .await
            .insert(
                procedure.split(),
                Procedure {
                    registration_id,
                    callee: session,
                    match_style,
                    state: Arc::new(Mutex::new(ProcedureState { active: false })),
                },
            )?;
        Ok(registration_id)
    }

    /// Activates a callee's procedure.
    ///
    /// Required for proper ordering of messages. The procedure should not receive invocations until
    /// after the peer has received the registration confirmation.
    pub async fn activate_procedure<S>(context: &RealmContext<'_, S>, procedure: &WildcardUri) {
        if let Some(procedure) = context.procedure(procedure).await {
            procedure.state.lock().await.active = true;
        }
    }

    /// Deregisters a procedure.
    pub async fn unregister<S>(context: &RealmContext<'_, S>, procedure: &WildcardUri) {
        context
            .realm()
            .procedure_manager
            .procedures
            .write()
            .await
            .remove(procedure.split());
    }

    pub async fn get<S>(
        context: &RealmContext<'_, S>,
        procedure: &WildcardUri,
    ) -> Option<Procedure> {
        context
            .realm()
            .procedure_manager
            .procedures
            .read()
            .await
            .find(procedure.split())
            .cloned()
    }
}
