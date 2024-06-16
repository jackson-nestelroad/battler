use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

/// The outcome of a move used on a single turn of battle.
#[derive(Clone, Copy, PartialEq, Eq, SerializeLabeledStringEnum, DeserializeLabeledStringEnum)]
pub enum MoveOutcome {
    #[string = "Skipped"]
    Skipped,
    #[string = "Failed"]
    Failed,
    #[string = "Success"]
    Success,
}

impl MoveOutcome {
    pub fn success(&self) -> bool {
        match self {
            Self::Success => true,
            _ => false,
        }
    }
}

impl From<MoveOutcome> for bool {
    fn from(value: MoveOutcome) -> Self {
        value.success()
    }
}

/// The outcome of a move used on a single target in a single turn of battle.
///
/// Differs from [`MoveOutcome`] in that it roughly tracks the effect a move had on a single target,
/// rather than the outcome of the use of the move as a whole.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum MoveOutcomeOnTarget {
    #[default]
    Failure,
    Success,
    Damage(u16),
}

impl MoveOutcomeOnTarget {
    pub fn hit(&self) -> bool {
        !self.failed()
    }

    pub fn failed(&self) -> bool {
        match self {
            Self::Failure => true,
            Self::Success => false,
            Self::Damage(_) => false,
        }
    }

    pub fn damage(&self) -> u16 {
        match self {
            Self::Failure => 0,
            Self::Success => 0,
            Self::Damage(damage) => *damage,
        }
    }

    pub fn combine(&self, other: Self) -> Self {
        match (*self, other) {
            (Self::Failure, Self::Failure) => Self::Failure,
            (Self::Failure, right @ _) => right,
            (Self::Success, Self::Failure | Self::Success) => Self::Success,
            (Self::Success, right @ _) => right,
            (left @ Self::Damage(_), Self::Failure | Self::Success) => left,
            (Self::Damage(left), Self::Damage(right)) => Self::Damage(left + right),
        }
    }
}

/// The result of a move event, which indicates how the rest of the move should be handled.
#[derive(Clone, Copy, PartialEq, Eq, SerializeLabeledStringEnum, DeserializeLabeledStringEnum)]
pub enum MoveEventResult {
    /// Fail the move immediately.
    #[string = "fail"]
    Fail,
    /// Stop the move, but the move did not necessarily fail.
    #[string = "stop"]
    Stop,
    /// Continue the move.
    #[string = "continue"]
    Advance,
}

impl MoveEventResult {
    pub fn advance(&self) -> bool {
        match self {
            Self::Advance => true,
            _ => false,
        }
    }

    pub fn failed(&self) -> bool {
        match self {
            Self::Fail => true,
            _ => false,
        }
    }
}

impl From<bool> for MoveEventResult {
    fn from(value: bool) -> Self {
        if value {
            Self::Advance
        } else {
            Self::Fail
        }
    }
}
