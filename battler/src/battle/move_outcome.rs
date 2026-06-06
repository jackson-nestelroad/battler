use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

/// The outcome of a move used on a single turn of battle.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, SerializeLabeledStringEnum, DeserializeLabeledStringEnum,
)]
pub enum MoveOutcome {
    /// The move was skipped. In other words, it neither succeeded nor failed.
    #[string = "Skipped"]
    Skipped,
    /// THe move failed completely.
    #[string = "Failed"]
    Failed,
    /// The move succeeded. This can also mean partially succeeded.
    #[string = "Succeeded"]
    Succeeded,
}

impl MoveOutcome {
    pub fn succeeded(&self) -> bool {
        match self {
            Self::Succeeded => true,
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
        if value { Self::Succeeded } else { Self::Failed }
    }
}

impl From<MoveOutcomeOnTarget> for MoveOutcome {
    fn from(value: MoveOutcomeOnTarget) -> Self {
        if value.success() {
            MoveOutcome::Succeeded
        } else if value.failed() {
            MoveOutcome::Failed
        } else {
            MoveOutcome::Skipped
        }
    }
}

impl From<EventResult> for MoveOutcome {
    fn from(value: EventResult) -> Self {
        match value {
            EventResult::Fail | EventResult::StopFail => Self::Failed,
            EventResult::StopReportFail | EventResult::Stop | EventResult::Skip => Self::Skipped,
            EventResult::Advance => Self::Succeeded,
        }
    }
}

/// The outcome of a move used on a single target in a single turn of battle.
///
/// Differs from [`MoveOutcome`] in that it roughly tracks the effect a move had on a single target,
/// rather than the outcome of the use of the move as a whole.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MoveOutcomeOnTarget {
    Unknown,
    EventResult(EventResult),
    HitSubstitute,
    Damage(u16),
}

impl MoveOutcomeOnTarget {
    /// Did the move succeed?
    fn success(&self) -> bool {
        match self {
            Self::Unknown | Self::Damage(_) | Self::HitSubstitute => true,
            Self::EventResult(result) => result.advance(),
        }
    }

    /// Did the move hit anything (including a Substitute)?
    pub fn hit(&self) -> bool {
        self.success()
    }

    /// Did the move hit the target as intended?
    pub fn advance(&self) -> bool {
        self.success() && *self != Self::HitSubstitute
    }

    /// Did the move fail?
    pub fn failed(&self) -> bool {
        match self {
            Self::EventResult(result) => result.failed(),
            _ => false,
        }
    }

    /// Should the move report a failure?
    pub fn report_failure(&self) -> bool {
        match self {
            Self::EventResult(result) => result.report_failure(),
            _ => false,
        }
    }

    /// Should the move not animate?
    pub fn do_not_animate(&self) -> bool {
        match self {
            Self::EventResult(result) => result.do_not_animate(),
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
            (Self::Unknown, right) => right,
            (left @ _, Self::Unknown) => left,
            (Self::HitSubstitute, right) => right,
            (Self::EventResult(a), Self::EventResult(b)) => Self::EventResult(a.combine(b)),
            (Self::Damage(left), Self::Damage(right)) => Self::Damage(left + right),
            (left @ Self::Damage(_), _) => left,
            (_, right @ Self::Damage(_)) => right,
            (left @ Self::EventResult(result), _) if result.advance() => left,
            (Self::EventResult(_), right) => right,
        }
    }
}

impl Default for MoveOutcomeOnTarget {
    fn default() -> Self {
        Self::EventResult(EventResult::default())
    }
}

impl From<bool> for MoveOutcomeOnTarget {
    fn from(value: bool) -> Self {
        Self::EventResult(EventResult::from(value))
    }
}

impl From<EventResult> for MoveOutcomeOnTarget {
    fn from(value: EventResult) -> Self {
        Self::EventResult(value)
    }
}

/// The result of a move event, which indicates how the rest of the move should be handled.
#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    SerializeLabeledStringEnum,
    DeserializeLabeledStringEnum,
)]
pub enum EventResult {
    /// Fail the move immediately.
    ///
    /// Do not animate. Report failure. Treat as failure.
    #[string = "fail"]
    Fail,
    /// Stop the move.
    ///
    /// Do not animate. Do not report failure. Treat as failure.
    #[string = "stopfail"]
    StopFail,
    /// Stop the move.
    ///
    /// Do not animate. Report failure. Treat as skipped.
    #[string = "StopReportFail"]
    StopReportFail,
    /// Stop the move.
    ///
    /// Do not animate. Do not report failure. Treat as skipped.
    #[string = "Stop"]
    Stop,
    /// Skip the move.
    ///
    /// Animate. Do not report failure. Treat as skipped.
    #[string = "skip"]
    Skip,
    /// Continue the move.
    ///
    /// Animate. Do not report failure. Treat as success.
    #[string = "continue"]
    #[default]
    Advance,
}

impl EventResult {
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
            Self::Fail | Self::StopFail => true,
            _ => false,
        }
    }

    /// Should the move report a failure?
    pub fn report_failure(&self) -> bool {
        match self {
            Self::Fail | Self::StopReportFail => true,
            _ => false,
        }
    }

    /// Should the move not animate?
    pub fn do_not_animate(&self) -> bool {
        match self {
            Self::Fail | Self::StopFail | Self::StopReportFail | Self::Stop => true,
            _ => false,
        }
    }

    /// Combines two results into one.
    pub fn combine(&self, other: Self) -> Self {
        match (*self, other) {
            (Self::Advance, _) => Self::Advance,
            (_, Self::Advance) => Self::Advance,
            (Self::Fail, _) => Self::Fail,
            (Self::StopFail | Self::StopReportFail | Self::Stop | Self::Skip, right @ _) => right,
        }
    }
}

impl From<bool> for EventResult {
    fn from(value: bool) -> Self {
        if value { Self::Advance } else { Self::Fail }
    }
}

impl From<MoveOutcomeOnTarget> for EventResult {
    fn from(value: MoveOutcomeOnTarget) -> Self {
        match value {
            MoveOutcomeOnTarget::EventResult(result) => result,
            _ => EventResult::Advance,
        }
    }
}
