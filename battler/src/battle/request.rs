use alloc::vec::Vec;

use serde::{
    Deserialize,
    Serialize,
};
use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

use crate::battle::{
    MonLearnMoveRequest,
    MonMoveRequest,
    PlayerBattleData,
};

/// Type type of [`Request`] that should be requested from a player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub enum RequestType {
    /// A request for a team order to be chosen during team preview.
    TeamPreview,
    /// A request for the active Mon(s) to act at the beginning of a turn.
    Turn,
    /// A request for one or more Mons to be switched in.
    Switch,
    /// A request for one or more Mons to learn one or more moves.
    ///
    /// Only applicable for single player simulations.
    LearnMove,
    /// A request for one or more Mons to be selected.
    Select,
}

/// A request for a team to be chosen in Team Preview.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct TeamPreviewRequest {
    pub max_team_size: Option<usize>,
}

/// A request for a player to command their Mons for the next turn.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct TurnRequest {
    pub active: Vec<MonMoveRequest>,
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allies: Vec<PlayerBattleData>,
}

/// A request for a Mon to be switched in.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct SwitchRequest {
    /// Active positions that need to be switched out.
    pub needs_switch: Vec<usize>,
}

/// A request for a Mon to learn one or more moves.
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct LearnMoveRequest {
    pub can_learn_move: MonLearnMoveRequest,
}

/// The reason a Mon must be selected.
#[derive(Debug, Clone, PartialEq, Eq, SerializeLabeledStringEnum, DeserializeLabeledStringEnum)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub enum SelectReason {
    #[string = "Revive"]
    Revive,
}

/// A position that triggered a Mon to be selected.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct SelectPosition {
    pub position: usize,
    pub reason: SelectReason,
}

/// A request for one or more Mons to be selected.
///
/// Conceptually similar to a [`SwitchRequest`], except something else happens with the selected
/// Mons (indicated by [`SelectReason`]).
#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub struct SelectRequest {
    /// Active positions that you must select a Mon for.
    pub positions: Vec<SelectPosition>,
}

/// A request for an action that a [`Player`][`crate::battle::Player`] must make before the battle
/// can continue.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type")]
#[cfg_attr(feature = "typescript", derive(ts_rs::TS))]
#[cfg_attr(feature = "typescript", ts(export))]
pub enum Request {
    /// A request for a team order to be chosen during team preview.
    #[serde(rename = "team")]
    TeamPreview(TeamPreviewRequest),
    /// A request for the active Mon(s) to act at the beginning of a turn.
    #[serde(rename = "turn")]
    Turn(TurnRequest),
    /// A request for one or more Mons to be switched in.
    #[serde(rename = "switch")]
    Switch(SwitchRequest),
    /// A request for a single Mon to learn a move.
    #[serde(rename = "learnmove")]
    LearnMove(LearnMoveRequest),
    /// A request to select one or more Mons.
    #[serde(rename = "select")]
    Select(SelectRequest),
}

impl Request {
    /// The type of the request.
    pub fn request_type(&self) -> RequestType {
        match self {
            Self::TeamPreview(_) => RequestType::TeamPreview,
            Self::Turn(_) => RequestType::Turn,
            Self::Switch(_) => RequestType::Switch,
            Self::LearnMove(_) => RequestType::LearnMove,
            Self::Select(_) => RequestType::Select,
        }
    }
}
