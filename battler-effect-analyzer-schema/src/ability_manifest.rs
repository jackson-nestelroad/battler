use battler_data::{
    Fraction,
    Id,
    MoveFlag,
    Stat,
    Type,
};
use hashbrown::HashSet;
use serde::{
    Deserialize,
    Serialize,
};

use crate::modifier::DamageModifier;

/// Survival mechanisms that prevent OHKOs or lethal hits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SurvivalType {
    /// Immune to one-hit knockout moves (e.g., Sturdy against Fissure).
    OhkoImmunity,
    /// Survives lethal hits with 1 HP when at 100% full health (e.g., Sturdy, Focus Sash).
    LethalDamageCapAtFullHp,
    /// Disguise absorbs the first damaging hit.
    Disguise,
}

/// Contact punishments triggered when an attacker makes contact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContactPunishment {
    /// Deals a fraction of max HP damage back to the attacker (e.g., Rough Skin 1/8, Rocky Helmet 1/6).
    DamageFraction(Fraction<u32>),
    /// Has a chance to inflict a status condition (e.g., Flame Body 30% burn, Static 30% paralysis).
    Status(String, Fraction<u32>),
}

/// Action performed upon absorbing an attack of a specific type.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealOrBoost {
    /// Heals the recipient by the given fraction of max HP (e.g., Volt Absorb, Water Absorb 1/4).
    Heal(Fraction<u32>),
    /// Boosts the recipient's stat stage (e.g., Lightning Rod SpAtk +1, Sap Sipper Atk +1).
    Boost(Stat, i8),
}

/// Behavioral and defensive flags for abilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AbilityFlag {
    /// Wonder Guard immunity (only super-effective damaging moves hit).
    WonderGuard,
    /// Traps opposing Mons from switching out (e.g., Shadow Tag, Arena Trap).
    Trapping,
    /// Suppresses field weather while bearer is active (e.g., Air Lock, Cloud Nine).
    SuppressesWeather,
    /// Ignores entry hazards (e.g., Magic Guard).
    IgnoresHazards,
}

/// Precompiled semantic manifest for an ability.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AbilityManifest {
    /// Unique ID of the ability.
    pub id: Id,
    /// Types this ability grants immunity to (e.g., Ground for Levitate).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub type_immunities: Vec<Type>,
    /// Types absorbed for healing or stat boosts (e.g., Volt Absorb, Lightning Rod).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub absorption: Vec<(Type, HealOrBoost)>,
    /// Non-volatile statuses this ability grants immunity to (e.g., Limber -> "par", Insomnia -> "slp").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub status_immunities: Vec<String>,
    /// Move flags this ability grants immunity to (e.g., MoveFlag::Sound for Soundproof).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub move_flag_immunities: Vec<MoveFlag>,
    /// Punishment inflicted on contact attackers.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub contact_punishment: Option<ContactPunishment>,
    /// Survival behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub survival: Option<SurvivalType>,
    /// Damage modifiers granted by this ability (e.g., Technician, Huge Power, Multiscale).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub damage_modifiers: Vec<DamageModifier>,
    /// Special behavioral flags for this ability.
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub flags: HashSet<AbilityFlag>,
}
