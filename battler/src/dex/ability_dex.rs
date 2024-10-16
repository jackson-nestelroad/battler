use crate::{
    abilities::{
        Ability,
        AbilityData,
    },
    common::Id,
    dex::{
        DataStore,
        ResourceDex,
        ResourceLookup,
        ResourceWrapper,
    },
    error::Error,
};

/// Lookup type for [`AbilityDex`].
pub struct AbilityLookup<'d> {
    data: &'d dyn DataStore,
}

impl<'d> ResourceLookup<'d, AbilityData> for AbilityLookup<'d> {
    fn new(data: &'d dyn DataStore) -> Self {
        Self { data }
    }

    fn lookup(&self, id: &Id) -> Result<AbilityData, Error> {
        self.data.get_ability(id)
    }
}

/// Wrapper type for [`AbilityDex`].
pub struct AbilityWrapper;

impl ResourceWrapper<AbilityData, Ability> for AbilityWrapper {
    fn wrap(id: Id, data: AbilityData) -> Ability {
        Ability::new(id, data)
    }
}

/// Indexed collection of abilities.
pub type AbilityDex<'d> = ResourceDex<'d, AbilityData, Ability, AbilityLookup<'d>, AbilityWrapper>;
