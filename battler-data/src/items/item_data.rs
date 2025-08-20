use ahash::HashSet;
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    HitEffect,
    ItemFlag,
    ItemInput,
    ItemTarget,
    Type,
};

/// Data for what happens when "Fling" is used with this item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlingData {
    pub power: u32,
    #[serde(default)]
    pub use_item: bool,
    pub hit_effect: Option<HitEffect>,
}

/// Data for what happens when "Natural Gift" is used with this item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NaturalGiftData {
    pub power: u32,
    pub typ: Type,
}

/// Data for what happens when "Judgment" is used with this item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JudgmentData {
    pub typ: Type,
}

/// Data for what happens when "Techno Blast" is used with this item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TechnoBlastData {
    pub typ: Type,
}

/// Data for what happens when "Multi-Attack" is used with this item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultiAttackData {
    pub typ: Type,
}

/// Data for special item effects; namely, item data associated with individual moves.
#[derive(Debug, Default, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct SpecialItemData {
    pub fling: Option<FlingData>,
    pub natural_gift: Option<NaturalGiftData>,
    pub judgment: Option<JudgmentData>,
    pub techno_blast: Option<TechnoBlastData>,
    pub multi_attack: Option<MultiAttackData>,
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
    /// Data for special item effects associated with individual moves.
    #[serde(default)]
    pub special_data: SpecialItemData,
    /// The species that this item allows Mega Evolution from.
    pub mega_evolves_from: Option<String>,
    /// The species that this item allows Mega Evolution into.
    pub mega_evolves_into: Option<String>,
    /// The forme this item forces a Mon into.
    pub force_forme: Option<String>,
    /// Item flags.
    pub flags: HashSet<ItemFlag>,

    /// Dynamic battle effects.
    #[serde(default)]
    pub effect: serde_json::Value,
    /// Dynamic battle effects of the condition created by this item.
    #[serde(default)]
    pub condition: serde_json::Value,
}
