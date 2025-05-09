use ahash::{
    HashMap,
    HashSet,
};
use serde::{
    Deserialize,
    Serialize,
};

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FieldPosition {
    pub side: usize,
    pub position: usize,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MonReference {
    pub player: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mon {
    Active(FieldPosition),
    Inactive(MonReference),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum MoveTarget {
    #[serde(untagged)]
    Single(Mon),
    #[serde(untagged)]
    Spread(HashSet<Mon>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Effect {
    pub effect_type: Option<String>,
    pub name: String,
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EffectData {
    pub effect: Option<Effect>,
    pub side: Option<usize>,
    pub slot: Option<usize>,
    pub player: Option<String>,
    pub target: Option<Mon>,
    pub source: Option<Mon>,
    pub source_effect: Option<Effect>,
    pub additional: HashMap<String, String>,
}

/// A battle log entry specifically for the battle UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiLogEntry {
    Caught {
        effect: EffectData,
    },
    Damage {
        health: (u64, u64),
        effect: EffectData,
    },
    Debug {
        values: HashMap<String, String>,
    },
    Effect {
        title: String,
        effect: EffectData,
    },
    Experience {},
    Faint {
        effect: EffectData,
    },
    Heal {
        health: (u64, u64),
        effect: EffectData,
    },
    Leave {
        mon: Mon,
    },
    LevelUp {
        mon: Mon,
        level: u64,
        stats: HashMap<String, u64>,
    },
    Move {
        name: String,
        mon: Mon,
        target: Option<MoveTarget>,
        animate: bool,
    },
    MoveUpdate {
        mon: Mon,
        move_name: String,
        learned: bool,
        forgot: String,
    },
    PlayerMessage {
        title: String,
        player: String,
    },
    StatBoost {
        mon: Mon,
        stat: String,
        by: i64,
    },
    Switch {
        switch_type: String,
        player: String,
        mon: usize,
        into_position: FieldPosition,
    },
    UpdateAppearance {
        effect: EffectData,
    },
}
