use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    common::{
        FastHashSet,
        Id,
        Identifiable,
    },
    effect::fxlang,
    items::{
        ItemFlags,
        ItemInput,
        ItemTarget,
    },
    mons::Type,
};

/// Data for what happens when Fling is used with this item.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlingItemData {
    pub power: u32,
    pub status: Option<String>,
    pub volatile_status: Option<String>,
}

/// Data for special item effects; namely, item data associated with inidividual moves.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct SpecialItemData {
    /// The type this item turns Techno Blast into.
    pub techno_blast_type: Option<Type>,
    /// The type this item turns Multi-Attack into.
    pub multi_attack_type: Option<Type>,
}

/// Data about a particular item.
///
/// Items can be held by a Mon in battle to produce various side effects. Items can affect stat
/// calculations, move calculations, move choice restrictions, and much more.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemData {
    /// Name of the item.
    pub name: String,
    /// Target of the item if used by a player.
    pub target: Option<ItemTarget>,
    /// Type of input received alongside the item.
    pub input: Option<ItemInput>,
    /// Data for what happens when Fling is used with this item.
    pub fling: Option<FlingItemData>,
    /// Data for special item effects associated with individual moves.
    #[serde(default)]
    pub special_data: SpecialItemData,
    /// The type of this item if it is a plate.
    pub plate_type: Option<Type>,
    /// The species that this item allows Mega Evolution from.
    pub mega_evoles_from: Option<String>,
    /// The species that this item allows Mega Evolution into.
    pub mega_evolves_into: Option<String>,
    /// The forme this item forces a Mon into.
    pub force_forme: Option<String>,
    /// Item flags.
    pub flags: FastHashSet<ItemFlags>,

    /// Dynamic battle effects.
    #[serde(default)]
    pub effect: fxlang::Effect,
    /// Dynamic battle effects of the condition created by this item.
    #[serde(default)]
    pub condition: fxlang::Condition,
}

/// An individual item, which can be held by a Mon in battle.
#[derive(Debug, Clone)]
pub struct Item {
    id: Id,
    pub data: ItemData,
}

impl Item {
    /// Constructs a new item.
    pub fn new(id: Id, data: ItemData) -> Self {
        Self { id, data }
    }
}

impl Identifiable for Item {
    fn id(&self) -> &Id {
        &self.id
    }
}
