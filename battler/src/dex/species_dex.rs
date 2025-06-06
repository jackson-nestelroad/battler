use anyhow::Result;
use battler_data::{
    DataStore,
    Id,
    SpeciesData,
};

use crate::{
    dex::{
        ResourceDex,
        ResourceLookup,
        ResourceWrapper,
    },
    mons::Species,
    WrapOptionError,
};

/// Lookup type for [`SpeciesDex`].
#[derive(Clone)]
pub struct SpeciesLookup<'d> {
    data: &'d dyn DataStore,
}

impl<'d> ResourceLookup<'d, SpeciesData> for SpeciesLookup<'d> {
    fn new(data: &'d dyn DataStore) -> Self {
        Self { data }
    }

    fn lookup(&self, id: &Id) -> Result<SpeciesData> {
        self.data
            .get_species(id)?
            .wrap_not_found_error_with_format(format_args!("species {id}"))
    }

    fn lookup_alias(&self, alias: &Id, real_id: &Id) -> Result<SpeciesData> {
        let data = self
            .data
            .get_species(real_id)?
            .wrap_not_found_error_with_format(format_args!("species {real_id}"))?;

        // Cosmetic formes do not have their own SpeciesData, so we must generate it ourselves.
        if let Some(cosmetic_forme) = data
            .cosmetic_formes
            .iter()
            .find(|forme| Id::from(forme.as_ref()) == *alias)
            .cloned()
        {
            let cosmetic_forme_data = data.create_cosmetic_forme_data(cosmetic_forme);
            return Ok(cosmetic_forme_data);
        }
        Ok(data)
    }
}

/// Wrapper type for [`SpeciesDex`].
pub struct SpeciesWrapper;

impl ResourceWrapper<SpeciesData, Species> for SpeciesWrapper {
    fn wrap(id: Id, data: SpeciesData) -> Species {
        Species::new(id, data)
    }
}

/// Indexed collection of species.
pub type SpeciesDex<'d> = ResourceDex<'d, SpeciesData, Species, SpeciesLookup<'d>, SpeciesWrapper>;
