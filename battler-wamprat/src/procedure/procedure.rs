use anyhow::Result;
use async_trait::async_trait;
use battler_wamp::peer::Invocation;

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

    /// Invokes the procedure and produces a result.
    async fn invoke(&self, input: Self::Input) -> Result<Self::Output>;
}
