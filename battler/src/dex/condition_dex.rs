use anyhow::Result;
use battler_data::{
    ConditionData,
    DataStore,
    Id,
};

use crate::{
    conditions::Condition,
    dex::{
        ResourceDex,
        ResourceLookup,
        ResourceWrapper,
    },
    WrapOptionError,
};

/// Lookup type for [`ConditionDex`].
#[derive(Clone)]
pub struct ConditionLookup<'d> {
    data: &'d dyn DataStore,
}

impl<'d> ResourceLookup<'d, ConditionData> for ConditionLookup<'d> {
    fn new(data: &'d dyn DataStore) -> Self {
        Self { data }
    }

    fn lookup(&self, id: &Id) -> Result<ConditionData> {
        self.data
            .get_condition(id)?
            .wrap_not_found_error_with_format(format_args!("condition {id}"))
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
