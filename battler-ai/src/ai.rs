use ahash::HashSet;
use anyhow::Result;
use battler::{
    DataStoreByName,
    PlayerBattleData,
    Request,
};
use battler_client::state::BattleState;

use crate::choice::{
    Choice,
    ChoiceFailure,
};

/// The context of a battle AI making a choice in a battle.
pub struct AiContext<'d> {
    pub data: &'d dyn DataStoreByName,
    pub state: BattleState,
    pub player_data: PlayerBattleData,
    pub choice_failures: HashSet<ChoiceFailure>,
}

/// An AI decision maker for a battle managed by battler.
pub trait BattlerAi {
    /// Makes a choice given the current context of the battle.
    fn make_choice(
        &mut self,
        context: AiContext,
        request: Request,
    ) -> impl Future<Output = Result<Vec<Choice>>> + Send;
}
