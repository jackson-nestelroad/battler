use std::{
    collections::hash_map::Entry,
    sync::Arc,
};

use ahash::HashSet;
use anyhow::Result;
use battler_wamp_uri::WildcardUri;
use futures_util::lock::Mutex;
use rand::Rng;
use tokio::sync::RwLock;

use crate::{
    core::{
        error::{
            BasicError,
            InteractionError,
        },
        hash::HashMap,
        id::Id,
        invocation_policy::InvocationPolicy,
        match_style::MatchStyle,
        roles::RouterRole,
    },
    router::context::RealmContext,
};

#[derive(Debug, Default)]
struct ProcedureState {
    active: bool,
}

/// The callee of a [`Procedure`].
#[derive(Debug, Clone)]
pub struct ProcedureCallee {
    /// Session ID.
    pub session: Id,
    /// Registration ID, as known by the session.
    pub registration: Id,
}

#[derive(Debug, Default)]
struct ProcedureRegistration {
    callees: Vec<ProcedureCallee>,
    last_callee_index: usize,
}

impl ProcedureRegistration {
    fn callees_len(&self) -> usize {
        self.callees.len()
    }

    fn filter_available_callees<'a>(
        &'a self,
        callees_attempted: &'a HashSet<Id>,
    ) -> impl Iterator<Item = ProcedureCallee> + 'a {
        self.callees
            .iter()
            .cloned()
            .filter(move |callee| !callees_attempted.contains(&callee.session))
    }

    fn get_callee(
        &mut self,
        invocation_policy: InvocationPolicy,
        callees_attempted: &HashSet<Id>,
    ) -> Result<ProcedureCallee> {
        let callees = self.filter_available_callees(callees_attempted);
        let callees = callees.collect::<Vec<_>>();
        if callees.is_empty() {
            return Err(InteractionError::NoAvailableCallee.into());
        }

        match invocation_policy {
            InvocationPolicy::Single | InvocationPolicy::First => {
                // SAFETY: callees is not empty.
                Ok(callees.first().unwrap().clone())
            }
            InvocationPolicy::RoundRobin => {
                // Pick the next one in the queue, regardless of if we already tried it.
                self.last_callee_index = (self.last_callee_index + 1) % self.callees.len();
                Ok(self.callees[self.last_callee_index].clone())
            }
            InvocationPolicy::Random => {
                let index = rand::rng().random_range(0..callees.len());
                Ok(callees[index].clone())
            }
            InvocationPolicy::Last => {
                // SAFETY: callees is not empty.
                Ok(callees.last().unwrap().clone())
            }
        }
    }

    fn add_callee(&mut self, session: Id, registration: Id) {
        self.callees.push(ProcedureCallee {
            session,
            registration,
        });
    }
}

/// Options for a registered procedure.
pub struct ProcedureOptions {
    pub match_style: Option<MatchStyle>,
    pub invocation_policy: InvocationPolicy,
    pub disclose_caller: bool,
}

/// A procedure that can be invoked by peers to perform some operation on the callee.
pub struct Procedure {
    options: ProcedureOptions,
    registration: Mutex<ProcedureRegistration>,
    state: Mutex<ProcedureState>,
}

impl Procedure {
    /// Gets a callee for a new invocation.
    pub async fn get_callee(&self, callees_attempted: &HashSet<Id>) -> Result<ProcedureCallee> {
        self.registration
            .lock()
            .await
            .get_callee(self.options.invocation_policy, callees_attempted)
    }

    /// Adds a callee to the procedure registration.
    async fn add_callee(&self, callee: Id, registration: Id) -> Result<()> {
        if self.options.invocation_policy == InvocationPolicy::Single
            && self.registration.lock().await.callees_len() > 0
        {
            return Err(BasicError::NotAllowed(
                "procedure does not allow more than one callee".to_owned(),
            )
            .into());
        }
        self.registration
            .lock()
            .await
            .add_callee(callee, registration);
        Ok(())
    }

    /// The caller's identity should be disclosed to the callee.
    pub fn disclose_caller(&self) -> bool {
        self.options.disclose_caller
    }
}

#[derive(Default)]
struct ProcedureNode {
    procedure: Option<Arc<Procedure>>,
    tree: HashMap<String, ProcedureNode>,
}

impl ProcedureNode {
    fn insert<'a>(
        &mut self,
        mut uri_components: impl Iterator<Item = &'a str>,
        procedure: Procedure,
    ) -> Result<Arc<Procedure>> {
        match uri_components.next() {
            Some(uri_component) => {
                if uri_component.is_empty()
                    && !procedure
                        .options
                        .match_style
                        .is_some_and(|match_style| match_style == MatchStyle::Wildcard)
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
            None => match &self.procedure {
                Some(existing) => {
                    if procedure.options.invocation_policy != existing.options.invocation_policy {
                        return Err(InteractionError::ProcedureAlreadyExists.into());
                    }
                    Ok(existing.clone())
                }
                None => {
                    let procedure = Arc::new(procedure);
                    self.procedure = Some(procedure.clone());
                    Ok(procedure)
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

    fn find<'a>(
        &self,
        mut uri_components: impl Iterator<Item = &'a str>,
    ) -> Option<Arc<Procedure>> {
        match uri_components.next() {
            Some(uri_component) => match self.tree.get(uri_component).or_else(|| self.tree.get(""))
            {
                Some(procedure) => procedure.find(uri_components),
                None => self.procedure.clone().filter(|procedure| {
                    procedure
                        .options
                        .match_style
                        .is_some_and(|match_style| match_style == MatchStyle::Prefix)
                }),
            },
            None => self.procedure.clone(),
        }
    }
}

/// A manager for all procedures owned by a realm.
#[derive(Default)]
pub struct ProcedureManager {
    /// Tree of procedures.
    procedures: RwLock<ProcedureNode>,
}

impl ProcedureManager {
    /// Registers a procedure.
    pub async fn register<S>(
        context: &RealmContext<'_, S>,
        session: Id,
        procedure: WildcardUri,
        options: ProcedureOptions,
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

        let procedure = context
            .realm()
            .procedure_manager
            .procedures
            .write()
            .await
            .insert(
                procedure.split(),
                Procedure {
                    options,
                    registration: Mutex::new(ProcedureRegistration::default()),
                    state: Mutex::new(ProcedureState { active: false }),
                },
            )?;
        procedure.add_callee(session, registration_id).await?;

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

    /// Gets the procedure matching the URI.
    pub async fn get<S>(
        context: &RealmContext<'_, S>,
        procedure: &WildcardUri,
    ) -> Option<Arc<Procedure>> {
        context
            .realm()
            .procedure_manager
            .procedures
            .read()
            .await
            .find(procedure.split())
    }
}
