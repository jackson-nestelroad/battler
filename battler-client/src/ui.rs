use ahash::{
    HashMap,
    HashSet,
};
use serde::{
    Deserialize,
    Serialize,
};

/// A position on the field.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FieldPosition {
    pub side: usize,
    pub position: usize,
}

/// A reference to a Mon that is likely not active on the field.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MonReference {
    pub player: String,
    pub name: String,
}

/// A Mon participating in the battle.
///
/// The Mon may be active or inactive. Active Mons can be seen on the field; inactive Mons can only
/// be referred to by name.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mon {
    Active(FieldPosition),
    Inactive(MonReference),
}

/// The target of a move.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveTarget {
    #[serde(untagged)]
    Single(Mon),
    #[serde(untagged)]
    Spread(HashSet<Mon>),
}

/// A generic effect.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Effect {
    pub effect_type: Option<String>,
    pub name: String,
}

/// Data for an activated effect in a battle.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectData {
    /// The effect that activated.
    pub effect: Option<Effect>,
    /// The side targeted by the effect.
    pub side: Option<usize>,
    /// The slot targeted by the effect.
    pub slot: Option<usize>,
    /// The player targeted by the effect.
    pub player: Option<String>,
    /// The Mon targeted by the effect.
    pub target: Option<Mon>,
    /// The Mon that triggered the effect.
    pub source: Option<Mon>,
    /// The effect that triggered the effect.
    pub source_effect: Option<Effect>,
    /// Any additional data from the battle log.
    pub additional: HashMap<String, String>,
}

/// A battle log entry specifically for the battle UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiLogEntry {
    /// A Mon was caught by a player.
    Caught { effect: EffectData },
    /// A Mon received damage.
    Damage {
        health: (u64, u64),
        effect: EffectData,
    },
    /// A debug log that should be shown to clients.
    Debug { values: HashMap<String, String> },
    /// A generic effect activated.
    Effect { title: String, effect: EffectData },
    /// A Mon received experience.
    Experience {},
    /// A Mon fainted.
    Faint { effect: EffectData },
    /// A Mon healed damage.
    Heal {
        health: (u64, u64),
        effect: EffectData,
    },
    /// A Mon left the field, due to the player leaving.
    Leave { mon: Mon },
    /// A Mon leveled up.
    LevelUp {
        mon: Mon,
        level: u64,
        stats: HashMap<String, u64>,
    },
    /// A Mon used a move.
    Move {
        name: String,
        mon: Mon,
        target: Option<MoveTarget>,
        animate: bool,
    },
    /// A Mon learned a move.
    MoveUpdate {
        mon: Mon,
        move_name: String,
        learned: bool,
        forgot: String,
    },
    /// A message was generated for an individual player.
    PlayerMessage { title: String, player: String },
    /// A Mon's stat received a boost (or drop).
    StatBoost { mon: Mon, stat: String, by: i64 },
    /// A Mon switched in.
    Switch {
        switch_type: String,
        player: String,
        mon: usize,
        into_position: FieldPosition,
    },
    /// A Mon's appearance updated.
    UpdateAppearance { effect: EffectData },
}
