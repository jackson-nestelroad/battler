use anyhow::Result;
use async_trait::async_trait;
use battler_wamp::{
    core::{
        error::WampError,
        invocation_policy::InvocationPolicy,
    },
    peer::Invocation,
};

/// Options for registering a procedure.
#[derive(Debug, Default)]
pub struct ProcedureOptions {
    pub invocation_policy: InvocationPolicy,
}

/// A procedure that responds to any given invocation with some result.
#[async_trait]
pub trait Procedure: Send + Sync {
    /// Invokes the procedure.
    ///
    /// It is the method's responsibility to call [`Invocation::respond`] to send the result to the
    /// caller.
    async fn invoke(&self, invocation: Invocation) -> Result<()>;
}

/// A strongly-typed procedure that generates some output based on some input.
#[async_trait]
pub trait TypedProcedure: Send + Sync {
    /// Input from the caller.
    type Input: battler_wamprat_message::WampApplicationMessage;

    /// Output to the caller.
    type Output: battler_wamprat_message::WampApplicationMessage;

    /// Error to the caller.
    type Error: Into<WampError>;

    /// Invokes the procedure and produces a result.
    async fn invoke(&self, input: Self::Input) -> Result<Self::Output, Self::Error>;

    /// Options for the procedure.
    fn options() -> ProcedureOptions {
        ProcedureOptions::default()
    }
}

/// A strongly-typed, pattern-matched procedure that generates some output based on some input.
#[async_trait]
pub trait TypedPatternMatchedProcedure: Send + Sync {
    /// Pattern of the procedure.
    type Pattern: battler_wamprat_uri::WampUriMatcher;

    /// Input from the caller.
    type Input: battler_wamprat_message::WampApplicationMessage;

    /// Output to the caller.
    type Output: battler_wamprat_message::WampApplicationMessage;

    /// Error to the caller.
    type Error: Into<WampError>;

    /// Invokes the procedure and produces a result.
    async fn invoke(
        &self,
        input: Self::Input,
        procedure: Self::Pattern,
    ) -> Result<Self::Output, Self::Error>;

    /// Options for the procedure.
    fn options() -> ProcedureOptions {
        ProcedureOptions::default()
    }
}

/// Type for reporting progress of a procedure that produces progressive results.
pub type ProgressReporter<'a, T> = Box<dyn Fn(T) -> Result<()> + Send + 'a>;

/// A strongly-typed procedure that generates progressive output based on some input.
#[async_trait]
pub trait TypedProgressiveProcedure: Send + Sync {
    /// Input from the caller.
    type Input: battler_wamprat_message::WampApplicationMessage;

    /// Output to the caller.
    type Output: battler_wamprat_message::WampApplicationMessage;

    /// Error to the caller.
    type Error: Into<WampError>;

    /// Invokes the procedure and produces a result.
    async fn invoke<'rpc>(
        &self,
        input: Self::Input,
        progress: ProgressReporter<'rpc, Self::Output>,
    ) -> Result<Self::Output, Self::Error>;

    /// Options for the procedure.
    fn options() -> ProcedureOptions {
        ProcedureOptions::default()
    }
}

/// A strongly-typed, pattern-matched procedure that generates progressive output based on some
/// input.
#[async_trait]
pub trait TypedPatternMatchedProgressiveProcedure: Send + Sync {
    /// Pattern of the procedure.
    type Pattern: battler_wamprat_uri::WampUriMatcher;

    /// Input from the caller.
    type Input: battler_wamprat_message::WampApplicationMessage;

    /// Output to the caller.
    type Output: battler_wamprat_message::WampApplicationMessage;

    /// Error to the caller.
    type Error: Into<WampError>;

    /// Invokes the procedure and produces a result.
    async fn invoke<'rpc>(
        &self,
        input: Self::Input,
        procedure: Self::Pattern,
        progress: ProgressReporter<'rpc, Self::Output>,
    ) -> Result<Self::Output, Self::Error>;

    /// Options for the procedure.
    fn options() -> ProcedureOptions {
        ProcedureOptions::default()
    }
}
