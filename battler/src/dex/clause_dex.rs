use crate::{
    common::Id,
    config::{
        Clause,
        ClauseData,
    },
    dex::{
        DataLookupResult,
        DataStore,
        ResourceDex,
        ResourceLookup,
        ResourceWrapper,
    },
};

/// Lookup type for [`ClauseDex`].
pub struct ClauseLookup<'d> {
    data: &'d dyn DataStore,
}

impl<'d> ResourceLookup<'d, ClauseData> for ClauseLookup<'d> {
    fn new(data: &'d dyn DataStore) -> Self {
        Self { data }
    }

    fn lookup(&self, id: &Id) -> DataLookupResult<ClauseData> {
        self.data.get_clause(id)
    }
}

/// Wrapper type for [`ClauseDex`].
pub struct ClauseWrapper;

impl ResourceWrapper<ClauseData, Clause> for ClauseWrapper {
    fn wrap(data: ClauseData) -> Clause {
        Clause::new(data)
    }
}

/// Indexed collection of clauses.
pub type ClauseDex<'d> = ResourceDex<'d, ClauseData, Clause, ClauseLookup<'d>, ClauseWrapper>;
