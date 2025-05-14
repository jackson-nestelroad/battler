use battler::DataStoreByName;
use battler_client::state::BattleState;

/// The context of a battle AI making a choice in a battle.
pub struct AiContext<'d> {
    pub data: &'d dyn DataStoreByName,
    pub state: BattleState,
}

/// An AI decision maker for a battle managed by battler.
pub trait BattlerAi {
    /// Makes a choice given the current context of the battle.
    fn make_choice(&mut self, context: AiContext);
}
