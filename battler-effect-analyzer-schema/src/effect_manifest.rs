use battler_data::{
    Id,
    MoveTarget,
};
use hashbrown::HashSet;
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    action::SemanticAction,
    modifier::{
        DamageModifier,
        EffectFlag,
        FixedDamage,
    },
    polarity::EffectPolarity,
    requirement::EffectRequirement,
};

/// Precompiled semantic manifest for a move or active effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectManifest {
    /// Unique ID of the effect.
    pub id: Id,
    /// Declared target scope of the move.
    pub target_scope: MoveTarget,
    /// Default polarity of the overall move.
    pub default_polarity: EffectPolarity,
    /// Specific typed actions performed by this effect and their individual polarities on the recipient.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<(EffectPolarity, SemanticAction)>,
    /// Viability requirements that must hold before this effect can be used.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub requirements: Vec<EffectRequirement>,
    /// Damage modifiers applied by this move.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub damage_modifiers: Vec<DamageModifier>,
    /// Fixed damage formula, if applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed_damage: Option<FixedDamage>,
    /// Engine simulation flags.
    #[serde(default, skip_serializing_if = "HashSet::is_empty")]
    pub flags: HashSet<EffectFlag>,
}
