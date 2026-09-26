use battler_data::{
    Fraction,
    Id,
    Stat,
    Type,
};
use hashbrown::HashSet;
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    ability_manifest::{
        ContactPunishment,
        SurvivalType,
    },
    action::SemanticAction,
    modifier::DamageModifier,
    polarity::EffectPolarity,
    target_role::TargetRole,
};

/// Type of move restriction imposed by a held item.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveLockType {
    /// Locks user into first selected move and multiplies stat (e.g., Choice Band, Choice Specs, Choice Scarf).
    Choice(Stat, Fraction<u32>),
    /// Disallows all status moves (e.g., Assault Vest).
    StatusMoveLock,
}

/// Action performed by a trainer bag item used from the bag in battle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BagItemAction {
    /// Eligible target role (e.g., User or Ally).
    pub target: TargetRole,
    /// Action executed on the recipient.
    pub action: SemanticAction,
    /// Semantic polarity of the item action.
    pub polarity: EffectPolarity,
}

/// Behavioral and defensive flags for held items.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ItemFlag {
    /// Grants immunity to entry hazards (e.g., Heavy-Duty Boots).
    IgnoresHazards,
    /// Suppresses weather effects on the holder (e.g., Utility Umbrella).
    WeatherSuppressedForHolder,
}

/// Precompiled semantic manifest for a held or bag item.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ItemManifest {
    /// Unique ID of the item.
    pub id: Id,
    /// Types this item grants immunity to (e.g., Ground for Air Balloon).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub type_immunities: Vec<Type>,
    /// Survival behavior (e.g., Focus Sash).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub survival: Option<SurvivalType>,
    /// Punishment inflicted on contact attackers (e.g., Rocky Helmet 1/6 max HP recoil).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_punishment: Option<ContactPunishment>,
    /// Move lock restriction (e.g., Choice Band, Assault Vest).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub move_lock: Option<MoveLockType>,
    /// Damage modifiers granted by this item (e.g., Life Orb, Charcoal, Eviolite, Resist Berries).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub damage_modifiers: Vec<DamageModifier>,
    /// In-battle trainer bag item behavior, if usable from the bag.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bag_item: Option<BagItemAction>,
    /// Special behavioral flags for this item.
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub flags: HashSet<ItemFlag>,
}
