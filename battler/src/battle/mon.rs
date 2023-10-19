use crate::{
    mons::Gender,
    teams::MonData,
};

/// Public [`Mon`] details, which are shared to both sides of a battle when the Mon appears or
/// during Team Preview.
pub struct PublicMonDetails<'d> {
    pub species_name: &'d str,
    pub level: u8,
    pub gender: Gender,
    pub shiny: bool,
}

/// A [`Mon`] in a battle, which battles against other Mons.
pub struct Mon {
    pub data: MonData,
    pub player: usize,

    active: bool,
}

// Block for getters.
impl Mon {
    pub fn active(&self) -> bool {
        self.active
    }
}

impl Mon {
    /// Creates a new [`Mon`] instance from [`MonData`].
    pub fn new(data: MonData) -> Self {
        Self {
            data,
            player: usize::MAX,

            active: false,
        }
    }

    /// Returns the public details for the Mon.
    pub fn public_details(&self) -> PublicMonDetails {
        PublicMonDetails {
            species_name: &self.data.species,
            level: self.data.level,
            gender: self.data.gender.clone(),
            shiny: self.data.shiny,
        }
    }
}
