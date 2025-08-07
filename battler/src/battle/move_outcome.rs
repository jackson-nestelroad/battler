use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

/// The outcome of a move used on a single turn of battle.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, SerializeLabeledStringEnum, DeserializeLabeledStringEnum,
)]
pub enum MoveOutcome {
    /// The move was skipped. In other words, it neither succeeded or failed.
    #[string = "Skipped"]
    Skipped,
    /// THe move failed completely.
    #[string = "Failed"]
    Failed,
    /// The move succeeded. This can also mean partially succeeded.
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

    pub fn failed(&self) -> bool {
        match self {
            Self::Failed => true,
            _ => false,
        }
    }
}

impl From<bool> for MoveOutcome {
    fn from(value: bool) -> Self {
        if value { Self::Success } else { Self::Failed }
    }
}

impl From<MoveEventResult> for MoveOutcome {
    fn from(value: MoveEventResult) -> Self {
        match value {
            MoveEventResult::Advance => Self::Success,
            MoveEventResult::Fail => Self::Failed,
            MoveEventResult::Stop => Self::Skipped,
        }
    }
}

/// The outcome of a move used on a single target in a single turn of battle.
///
/// Differs from [`MoveOutcome`] in that it roughly tracks the effect a move had on a single target,
/// rather than the outcome of the use of the move as a whole.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum MoveOutcomeOnTarget {
    /// It is unknown how the move affected the target.
    #[default]
    Unknown,
    /// The move failed to do anything to the target.
    Failure,
    /// The move hit a Substitute.
    HitSubstitute,
    /// The move successfully hit the target.
    Success,
    /// The move successfully dealt damage to the target.
    Damage(u16),
}

impl MoveOutcomeOnTarget {
    /// Did the move hit anything (including a Substitute)?
    pub fn hit(&self) -> bool {
        match self {
            Self::Failure => false,
            _ => true,
        }
    }

    /// Did the move hit the target as intended?
    pub fn hit_target(&self) -> bool {
        match self {
            Self::Failure | Self::HitSubstitute => false,
            _ => true,
        }
    }

    /// Did the move fail?
    pub fn failed(&self) -> bool {
        match self {
            Self::Failure => true,
            _ => false,
        }
    }

    /// How much damage the move dealt to the target.
    pub fn damage(&self) -> u16 {
        match self {
            Self::Damage(damage) => *damage,
            _ => 0,
        }
    }

    /// Combines two move outcomes into one.
    ///
    /// Important when moves do multiple things and we must determine the outcome on the target as a
    /// whole.
    pub fn combine(&self, other: Self) -> Self {
        match (*self, other) {
            (Self::Unknown, right @ _) => right,
            (Self::Failure, Self::Unknown) => Self::Failure,
            (Self::Failure, right @ _) => right,
            (Self::HitSubstitute, Self::Unknown) => Self::HitSubstitute,
            (Self::HitSubstitute, right @ _) => right,
            (Self::Success, Self::Damage(right)) => Self::Damage(right),
            (Self::Success, _) => Self::Success,
            (Self::Damage(left), Self::Damage(right)) => Self::Damage(left + right),
            (left @ Self::Damage(_), _) => left,
        }
    }
}

impl From<bool> for MoveOutcomeOnTarget {
    fn from(value: bool) -> Self {
        if value { Self::Success } else { Self::Failure }
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
    /// Keep executing the move?
    pub fn advance(&self) -> bool {
        match self {
            Self::Advance => true,
            _ => false,
        }
    }

    /// Fail the move immediately?
    pub fn failed(&self) -> bool {
        match self {
            Self::Fail => true,
            _ => false,
        }
    }

    /// Combines two results into one.
    pub fn combine(&self, other: Self) -> Self {
        match (*self, other) {
            (Self::Fail, _) => Self::Fail,
            (Self::Stop, _) => Self::Stop,
            (Self::Advance, Self::Advance) => Self::Advance,
            (Self::Advance, right @ _) => right,
        }
    }
}

impl From<bool> for MoveEventResult {
    fn from(value: bool) -> Self {
        if value { Self::Advance } else { Self::Fail }
    }
}
