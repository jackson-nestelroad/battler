use battler_data::{
    AbilityData,
    Id,
    Identifiable,
};

use crate::effect::fxlang;

/// An individual ability on a Mon that affects the battle in a wide variety of ways.
#[derive(Clone)]
pub struct Ability {
    id: Id,
    pub data: AbilityData,
    pub effect: fxlang::Effect,
    pub condition: fxlang::Effect,
}

impl Ability {
    /// Creates a new [`Ability`] instance from [`AbilityData`].
    pub fn new(id: Id, data: AbilityData) -> Self {
        let effect = data.effect.clone().try_into().unwrap_or_default();
        let condition = data.condition.clone().try_into().unwrap_or_default();
        Self {
            id,
            data,
            effect,
            condition,
        }
    }
}

impl Identifiable for Ability {
    fn id(&self) -> &Id {
        &self.id
    }
}
