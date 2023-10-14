use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    battle::BattleType,
    common::Error,
    config::{
        RuleSet,
        SerializedRuleSet,
    },
    dex::Dex,
};

/// Data for the format of a battle, which describes how a battle is configured.
#[derive(Debug, Serialize, Deserialize)]
pub struct FormatData {
    /// The type of battle that will take place.
    pub battle_type: BattleType,
    /// The rules in place that must be validated before and during the battle.
    pub rules: SerializedRuleSet,
}

/// The format of a battle, which describes how a battle is configured.
#[derive(Clone)]
pub struct Format {
    /// The type of battle that is taking place.
    pub battle_type: BattleType,
    /// The rules in place that must be validated before and during the battle.
    pub rules: RuleSet,
}

impl Format {
    /// Creates a new [`Format`] from [`FormatData`].
    pub fn new(data: FormatData, dex: &Dex) -> Result<Self, Error> {
        let rules = RuleSet::new(data.rules, &data.battle_type, dex)?;
        Ok(Self {
            battle_type: data.battle_type,
            rules,
        })
    }

    /// Constructs the [`FormatData`] for the [`Format`].
    pub fn data(&self) -> FormatData {
        FormatData {
            battle_type: self.battle_type.clone(),
            rules: self.rules.serialized(),
        }
    }
}
