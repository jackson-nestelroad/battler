use serde::{
    Deserialize,
    Serialize,
};

/// Options for the battle timer.
///
/// Each number is in seconds.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct TimerOptions {
    /// The amount of time a player has over the whole battle.
    #[serde(default)]
    pub player_time: u32,
    /// The amount of grace time the player is allowed at the start of the battle, typically for
    /// team preview.
    #[serde(default)]
    pub grace: u32,
    /// The amount of time a player gets per turn.
    #[serde(default)]
    pub time_per_turn: u32,
    /// The amount of time a player gets for their first turn.
    #[serde(default)]
    pub time_per_first_turn: u32,
    /// Whether to automatically choose the player's next move when they timeout, as opposed to
    /// forfeiting them.
    #[serde(default)]
    pub timeout_auto_choose: bool,
}
