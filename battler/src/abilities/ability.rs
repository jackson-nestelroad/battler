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
    effect::fxlang,
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

    /// Dynamic battle effects.
    #[serde(default)]
    pub effect: fxlang::Effect,
}

/// An individual ability on a Mon that affects the battle in a wide variety of ways.
#[derive(Clone)]
pub struct Ability {
    id: Id,
    pub data: AbilityData,
}

impl Ability {
    /// Creates a new [`Ability`] instance from [`AbilityData`].
    pub fn new(id: Id, data: AbilityData) -> Self {
        Self { id, data }
    }
}

impl Identifiable for Ability {
    fn id(&self) -> &Id {
        &self.id
    }
}
