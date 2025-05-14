use battler_data::{
    Id,
    Identifiable,
    ItemData,
};

use crate::effect::fxlang;

/// An individual item, which can be held by a Mon in battle.
#[derive(Debug, Clone)]
pub struct Item {
    id: Id,
    pub data: ItemData,
    pub effect: fxlang::Effect,
    pub condition: fxlang::Condition,
}

impl Item {
    /// Constructs a new item.
    pub fn new(id: Id, data: ItemData) -> Self {
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

impl Identifiable for Item {
    fn id(&self) -> &Id {
        &self.id
    }
}
