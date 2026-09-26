use battler_data::{
    BoostTable,
    Fraction,
    MoveCategory,
};
use serde::{
    Deserialize,
    Serialize,
};

fn default_max_stacks() -> u32 {
    1
}

/// A typed action produced by an effect (damage, stat boost, status, heal, condition, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SemanticAction {
    /// Deals damage to the recipient.
    Damage {
        category: MoveCategory,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        base_power: Option<u32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        recoil_percent: Option<Fraction<u32>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        drain_percent: Option<Fraction<u32>>,
    },
    /// Modifies stat stages (e.g., Swords Dance, Screech).
    StatChange {
        boosts: BoostTable,
    },
    /// Inflicts a non-volatile status (e.g., sleep, burn, poison, paralysis).
    StatusInfliction {
        status: String,
        chance: Fraction<u32>,
    },
    /// Cures non-volatile status effects.
    StatusCure {
        /// Specific statuses cured, or empty for all non-volatile statuses (e.g., Aromatherapy).
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        statuses: Vec<String>,
    },
    /// Applies a volatile condition to a Mon (e.g., Taunt, Substitute, Confusion).
    ApplyCondition {
        condition_id: String,
        is_volatile: bool,
        #[serde(default = "default_max_stacks")]
        max_stacks: u32,
    },
    /// Applies a side condition (e.g., Stealth Rock, Spikes, Tailwind, Reflect).
    ApplySideCondition {
        condition_id: String,
        #[serde(default = "default_max_stacks")]
        max_stacks: u32,
    },
    /// Sets a field condition (e.g., Trick Room, Electric Terrain, Rain).
    SetFieldCondition {
        condition_id: String,
    },
    /// Restores hit points as a fraction of max HP.
    Heal {
        fraction: Fraction<u32>,
    },
    /// Revives a fainted Mon.
    Revive {
        fraction: Fraction<u32>,
    },
    /// Forces a switch.
    ForceSwitch {
        /// true = forces the target out (e.g., Roar, Whirlwind); false = user pivots out (e.g., U-turn, Volt Switch).
        target: bool,
    },
    /// Protects the recipient from attacks (e.g., Protect, Detect, Spiky Shield).
    Protection,
}
