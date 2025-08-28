use battler_data::{
    Id,
    Identifiable,
    SpeciesData,
};

use crate::effect::fxlang;

/// A Mon species.
#[derive(Debug, Clone)]
pub struct Species {
    id: Id,
    pub data: SpeciesData,
    pub effect: fxlang::Effect,
}

impl Species {
    /// Constructs a new [`Species`] instance from [`SpeciesData`].
    pub fn new(id: Id, data: SpeciesData) -> Self {
        let effect = data.effect.clone().try_into().unwrap_or_default();
        Self { id, data, effect }
    }
}

impl Identifiable for Species {
    fn id(&self) -> &Id {
        &self.id
    }
}
