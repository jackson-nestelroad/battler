use anyhow::Result;
use async_trait::async_trait;
use battler::Request;
use battler_choice::Choice;

use crate::{
    AiContext,
    BattlerAiStructured,
};

/// A battle AI where decisions are made randomly.
#[derive(Debug, Default)]
pub struct Random {}

#[async_trait]
impl BattlerAiStructured for Random {
    async fn make_choice<'a>(&mut self, _: &AiContext<'a>, _: &Request) -> Result<Vec<Choice>> {
        Ok(Vec::from_iter([Choice::RandomAll]))
    }
}
