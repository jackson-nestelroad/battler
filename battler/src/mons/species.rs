use battler_data::{
    Id,
    Identifiable,
    SpeciesData,
};

/// A Mon species.
#[derive(Debug, Clone)]
pub struct Species {
    id: Id,
    pub data: SpeciesData,
}

impl Species {
    /// Constructs a new [`Species`] instance from [`SpeciesData`].
    pub fn new(id: Id, data: SpeciesData) -> Self {
        Self { id, data }
    }
}

impl Identifiable for Species {
    fn id(&self) -> &Id {
        &self.id
    }
}
