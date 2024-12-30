use anyhow::Result;
use async_trait::async_trait;
use battler_wamp::peer::Invocation;

/// A procedure that responds to any given invocation with some result.
#[async_trait]
pub trait Procedure: Send + Sync {
    async fn invoke(&self, invocation: Invocation) -> Result<()>;
}

/// A strongly-typed procedure that generates some output based on some input.
#[async_trait]
pub trait TypedProcedure: Send + Sync {
    type Input: battler_wamprat_message::WampApplicationMessage;
    type Output: battler_wamprat_message::WampApplicationMessage;
    async fn invoke(&self, input: Self::Input) -> Result<Self::Output>;
}
