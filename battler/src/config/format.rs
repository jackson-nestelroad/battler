use std::u8;

use anyhow::Result;
use serde::{
    Deserialize,
    Serialize,
};

use crate::{
    battle::BattleType,
    config::{
        RuleSet,
        SerializedRuleSet,
    },
    dex::Dex,
};

/// Customizable options for any format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatOptions {
    /// The number of steps away from a Mon that is counted as being adjacent to it for
    /// adjacent-targeting attacks.
    ///
    /// By default, moves that target adjacent Mons can reach any Mon two steps away from it.
    /// However, some battles (such as Horde Battles) require one Mon to be able to fight five Mons
    /// at once. This requires an `adjacency_reach` of 3, since the Mons on the edges will be three
    /// steps away from the center.
    ///
    /// For visualization, the following battle:
    ///
    /// ```ignore
    /// 5  4  3  2  1
    ///       1
    /// ```
    ///
    /// maps to the following adjacency counts, relative to the single Mon on the bottom side
    ///
    /// ```ignore
    /// 3  2  1  2  3
    ///       0
    /// ```
    pub adjacency_reach: u8,

    /// The maximum level that will obey its player if it originates from a different trainer.
    pub obedience_cap: u8,

    /// Whether or not players can use items from their bag.
    pub bag_items: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            adjacency_reach: 2,
            obedience_cap: u8::MAX,
            bag_items: false,
        }
    }
}

/// Data for the format of a battle, which describes how a battle is configured.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FormatData {
    /// The type of battle that will take place.
    pub battle_type: BattleType,
    /// The rules in place that must be validated before and during the battle.
    pub rules: SerializedRuleSet,
    /// Options for the format.
    #[serde(default)]
    pub options: FormatOptions,
}

/// The format of a battle, which describes how a battle is configured.
#[derive(Clone)]
pub struct Format {
    /// The type of battle that is taking place.
    pub battle_type: BattleType,
    /// The rules in place that must be validated before and during the battle.
    pub rules: RuleSet,
    /// Options for the format.
    pub options: FormatOptions,
}

impl Format {
    /// Creates a new format.
    pub fn new(data: FormatData, dex: &Dex) -> Result<Self> {
        let rules = RuleSet::new(data.rules, &data.battle_type, dex)?;
        Ok(Self {
            battle_type: data.battle_type,
            rules,
            options: data.options,
        })
    }

    /// Constructs the [`FormatData`] for the [`Format`].
    pub fn data(&self) -> FormatData {
        FormatData {
            battle_type: self.battle_type.clone(),
            rules: self.rules.serialized(),
            options: self.options.clone(),
        }
    }
}
