use serde::{
    Deserialize,
    Serialize,
};
use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};
use uuid::Uuid;

/// The state of a [`Player`] in a [`Battle`].
#[derive(Debug, Clone, PartialEq, Eq, SerializeLabeledStringEnum, DeserializeLabeledStringEnum)]
pub enum PlayerState {
    /// The player is not ready to start the battle.
    #[string = "waiting"]
    Waiting,
    /// The player is ready to start the battle.
    #[string = "ready"]
    Ready,
}

/// The result of a validating a [`Player`] in a [`Battle`].
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerValidation {
    /// Problems generated by validation.
    pub problems: Vec<String>,
}

/// A player in a [`Battle`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    /// The unique identifier of the player, used for all interactions made by the player.
    pub id: String,
    /// Name of the player.
    pub name: String,
    /// The state of the player.
    pub state: PlayerState,
}

/// A side in a [`Battle`].
///
/// A side is made up of one or more [`Player`]s.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Side {
    /// Name of the side.
    pub name: String,
    /// Players on the side.
    pub players: Vec<Player>,
}

/// The state of a [`Battle`].
#[derive(Debug, Clone, PartialEq, Eq, SerializeLabeledStringEnum, DeserializeLabeledStringEnum)]
pub enum BattleState {
    /// The battle is being prepared.
    #[string = "preparing"]
    Preparing,
    /// The battle is active and ongoing.
    #[string = "active"]
    Active,
    /// The battle ended.
    #[string = "finished"]
    Finished,
}

/// A battle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Battle {
    /// The unique identifier of the battle, used for all interactions made by players.
    #[serde(with = "uuid::serde::simple")]
    pub uuid: Uuid,
    /// The state of the battle.
    pub state: BattleState,
    /// The sides participating in the battle.
    pub sides: Vec<Side>,
    /// The error that occurred when continuing the battle.
    pub error: Option<String>,
}

/// A preview of a [`Player`] in a [`Battle`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PlayerPreview {
    /// The unique identifier of the player.
    pub id: String,
    /// Name of the player.
    pub name: String,
}

/// A preview of a [`Side`] in a [`Battle`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SidePreview {
    /// Players on the side.
    pub players: Vec<PlayerPreview>,
}

/// A preview of a [`Battle`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattlePreview {
    /// The unique identifier of the battle.
    #[serde(with = "uuid::serde::simple")]
    pub uuid: Uuid,
    /// The sides participating in the battle.
    pub sides: Vec<SidePreview>,
}
