use anyhow::Result;
use battler_data::{
    Rule,
    SerializedRuleSet,
};
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    battle::BattleType,
    config::RuleSet,
    dex::Dex,
};

/// Data for the format of a battle, which describes how a battle is configured.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct FormatData {
    /// The type of battle that will take place.
    pub battle_type: BattleType,
    /// The rules in place that must be validated before and during the battle.
    pub rules: Vec<String>,
}

/// The format of a battle, which describes how a battle is configured.
#[derive(Clone)]
pub struct Format {
    /// The type of battle that is taking place.
    pub battle_type: BattleType,
    /// The rules in place that must be validated before and during the battle.
    pub rules: RuleSet,
    /// The original human-readable rule names.
    pub original_rules: Vec<String>,
}

impl Format {
    /// Creates a new format.
    pub fn new(data: FormatData, dex: &Dex) -> Result<Self> {
        let serialized_rules = data
            .rules
            .iter()
            .map(|s| s.parse::<Rule>())
            .collect::<Result<SerializedRuleSet, _>>()?;
        let rules = RuleSet::new(serialized_rules, &data.battle_type, dex)?;
        Ok(Self {
            battle_type: data.battle_type,
            rules,
            original_rules: data.rules,
        })
    }

    /// Constructs the [`FormatData`] for the [`Format`].
    pub fn data(&self) -> FormatData {
        FormatData {
            battle_type: self.battle_type.clone(),
            rules: self.original_rules.clone(),
        }
    }

    /// Returns the rules of the format as strings.
    pub fn rules(&self) -> Vec<String> {
        self.original_rules.clone()
    }
}
