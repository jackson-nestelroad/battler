use anyhow::Result;
use battler::{
    DataStoreByName,
    PlayerBattleData,
    Request,
};
use battler_client::state::BattleState;

/// A previous choice that resulted in a failure.
#[derive(Debug, Clone)]
pub struct ChoiceFailure {
    pub choice: String,
    pub error: String,
}

/// The context of a battle AI making a choice in a battle.
pub struct AiContext<'d> {
    pub data: &'d dyn DataStoreByName,
    pub state: BattleState,
    pub player_data: PlayerBattleData,
    pub previous_choice_failures: Vec<ChoiceFailure>,
}

/// An AI decision maker for a battle managed by battler.
pub trait BattlerAi {
    /// Makes a choice given the current context of the battle.
    fn make_choice(&mut self, context: AiContext, request: Request) -> Result<String>;
}
