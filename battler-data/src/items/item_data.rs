use alloc::string::String;

use hashbrown::HashSet;
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
    #[serde(rename = "type")]
    pub typ: Type,
}

/// Data for what happens when "Judgment" is used with this item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JudgmentData {
    #[serde(rename = "type")]
    pub typ: Type,
}

/// Data for what happens when "Techno Blast" is used with this item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TechnoBlastData {
    #[serde(rename = "type")]
    pub typ: Type,
}

/// Data for what happens when "Multi-Attack" is used with this item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MultiAttackData {
    #[serde(rename = "type")]
    pub typ: Type,
}

/// Data for Mega Evolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MegaEvolutionData {
    pub from: String,
    pub into: String,
}

/// Source of a Z-Crystal's power.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZCrystalSource {
    #[serde(rename = "from")]
    Move(String),
    #[serde(rename = "type")]
    Type(Type),
}

/// Data for Z-Crystals.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZCrystalData {
    #[serde(flatten)]
    pub source: Option<ZCrystalSource>,
    pub into: String,
    #[serde(default)]
    pub users: HashSet<String>,
}

/// Data for special item effects; namely, item data associated with moves and effects.
#[derive(Debug, Default, PartialEq, Eq, Clone, Serialize, Deserialize)]
pub struct SpecialItemData {
    pub mega_evolution: Option<MegaEvolutionData>,
    pub z_crystal: Option<ZCrystalData>,

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
    /// Data for special item effects associated with moves and effects.
    #[serde(default)]
    pub special_data: SpecialItemData,
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
