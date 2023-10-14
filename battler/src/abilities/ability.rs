use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    abilities::AbilityFlags,
    common::{
        FastHashSet,
        Id,
        Identifiable,
    },
};

/// Data about a particular ability.
///
/// Every Mon has one ability, which affects the battle in a wide variety of ways.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbilityData {
    /// Name of the ability.
    pub name: String,
    /// Ability flags.
    pub flags: FastHashSet<AbilityFlags>,
}

/// An individual ability on a Mon that affects the battle in a wide variety of ways.
#[derive(Clone)]
pub struct Ability {
    pub data: AbilityData,
    id: Id,
}

impl Ability {
    /// Creates a new [`Ability`] instance from [`AbilityData`].
    pub fn new(data: AbilityData) -> Self {
        let id = Id::from(data.name.as_ref());
        Self { data, id }
    }
}

impl Identifiable for Ability {
    fn id(&self) -> &Id {
        &self.id
    }
}
