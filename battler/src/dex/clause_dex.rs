use anyhow::Result;
use battler_data::{
    ClauseData,
    DataStore,
    Id,
};

use crate::{
    config::Clause,
    dex::{
        ResourceDex,
        ResourceLookup,
        ResourceWrapper,
    },
    WrapOptionError,
};

/// Lookup type for [`ClauseDex`].
#[derive(Clone)]
pub struct ClauseLookup<'d> {
    data: &'d dyn DataStore,
}

impl<'d> ResourceLookup<'d, ClauseData> for ClauseLookup<'d> {
    fn new(data: &'d dyn DataStore) -> Self {
        Self { data }
    }

    fn lookup(&self, id: &Id) -> Result<ClauseData> {
        self.data
            .get_clause(id)?
            .wrap_not_found_error_with_format(format_args!("clause {id}"))
    }
}

/// Wrapper type for [`ClauseDex`].
pub struct ClauseWrapper;

impl ResourceWrapper<ClauseData, Clause> for ClauseWrapper {
    fn wrap(id: Id, data: ClauseData) -> Clause {
        Clause::new(id, data)
    }
}

/// Indexed collection of clauses.
pub type ClauseDex<'d> = ResourceDex<'d, ClauseData, Clause, ClauseLookup<'d>, ClauseWrapper>;
