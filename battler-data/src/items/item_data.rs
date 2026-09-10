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
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct FlingData {
    pub power: u32,
    #[serde(default)]
    pub use_item: bool,
    pub hit_effect: Option<HitEffect>,
}

/// Data for what happens when "Natural Gift" is used with this item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct NaturalGiftData {
    pub power: u32,
    #[serde(rename = "type")]
    pub typ: Type,
}

/// Data for what happens when "Judgment" is used with this item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct JudgmentData {
    #[serde(rename = "type")]
    pub typ: Type,
}

/// Data for what happens when "Techno Blast" is used with this item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct TechnoBlastData {
    #[serde(rename = "type")]
    pub typ: Type,
}

/// Data for what happens when "Multi-Attack" is used with this item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct MultiAttackData {
    #[serde(rename = "type")]
    pub typ: Type,
}

/// Data for Mega Evolution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct MegaEvolutionData {
    pub from: String,
    pub into: String,
}

/// Source of a Z-Crystal's power.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub enum ZCrystalSource {
    #[serde(rename = "from")]
    Move(String),
    #[serde(rename = "type")]
    Type(Type),
}

/// Data for Ultra Burst.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct UltraBurstData {
    #[cfg_attr(feature = "typescript", ts(as = "Vec<String>"))]
    pub from: HashSet<String>,
    pub into: String,
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

#[cfg(feature = "typescript")]
impl ts_rs::TS for ZCrystalData {
    type WithoutGenerics = Self;

    fn decl() -> alloc::string::String {
        alloc::borrow::ToOwned::to_owned(
            "type ZCrystalData = {\n    into: string;\n    users?: Array<string>;\n    from?: string;\n    type?: Type;\n};",
        )
    }

    fn decl_concrete() -> alloc::string::String {
        Self::decl()
    }

    fn name() -> alloc::string::String {
        alloc::borrow::ToOwned::to_owned("ZCrystalData")
    }

    fn inline() -> alloc::string::String {
        alloc::borrow::ToOwned::to_owned(
            "{\n    into: string;\n    users?: Array<string>;\n    from?: string;\n    type?: Type;\n}",
        )
    }

    fn inline_flattened() -> alloc::string::String {
        alloc::borrow::ToOwned::to_owned(
            "into: string;\n    users?: Array<string>;\n    from?: string;\n    type?: Type;",
        )
    }

    fn output_path() -> Option<&'static std::path::Path> {
        Some(std::path::Path::new("ZCrystalData.ts"))
    }

    fn visit_dependencies(visitor: &mut impl ts_rs::TypeVisitor)
    where
        Self: 'static,
    {
        visitor.visit::<crate::mons::Type>();
    }
}

/// Data for special item effects; namely, item data associated with moves and effects.
#[derive(Debug, Default, PartialEq, Eq, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct SpecialItemData {
    pub mega_evolution: Option<MegaEvolutionData>,
    pub z_crystal: Option<ZCrystalData>,
    pub ultra_burst: Option<UltraBurstData>,

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
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
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
    #[cfg_attr(feature = "typescript", ts(as = "Vec<ItemFlag>"))]
    pub flags: HashSet<ItemFlag>,

    /// Dynamic battle effects.
    #[serde(default)]
    #[cfg_attr(feature = "typescript", ts(type = "unknown"))]
    pub effect: serde_json::Value,
    /// Dynamic battle effects of the condition created by this item.
    #[serde(default)]
    #[cfg_attr(feature = "typescript", ts(type = "unknown"))]
    pub condition: serde_json::Value,
}
