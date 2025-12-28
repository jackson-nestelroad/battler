use std::time::{
    Duration,
    SystemTime,
};

use ahash::{
    HashMap,
    HashSet,
};
use battler::CoreBattleOptions;
use battler_service::BattleServiceOptions;
use serde::{
    Deserialize,
    Serialize,
};
use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};
use uuid::Uuid;

/// An AI player using random choices.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RandomOptions {}

/// An AI player using Gemini.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeminiOptions {}

/// The type of an AI player.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum AiPlayerType {
    #[serde(rename = "random")]
    Random(RandomOptions),
    #[serde(rename = "gemini")]
    Gemini(GeminiOptions),
}

/// Options for an AI player.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiPlayerOptions {
    /// AI type.
    pub ai_type: AiPlayerType,
    /// Player IDs.
    pub players: HashSet<String>,
}

/// A set of AI players.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AiPlayers {
    /// Players by ID.
    pub players: HashMap<String, AiPlayerOptions>,
}

/// Options for a proposed battle.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProposedBattleOptions {
    /// Battle options.
    pub battle_options: CoreBattleOptions,
    /// Service options.
    pub service_options: BattleServiceOptions,
    /// Timeout, after which the proposed battle will be deleted.
    pub timeout: Duration,
}

/// The status of a player with respect to a proposed battle.
#[derive(Debug, Clone, PartialEq, Eq, SerializeLabeledStringEnum, DeserializeLabeledStringEnum)]
pub enum PlayerStatus {
    #[string = "rejected"]
    Rejected,
    #[string = "accepted"]
    Accepted,
}

/// A player in a proposed battle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Player {
    /// Player ID.
    pub id: String,
    /// Player name.
    pub name: String,
    /// Status with respect to the proposed battle.
    pub status: Option<PlayerStatus>,
}

/// A side in a proposed battle.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Side {
    /// Side name.
    pub name: String,
    /// Players on the side.
    pub players: Vec<Player>,
}

/// A proposed battle, which has not yet started because all players have not accepted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedBattle {
    /// Unique identifier.
    pub uuid: Uuid,
    /// Sides of the battle.
    pub sides: Vec<Side>,
    /// Deadline in which the battle must start.
    pub deadline: SystemTime,
    /// The underlying battle, set only if the battle was fully accepted and created.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub battle: Option<Uuid>,
}

/// A player's response to a proposed battle.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedBattleResponse {
    /// Battle accepted?
    pub accept: bool,
}

/// A rejection of a proposed battle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedBattleRejection {
    /// Players who rejected.
    pub players: Vec<String>,
}

/// An update to a proposed battle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProposedBattleUpdate {
    /// The proposed battle.
    pub proposed_battle: ProposedBattle,
    /// The rejection, set only if the battle was rejected and deleted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection: Option<ProposedBattleRejection>,
    /// The reason the proposed battle was deleted, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deletion_reason: Option<String>,
}
