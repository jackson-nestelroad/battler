use anyhow::Result;
use serde_string_enum::{
    DeserializeLabeledStringEnum,
    SerializeLabeledStringEnum,
};

use crate::battle::MoveOutcomeOnTarget;

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

    /// Evaluates the closure if this result advances, otherwise returns this result.
    pub fn and_then<F>(self, f: F) -> Self
    where
        F: FnOnce() -> Self,
    {
        if self.advance() { f() } else { self }
    }

    /// Evaluates the closure if this result advances, otherwise returns this result.
    ///
    /// Variant for closures that return a [`Result`].
    pub fn and_then_try<F>(self, f: F) -> Result<Self>
    where
        F: FnOnce() -> Result<Self>,
    {
        if self.advance() { f() } else { Ok(self) }
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

/// Returns the event result early if it does not advance.
#[macro_export]
macro_rules! try_event {
    ($expr:expr) => {{
        let res = $expr;
        if !res.advance() {
            return res;
        }
        res
    }};
    ($expr:expr, $wrapper:ident) => {{
        let res = $expr;
        if !res.advance() {
            return $wrapper(res);
        }
        res
    }};
    ($expr:expr, $res:ident => $ret_expr:expr) => {{
        let $res = $expr;
        if !$res.advance() {
            return $ret_expr;
        }
        $res
    }};
}
