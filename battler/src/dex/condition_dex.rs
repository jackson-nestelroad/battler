use crate::{
    common::Id,
    conditions::{
        Condition,
        ConditionData,
    },
    dex::{
        DataStore,
        ResourceDex,
        ResourceLookup,
        ResourceWrapper,
    },
    error::Error,
};

/// Lookup type for [`ConditionDex`].
pub struct ConditionLookup<'d> {
    data: &'d dyn DataStore,
}

impl<'d> ResourceLookup<'d, ConditionData> for ConditionLookup<'d> {
    fn new(data: &'d dyn DataStore) -> Self {
        Self { data }
    }

    fn lookup(&self, id: &Id) -> Result<ConditionData, Error> {
        self.data.get_condition(id)
    }
}

/// Wrapper type for [`ConditionDex`].
pub struct ConditionWrapper;

impl ResourceWrapper<ConditionData, Condition> for ConditionWrapper {
    fn wrap(id: Id, data: ConditionData) -> Condition {
        Condition::new(id, data)
    }
}

/// Indexed collection of conditions.
pub type ConditionDex<'d> =
    ResourceDex<'d, ConditionData, Condition, ConditionLookup<'d>, ConditionWrapper>;
