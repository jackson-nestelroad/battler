use ahash::HashSet;
use anyhow::Result;
use async_trait::async_trait;
use battler::{
    DataStoreByName,
    PlayerBattleData,
    Request,
};
use battler_choice::Choice;
use battler_client::state::BattleState;
use itertools::Itertools;

use crate::choice::{
    ChoiceFailure,
    MakeChoiceFailure,
};

/// The context of a battle AI making a choice in a battle.
pub struct AiContext<'d> {
    pub data: &'d dyn DataStoreByName,
    pub state: BattleState,
    pub player_data: PlayerBattleData,
    pub choice_failures: HashSet<ChoiceFailure>,
    pub make_choice_failures: Vec<MakeChoiceFailure>,
}

/// An AI decision maker for a battle managed by battler.
#[async_trait]
pub trait BattlerAi {
    /// Makes a choice given the current context of the battle.
    async fn make_choice<'a>(
        &mut self,
        context: &AiContext<'a>,
        request: &Request,
    ) -> Result<String>;
}

/// An AI decision maker for a battle managed by battler, with structured output.
#[async_trait]
pub trait BattlerAiStructured {
    /// Makes a choice given the current context of the battle.
    async fn make_choice<'a>(
        &mut self,
        context: &AiContext<'a>,
        request: &Request,
    ) -> Result<Vec<Choice>>;
}

#[async_trait]
impl<T> BattlerAi for T
where
    T: BattlerAiStructured + Send,
{
    async fn make_choice<'a>(
        &mut self,
        context: &AiContext<'a>,
        request: &Request,
    ) -> Result<String> {
        Ok(self
            .make_choice(context, request)
            .await?
            .into_iter()
            .map(|choice| choice.to_string())
            .join(";"))
    }
}
